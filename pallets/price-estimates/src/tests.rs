// This file is part of Substrate.

// Copyright (C) 2020-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// use crate::*;
// use crate::mock::*;
use crate::mock::{AccountId, AresId, BlockNumber, Balances, Call, Estimates, Extrinsic, System, helper_create_new_estimates_with_deviation, new_test_ext, Origin, run_to_block, Test, TestSymbolInfo, helper_create_new_estimates_with_range, TestPalletId, Balance, MaximumKeepLengthOfOldData};
use codec::Decode;
use frame_support::{
	assert_noop, assert_ok, parameter_types,
	traits::{ConstU32, ConstU64},
};
// use sp_core::{
// 	offchain::{testing, OffchainWorkerExt, TransactionPoolExt},
// 	sr25519::Signature,
// 	H256,
// };

use sp_core::{
	crypto::key_types::DUMMY,
	offchain::{testing::TestOffchainExt, OffchainDbExt, StorageKind, OffchainWorkerExt, testing, TransactionPoolExt},
};

use std::sync::Arc;
use frame_support::traits::Len;
use crate::*;
use frame_system::offchain::{SignedPayload, SigningTypes};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{testing::{Header, TestXt}, traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentityLookup, Verify}, RuntimeAppPublic, Permill, print};
use ares_oracle_provider_support::SymbolInfo;
use sp_runtime::traits::{AppVerify, ValidateUnsigned};
use sp_runtime::transaction_validity::TransactionSource;
use ares_oracle_provider_support::{ChainPrice, ConvertChainPrice};
use bound_vec_helper::BoundVecHelper;
use crate::{ActiveEstimates, Admins, BalanceOf, CompletedEstimates, Error, LockedEstimates, MinimumInitReward, MinimumTicketPrice, PreparedEstimates, UnresolvedEstimates};
use crate::types::{AccountParticipateEstimates, BoundedVecOfSymbol, BoundedVecOfChooseWinnersPayload, ChooseTrigerPayload, EstimatesState, EstimatesType, MultiplierOption};

// fn test_pub() -> sp_core::sr25519::Public {
// 	sp_core::sr25519::Public::from_raw([1u8; 32])
// }

#[test]
fn test_call_preference() {
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {

		// Get configuration informations form storage.
		let admins = Admins::<Test>::get();
		// let white_list = Whitelist::<Test>::get();
		let locked_estimates =LockedEstimates::<Test>::get();
		let min_ticket_price = MinimumTicketPrice::<Test>::get();
		let min_init_reward = MinimumInitReward::<Test>::get();

		// Variables configured in creation.
		assert_eq!(admins, vec![AccountId::from_raw([1; 32])]);
		// assert_eq!(white_list, vec![AccountId::from_raw([2; 32])]);
		assert_eq!(locked_estimates, 2);
		assert_eq!(min_ticket_price, 100);
		assert_eq!(min_init_reward, 100);

		// Update configuration.
		assert_ok!(Estimates::preference(
            Origin::root(),
            Some(vec![AccountId::from_raw([7; 32])]), // admins: Option<Vec<T::AccountId>>,
            // Some(vec![AccountId::from_raw([8; 32])]), // whitelist: Option<Vec<T::AccountId>>,
            Some(50), // locked_estimates: Option<T::BlockNumber>,
            Some(600), // minimum_ticket_price: Option<BalanceOf<T>>,
            Some(700), // minimum_init_reward: Option<BalanceOf<T>>,
        ));

		// Get configuration informations form storage.
		let admins = Admins::<Test>::get();
		// let white_list = Whitelist::<Test>::get();
		let locked_estimates =LockedEstimates::<Test>::get();
		let min_ticket_price = MinimumTicketPrice::<Test>::get();
		let min_init_reward = MinimumInitReward::<Test>::get();

		// Configuration variables have been updated.
		assert_eq!(admins, vec![AccountId::from_raw([7; 32])]);
		// assert_eq!(white_list, vec![AccountId::from_raw([8; 32])]);
		assert_eq!(locked_estimates, 50);
		assert_eq!(min_ticket_price, 600);
		assert_eq!(min_init_reward, 700);
	});
}

#[test]
fn test_call_new_estimates_with_DEVIATION_no_palyer() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let multiplier: Vec<MultiplierOption> = vec![
			MultiplierOption::Base(1),
			MultiplierOption::Base(3),
			MultiplierOption::Base(5),
		]; //     multiplier: Vec<MultiplierOption>,
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		//
		helper_create_new_estimates_with_deviation (
			5,
			deviation,
			init_reward,
			price,
		);

		// // Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			(BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone())
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			(BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone())
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			(BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone())
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			(BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone())
		));

		// Check that the white list account is valid? No!!!!!
		// assert!(!Estimates::can_send_signed(), "Not valid");
		// So need to set it up first.
		// assert_ok!(Estimates::preference(
		//     Origin::root(),
		//     None, // admins: Option<Vec<T::AccountId>>,
		//     // Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
		//     None, // locked_estimates: Option<T::BlockNumber>,
		//     None, // minimum_ticket_price: Option<BalanceOf<T>>,
		//     None, // minimum_init_reward: Option<BalanceOf<T>>,
		// ));
		// Check that the white list account is valid ? Yes, it's set.
		// assert!(Estimates::can_send_signed(), "Yes, it's set");

		assert!(CompletedEstimates::<Test>::contains_key(
			(BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone())
		));
		let completed = CompletedEstimates::<Test>::get(
			(BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone())
		);
		assert_eq!(completed.len(), 0);
		let estimate_config = ActiveEstimates::<Test>::get(
			(BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone())
		);
		assert!(estimate_config.is_some());
		let mut estimate_config = estimate_config.unwrap();

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		// assert_eq!(tx.signature.unwrap().0, 1);
		println!("tx.call = {:?}", tx.call);

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()) ,
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		let symbol = BoundedVecOfSymbol::create_on_vec(symbol.clone());

		// Estimate will to end without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			(symbol.clone(), estimates_type.clone())
		));
		assert!(CompletedEstimates::<Test>::contains_key(
			(symbol.clone(), estimates_type.clone())
		));
		let completed = CompletedEstimates::<Test>::get(
			(symbol.clone(), estimates_type.clone())
		);
		assert_eq!(completed.len(), 1);
		estimate_config.state = EstimatesState::Completed;
		estimate_config.total_reward = 0;
		estimate_config.symbol_completed_price = TestSymbolInfo::price(&symbol).unwrap().0;
		estimate_config.symbol_fraction = TestSymbolInfo::price(&symbol).unwrap().1;
		assert_eq!(
			completed[0],
			estimate_config
		);
	});
}

#[test]
fn test_call_new_estimates_with_DEVIATION_has_winner() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		//
		helper_create_new_estimates_with_deviation (
			5,
			deviation,
			init_reward,
			price,
		);

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());

		// // Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: Some(23164822300),
			range_index: None,
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: (2500/3*3)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            Some(231648223),
            Some(4),
            account_participate.range_index.clone(),
            account_participate.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			let transaction_validity = Estimates::validate_unsigned(TransactionSource::Local, &crate::Call::choose_winner {
				trigger_payload: body.clone(),
				signature: signature.clone()
			});

			println!("{:?}", transaction_validity);

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		//
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key
		));
		assert_eq!(CompletedEstimates::<Test>::get(
			&storage_key
		).len(), 1);
		let winners = Winners::<Test>::get(&storage_key, 0);
		// println!("{:?}", winners);
		assert!(winners.is_some());
		assert_eq!(winners.unwrap().len(), 1);
		let participants = Participants::<Test>::get(&storage_key, 0);
		// println!("{:?}", participants);
		assert_eq!(participants.len(), 1);

		// clean
		assert_ok!(Estimates::data_cleaning(Origin::none()));
		assert_noop!(Estimates::data_cleaning(Origin::none()), Error::<Test>::TooOften);

		// No change becouse the conditional is not met (block not reach)
		let winners = Winners::<Test>::get(&storage_key, 0);
		assert!(winners.is_some());
		assert_eq!(winners.unwrap().len(), 1);
		let participants = Participants::<Test>::get(&storage_key, 0);
		assert_eq!(participants.len(), 1);

		// Check winner free balance , the last value subtracted is the transfer fee.
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 + init_reward - 1);

		// Force the BlockNumber
		let complate_conf_vec = CompletedEstimates::<Test>::get(&storage_key);
		assert_eq!(complate_conf_vec.len(), 1);
		let complate_conf = complate_conf_vec[0].clone();
		System::set_block_number(complate_conf.distribute + MaximumKeepLengthOfOldData::get());
		assert_ok!(Estimates::data_cleaning(Origin::none()));
		let winners = Winners::<Test>::get(&storage_key, 0);
		assert!(winners.is_none());
		let participants = Participants::<Test>::get(&storage_key, 0);
		assert_eq!(participants.len(), 0);
		let complate_conf_vec = CompletedEstimates::<Test>::get(&storage_key);
		assert_eq!(complate_conf_vec.len(), 0);


	});
}

#[test]
fn test_call_new_estimates_with_DEVIATION_with_invalid_price_and_force_complete() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		run_to_block(50);

		//
		helper_create_new_estimates_with_deviation (
			80,
			deviation,
			init_reward,
			price,
		);

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());

		// // Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(85);
		assert_eq!(System::block_number(), 85) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check that the white list account is valid? No!!!!!
		// assert!(!Estimates::can_send_signed(), "Not valid");
		// So need to set it up first.
		assert_ok!(Estimates::preference(
            Origin::root(),
            Some(vec![AccountId::from_raw([6; 32])]),
            // Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T>>,
            None, // minimum_init_reward: Option<BalanceOf<T>>,
        ));
		// Check that the white list account is valid ? Yes, it's set.
		// assert!(Estimates::can_send_signed(), "Yes, it's set");

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 90,
			estimates: Some(23164822300),
			range_index: None,
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: (2500/3*3)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            Some(231648223),
            Some(4),
            account_participate.range_index.clone(),
            account_participate.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(91);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// Check
		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// Not input CompletedEstimates
		assert_eq!(0 , CompletedEstimates::<Test>::get(
			&storage_key
		).len());

		// Check UnresolvedEstimates
		assert!(UnresolvedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Use admin go to force completed.
		assert_noop!(
		   Estimates::force_complete(
                Origin::signed(AccountId::from_raw([3; 32])),
                symbol.clone(),
				estimates_type.clone(),
                231648223,
                4,
            ),
		   Error::<Test>::NotMember
		);

		assert_ok!(
		   Estimates::force_complete(
                Origin::signed(AccountId::from_raw([6; 32])),
                symbol.clone(),
				estimates_type.clone(),
                231648223,
                4,
            )
		);

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert_eq!(1, CompletedEstimates::<Test>::get(
			&storage_key
		).len());

		// Check UnresolvedEstimates
		assert!(!UnresolvedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check winner free balance , the last value subtracted is the transfer fee.
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 + init_reward - 1);

	});
}

#[test]
fn test_fix_immortality_estimates_bug_08250957() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {
		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		run_to_block(50);
		//
		let admin_acc =helper_create_new_estimates_with_deviation (
			80,
			deviation,
			init_reward,
			price,
		);

		assert_eq!(Balances::free_balance(&admin_acc), 1000000000100 - init_reward );

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());

		// // Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(85);
		assert_eq!(System::block_number(), 85) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Add a new estimates before the old one is over
		let admin_acc = helper_create_new_estimates_with_deviation (
			86,
			deviation,
			init_reward,
			price,
		);

		assert_eq!(Balances::free_balance(&admin_acc), 1000000000100 - init_reward * 2);

		// New one's ending is 90, id is 1
		let new_prepared = PreparedEstimates::<Test>::get(&storage_key);
		assert!(new_prepared.is_some());
		let new_prepared = new_prepared.unwrap();
		assert_eq!(
			SymbolEstimatesId::<Test>::get(&storage_key),
			Some(2)
		);
		assert_eq!(new_prepared.id, 1);
		assert_eq!(new_prepared.start, 91);
		assert_eq!(new_prepared.end, 96);

		// Current block number is 86
		assert_eq!(System::block_number(), 86) ;

		// Check that the white list account is valid? No!!!!!
		// assert!(!Estimates::can_send_signed(), "Not valid");
		// So need to set it up first.
		assert_ok!(Estimates::preference(
            Origin::root(),
            Some(vec![AccountId::from_raw([6; 32])]),
            // Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T>>,
            None, // minimum_init_reward: Option<BalanceOf<T>>,
        ));
		// Check that the white list account is valid ? Yes, it's set.
		// assert!(Estimates::can_send_signed(), "Yes, it's set");

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 90,
			estimates: Some(23164822300),
			range_index: None,
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: (2500/3*3)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            Some(231648223),
            Some(4),
            account_participate.range_index.clone(),
            account_participate.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(91);

		println!("DEBUG-ActiveEstimates A :{:?}", ActiveEstimates::<Test>::get(
			&storage_key
		));

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()) ,
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		run_to_block(97);

		// Check
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// Not input CompletedEstimates
		assert_eq!(0 , CompletedEstimates::<Test>::get(
			&storage_key
		).len());

		// Check UnresolvedEstimates
		assert_eq!(1, UnresolvedEstimates::<Test>::get(
			&storage_key
		).len());

		// New Estimates can not start yet because the old one not completed.
		assert!(PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check user balance.
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);

		// Go to 99 block number participate will don't change
		assert_ok!(
		   Estimates::force_complete(
                Origin::signed(AccountId::from_raw([6; 32])),
                symbol.clone(),
				estimates_type.clone(),
                231648223,
                4,
            )
		);

		// Check user balance.
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 + init_reward - 1);

		run_to_block(98);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		run_to_block(99);
		println!("B =============================");

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// Prepared estimate will go to unresolved list
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert_eq!(2, CompletedEstimates::<Test>::get(
			&storage_key
		).len());

		// Check UnresolvedEstimates
		assert_eq!(0, UnresolvedEstimates::<Test>::get(
			&storage_key
		).len());

		println!("UnresolvedEstimates {:?}", CompletedEstimates::<Test>::get(
			&storage_key
		))

		// Check winner free balance , the last value subtracted is the transfer fee.
		// assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 + init_reward - 1);

	});
}

#[test]
fn test_call_new_estimates_with_DEVIATION_has_2_winner() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		//
		helper_create_new_estimates_with_deviation (
			5,
			deviation,
			init_reward,
			price,
		);

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());
		// // Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate1 = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: Some(23164822300),
			range_index: None,
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: ((1000+500*8)/8*3)
		};

		// Make AccountParticipateEstimates
		let account_participate2 = AccountParticipateEstimates{
			account: AccountId::from_raw([4; 32]),
			end: 15,
			estimates: Some(23164822300),
			range_index: None,
			bsc_address: None,
			multiplier: MultiplierOption::Base(5),
			reward: ((1000+500*8)/8*5)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100);
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate1.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            Some(231648223),
            Some(4),
            account_participate1.range_index.clone(),
            account_participate1.multiplier.clone(),
            None, // bsc address
        ));
		// println!("-------------");
		// Participants::<Test>::iter().any(|x|{
		//     println!("x = {:?}", x.2);
		//     false
		// });
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate2.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            Some(231648223),
            Some(4),
            account_participate2.range_index.clone(),
            account_participate2.multiplier.clone(),
            None, // bsc address
        ));



		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100 - price * 3);
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100 - price * 5);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		// assert_eq!(tx.signature.unwrap().0, 1);
		// println!("tx.call = {:?}", tx.call);

		// Create winner
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		winners.try_push(account_participate1.clone());
		winners.try_push(account_participate2.clone());

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check winner free balance , the last value subtracted is the transfer fee.
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100 - price * 3 + ((init_reward+ price * 8)/8*3));
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100 - price * 5 + ((init_reward+ price * 8)/8*5));
	});
}

#[test]
fn test_call_new_estimates_with_DEVIATION_no_winner() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {
		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		// ######################### SETP 1 ##########################
		// Create estimates
		helper_create_new_estimates_with_deviation (
			5,
			deviation,
			init_reward,
			price,
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.
		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;

		// ######################### SETP 3 ##########################
		//

		// Make AccountParticipateEstimates
		let account_participate = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: Some(231648223 + deviation * 231648223 + 10), // More deviation
			range_index: None,
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: (2500/3*3)
		};

		// Check no free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100);

		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate.estimates.clone(),
            Some(4),
            account_participate.range_index.clone(),
            account_participate.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		// assert_eq!(tx.signature.unwrap().0, 1);
		// println!("tx.call = {:?}", tx.call);

		// Create winner, No one is winner.
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		// winners.try_push(account_participate.clone());

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type);

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check winner free balance , the last value subtracted is the transfer fee.
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);
	});
}

// Test range
#[test]
fn test_call_new_estimates_with_RANGE_no_palyer() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::RANGE; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		// let multiplier: Vec<MultiplierOption> = vec![
		//     MultiplierOption::Base(1),
		//     MultiplierOption::Base(3),
		//     MultiplierOption::Base(5),
		// ]; //     multiplier: Vec<MultiplierOption>,
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		// ######################### SETP 2 ##########################
		// Create new estimates.
		helper_create_new_estimates_with_range (
			5,
			vec![21481_3055u64, 23481_3055u64, 27481_3055u64, 29481_3055u64],
			4,
			init_reward,
			price,
		);

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), EstimatesType::RANGE);
		// // Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();


		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// The estimate will force to end without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

	});
}

#[test]
fn test_call_new_estimates_with_RANGE_has_winner() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		// let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		// Create new estimates. (23164_8223)
		helper_create_new_estimates_with_range (
			5,
			// 23164_8223 <= 21481_3055u64 NO 0
			// 21481_3055u64 < 23164_8223 && 23164_8223 <= 27481_3055u64 YES 1
			// 27481_3055u64 <= 23164_8223 && 23164_8223 <= 29481_3055u64 NO 2
			// 23164_8223 > 29481_3055u64 4
			vec![21481_3055u64, 23481_3055u64, 27481_3055u64, 29481_3055u64],
			4,
			init_reward,
			price,
		);

		let estimates_type = EstimatesType::RANGE;
		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());

		// Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: None,
			range_index: Some(1),
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: (2500/3*3)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate.estimates.clone(),
            None,
            account_participate.range_index.clone(),
            account_participate.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		// assert_eq!(tx.signature.unwrap().0, 1);
		// println!("tx.call = {:?}", tx.call);

		// Create winner
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		winners.try_push(account_participate.clone());

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check winner free balance , the last value subtracted is the transfer fee.
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 + init_reward - 1);

	});
}

#[test]
fn test_call_new_estimates_with_RANGE_has_winner_on_left() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::RANGE; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		// Create new estimates. (23164_8223)
		helper_create_new_estimates_with_range (
			5,
			// 23164_8223 <= 23481_3055u64 YES 0
			vec![23481_3055u64, 27481_3055u64, 29481_3055u64, 31481_3055u64],
			4,
			init_reward,
			price,
		);

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());
		// Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: None,
			range_index: Some(0),
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: (2500/3*3)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate.estimates.clone(),
            None,
            account_participate.range_index.clone(),
            account_participate.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		// assert_eq!(tx.signature.unwrap().0, 1);
		// println!("tx.call = {:?}", tx.call);

		// Create winner
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		winners.try_push(account_participate.clone());

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check winner free balance , the last value subtracted is the transfer fee.
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 + init_reward - 1);

	});
}

#[test]
fn test_call_new_estimates_with_RANGE_has_winner_on_right() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::RANGE; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		// Create new estimates. (23164_8223)
		helper_create_new_estimates_with_range (
			5,
			// 23164_8223 > 23064_8223u64 YES. so select index 4
			vec![20064_8223u64, 21064_8223u64, 22064_8223u64, 23064_8223u64],
			4,
			init_reward,
			price,
		);

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());

		// Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: None,
			range_index: Some(4),
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: (2500/3*3)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate.estimates.clone(),
            None,
            account_participate.range_index.clone(),
            account_participate.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		// assert_eq!(tx.signature.unwrap().0, 1);
		// println!("tx.call = {:?}", tx.call);

		// Create winner
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		winners.try_push(account_participate.clone());

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check winner free balance , the last value subtracted is the transfer fee.
		assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 + init_reward - 1);

	});
}

#[test]
fn test_call_new_estimates_with_RANGE_has_2_palyer_2_winner_on_right() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::RANGE; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		// Create new estimates. (23164_8223)
		helper_create_new_estimates_with_range (
			5,
			// 23164_8223 > 23064_8223u64 YES. so select index 4
			vec![20064_8223u64, 21064_8223u64, 22064_8223u64, 23064_8223u64],
			4,
			init_reward,
			price,
		);

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());

		// Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate1 = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: None,
			range_index: Some(4),
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: ((2500+500*5)/8*3)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100);

		// Make AccountParticipateEstimates
		let account_participate2 = AccountParticipateEstimates{
			account: AccountId::from_raw([4; 32]),
			end: 15,
			estimates: None,
			range_index: Some(4),
			bsc_address: None,
			multiplier: MultiplierOption::Base(5),
			reward: ((2500+500*5)/8*5)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate1.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate1.estimates.clone(),
            None,
            account_participate1.range_index.clone(),
            account_participate1.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100 - price * 3);

		// Make player2.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate2.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate2.estimates.clone(),
            None,
            account_participate2.range_index.clone(),
            account_participate2.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100 - price * 5);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		// assert_eq!(tx.signature.unwrap().0, 1);
		// println!("tx.call = {:?}", tx.call);

		// Create winner
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		winners.try_push(account_participate1.clone());
		winners.try_push(account_participate2.clone());

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check winner free balance , the last value subtracted is the transfer fee.
		// (price * 5) is player2 bet
		assert_eq!(
			Balances::free_balance(&account_participate1.account.clone()),
			3000000000100 - price * 3u64 + (init_reward + (price * 8u64)) / 8u64 * 3u64
		);

	});
}

#[test]
fn test_call_new_estimates_with_RANGE_has_2_palyer_1_winner_on_right() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::RANGE; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		// Create new estimates. (23164_8223)
		helper_create_new_estimates_with_range (
			5,
			// 23164_8223 > 23064_8223u64 YES. so select index 4
			vec![20064_8223u64, 21064_8223u64, 22064_8223u64, 23064_8223u64],
			4,
			init_reward,
			price,
		);

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());

		// Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check that the white list account is valid? No!!!!!
		// assert!(!Estimates::can_send_signed(), "Not valid");
		// So need to set it up first.
		// assert_ok!(Estimates::preference(
		//     Origin::root(),
		//     None, // admins: Option<Vec<T::AccountId>>,
		//     Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
		//     None, // locked_estimates: Option<T::BlockNumber>,
		//     None, // minimum_ticket_price: Option<BalanceOf<T>>,
		//     None, // minimum_init_reward: Option<BalanceOf<T>>,
		// ));
		// Check that the white list account is valid ? Yes, it's set.
		// assert!(Estimates::can_send_signed(), "Yes, it's set");

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate1 = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: None,
			range_index: Some(4),
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: ((2500+500*5)/3*3)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100);

		// Make AccountParticipateEstimates
		let account_participate2 = AccountParticipateEstimates{
			account: AccountId::from_raw([4; 32]),
			end: 15,
			estimates: None,
			range_index: Some(3),
			bsc_address: None,
			multiplier: MultiplierOption::Base(5),
			reward: 0
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate1.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate1.estimates.clone(),
            None,
            account_participate1.range_index.clone(),
            account_participate1.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100 - price * 3);

		// Make player2.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate2.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate2.estimates.clone(),
            None,
            account_participate2.range_index.clone(),
            account_participate2.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100 - price * 5);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		// assert_eq!(tx.signature.unwrap().0, 1);
		// println!("tx.call = {:?}", tx.call);

		// Create winner
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		winners.try_push(account_participate1.clone());

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key
		));

		// Check winner free balance , the last value subtracted is the transfer fee.
		// (price * 5) is player2 bet
		assert_eq!(
			Balances::free_balance(&account_participate1.account.clone()),
			3000000000100 - price * 3u64 + (init_reward + (price * 8u64)) / 3u64 * 3u64
		);

	});
}

#[test]
fn test_config_type_max_end_delay() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type =  EstimatesType::RANGE; //     estimates_type: EstimatesType,
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		// Create new estimates. (23164_8223)
		helper_create_new_estimates_with_range (
			5,
			// 23164_8223 > 23064_8223u64 YES. so select index 4
			vec![20064_8223u64, 21064_8223u64, 22064_8223u64, 23064_8223u64],
			4,
			init_reward,
			price,
		);

		let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone());

		// Check estimate.
		let estimate = PreparedEstimates::<Test>::get(
			&storage_key
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(10);
		assert_eq!(System::block_number(), 10) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		let mut active_estimate = estimate.clone().unwrap();
		active_estimate.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
			&storage_key
		));
		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key
		));

		// ######################### SETP 2 ##########################
		// Make AccountParticipateEstimates
		let account_participate1 = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: None,
			range_index: Some(4),
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: ((2500+500*5)/3*3)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100);

		// Make AccountParticipateEstimates
		let account_participate2 = AccountParticipateEstimates{
			account: AccountId::from_raw([4; 32]),
			end: 15,
			estimates: None,
			range_index: Some(3),
			bsc_address: None,
			multiplier: MultiplierOption::Base(5),
			reward: 0
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate1.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate1.estimates.clone(),
            None,
            account_participate1.range_index.clone(),
            account_participate1.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100 - price * 3);

		// Make player2.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate2.account.clone()),
            symbol.clone(),
			estimates_type.clone(),
            account_participate2.estimates.clone(),
            None,
            account_participate2.range_index.clone(),
            account_participate2.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100 - price * 5);

		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(16);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		// assert_eq!(tx.signature.unwrap().0, 1);
		// println!("tx.call = {:?}", tx.call);

		// Create winner
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		winners.try_push(account_participate1.clone());

		// Adjust the blocknumber than the MaxEndDelay
		run_to_block(36);

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key
		));

		assert!(UnresolvedEstimates::<Test>::contains_key(
			&storage_key
		));

		let estimates = UnresolvedEstimates::<Test>::get(
			&storage_key
		);
		assert_eq!(estimates.unwrap().state, EstimatesState::Unresolved);
	});
}

#[test]
fn test_supports_creating_multiple_tasks_at_the_same_time() {
	let mut t = new_test_ext();
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AresId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
		.get(0)
		.unwrap()
		.clone();

	// let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
	//     .get(1)
	//     .unwrap()
	//     .clone();

	t.register_extension(OffchainWorkerExt::new(offchain.clone()));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.register_extension(OffchainDbExt::new(offchain.clone()));

	t.execute_with(|| {

		//
		let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
		let start: BlockNumber = 10; //     start: T::BlockNumber,
		let end: BlockNumber = 15; //     end: T::BlockNumber,
		let distribute: BlockNumber = 20; //     distribute: T::BlockNumber,
		let estimates_type_deviation =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
		let estimates_type_range = EstimatesType::RANGE;
		let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
		let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
		let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
		let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

		//
		helper_create_new_estimates_with_deviation (
			5,
			deviation,
			init_reward,
			price,
		);
		// helper_create_new_estimates_with_range (
		// 	6,
		// 	vec![21481_3055u64, 23481_3055u64, 27481_3055u64, 29481_3055u64],
		// 	4,
		// 	init_reward,
		// 	price,
		// );
		// Create new estimates. (23164_8223)
		helper_create_new_estimates_with_range (
			6,
			// 23164_8223 > 23064_8223u64 YES. so select index 4
			vec![20064_8223u64, 21064_8223u64, 22064_8223u64, 23064_8223u64],
			4,
			init_reward,
			price,
		);

		let storage_key_deviation = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type_deviation.clone());
		let storage_key_range = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type_range.clone());

		// // Check estimate.
		let estimate_deviation = PreparedEstimates::<Test>::get(
			&storage_key_deviation
		);
		let estimate_range = PreparedEstimates::<Test>::get(
			&storage_key_range
		);

		// ######################### SETP 2 ##########################
		// The arrival start block ActiveEstimates will fill new value.

		run_to_block(11);
		assert_eq!(System::block_number(), 11) ;
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key_deviation
		));
		assert!(ActiveEstimates::<Test>::contains_key(
			&storage_key_range
		));

		let mut active_estimate_deviation = estimate_deviation.clone().unwrap();
		active_estimate_deviation.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate_deviation), ActiveEstimates::<Test>::get(
			&storage_key_deviation
		));

		let mut active_estimate_range = estimate_range.clone().unwrap();
		active_estimate_range.state = EstimatesState::Active;
		assert_eq!(Some(active_estimate_range), ActiveEstimates::<Test>::get(
			&storage_key_range
		));

		// Then PreparedEstimates is empty.
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key_deviation
		));
		assert!(!PreparedEstimates::<Test>::contains_key(
			&storage_key_range
		));

		// ######################### SETP 2.1 ##########################
		// Make AccountParticipateEstimates
		let account_participate1 = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: Some(23164822300),
			range_index: None,
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: ((1000+500*8)/8*3)
		};

		// Make AccountParticipateEstimates
		let account_participate2 = AccountParticipateEstimates{
			account: AccountId::from_raw([4; 32]),
			end: 15,
			estimates: Some(23164822300),
			range_index: None,
			bsc_address: None,
			multiplier: MultiplierOption::Base(5),
			reward: ((1000+500*8)/8*5)
		};

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100);
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100);

		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate1.account.clone()),
            symbol.clone(),
			estimates_type_deviation.clone(),
            Some(231648223),
            Some(4),
            account_participate1.range_index.clone(),
            account_participate1.multiplier.clone(),
            None, // bsc address
        ));

		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate2.account.clone()),
            symbol.clone(),
			estimates_type_deviation.clone(),
            Some(231648223),
            Some(4),
            account_participate2.range_index.clone(),
            account_participate2.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100 - price * 3);
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100 - price * 5);

		//----------------------------------------------------------------------

		// ######################### SETP 2.2 ##########################
		// Make AccountParticipateEstimates
		let account_participate3 = AccountParticipateEstimates{
			account: AccountId::from_raw([3; 32]),
			end: 15,
			estimates: None,
			range_index: Some(4),
			bsc_address: None,
			multiplier: MultiplierOption::Base(3),
			reward: ((2500+500*5)/8*3)
		};

		// Make AccountParticipateEstimates
		let account_participate4 = AccountParticipateEstimates{
			account: AccountId::from_raw([4; 32]),
			end: 15,
			estimates: None,
			range_index: Some(4),
			bsc_address: None,
			multiplier: MultiplierOption::Base(5),
			reward: ((2500+500*5)/8*5)
		};


		// Make winner.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate3.account.clone()),
            symbol.clone(),
			estimates_type_range.clone(),
            account_participate3.estimates.clone(),
            None,
            account_participate3.range_index.clone(),
            account_participate3.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100 - price * 6);

		// Make player2.
		// Call participate_estimates
		assert_ok!(Estimates::participate_estimates(
            Origin::signed(account_participate4.account.clone()),
            symbol.clone(),
			estimates_type_range.clone(),
            account_participate4.estimates.clone(),
            None,
            account_participate4.range_index.clone(),
            account_participate4.multiplier.clone(),
            None, // bsc address
        ));

		// Check winner free balance
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100 - price * 10);

		println!("account_participate1.account.balance={:?}", Balances::free_balance(&account_participate1.account.clone()));
		println!("account_participate2.account.balance={:?}", Balances::free_balance(&account_participate2.account.clone()));


		// ######################### SETP 3 ##########################
		// Go to the end block and call cal_winners
		run_to_block(17);

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		// Create winner
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		winners.try_push(account_participate3.clone());
		winners.try_push(account_participate4.clone());

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type_range.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key_range
		));

		// ---------- fro deviation
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		// Create winner
		let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
		winners.try_push(account_participate1.clone());
		winners.try_push(account_participate2.clone());

		if let Call::Estimates(
			crate::Call::choose_winner {
				trigger_payload: body,
				signature: signature
			}) = tx.call {

			assert_eq!(body, ChooseTrigerPayload {
				public: public_key_1.clone(),
				symbol: (BoundedVecOfSymbol::create_on_vec(symbol.clone()),estimates_type_deviation.clone()),
			});

			let signature_valid =
				<ChooseTrigerPayload<
					<Test as SigningTypes>::Public,
				> as SignedPayload<Test>>::verify::<ares_oracle::ares_crypto::AresCrypto<AresId>>(&body, signature.clone());
			assert!(signature_valid);

			assert_ok!(Estimates::choose_winner(
                Origin::none(),
                body,
                signature,
            ));
		}

		// Check winner free balance , the last value subtracted is the transfer fee.
		// assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100 - price * 3 + ((init_reward+ price * 8)/8*3));
		// assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100 - price * 5 + ((init_reward+ price * 8)/8*5));

		// You can't end the event without WINNER.
		assert!(!ActiveEstimates::<Test>::contains_key(
			&storage_key_deviation
		));

		// ################### Next, wait for the reward.
		// ###############################################
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key_range
		));
		assert!(CompletedEstimates::<Test>::contains_key(
			&storage_key_deviation
		));

		// all amount
		let each_amount_range = (init_reward+price*8)/8;
		let each_amount_deviation = (init_reward+price*8)/8;

		// Check winner free balance , the last value subtracted is the transfer fee.
		assert_eq!(Balances::free_balance(&account_participate1.account.clone()), 3000000000100 - price * 6 + each_amount_range * 3 + each_amount_deviation * 3 );
		assert_eq!(Balances::free_balance(&account_participate2.account.clone()), 4000000000100 - price * 10 + each_amount_range * 5 + each_amount_deviation * 5);
	});
}

#[test]
fn test_subaccount() {

	let acc: AccountId = TestPalletId.try_into_account().unwrap();
	println!("ACC: {:?}", acc );

	let uni_usdt: AccountId = TestPalletId.try_into_sub_account("uni-usdt".as_bytes().to_vec()).unwrap();
	// 6d6f 64 6c70792f61 7265737424 20756e692d75736474 00000000000000000000
	// 6d6f646c70792f61726573742420756e692d7573647400000000000000000000
	println!("uni_usdt 1 = {:?}, {:?}", uni_usdt, "uni-usdt".as_bytes().to_vec());

	let uni_usdt: AccountId = TestPalletId.try_into_sub_account(b"uni-usdt").unwrap();
	// 6d6f 64 6c70792f61 7265737424 20756e692d75736474 00000000000000000000
	// 6d6f646c70792f61726573742420756e692d7573647400000000000000000000
	println!("uni_usdt 2 = {:?}, {:?}", uni_usdt, b"uni-usdt");


	println!("---------------------------");

	// let btc_usdc: AccountId = TestPalletId.into_sub_account("btc-usdt".bytes().map(|b|{
	//     println!("b={:?}", b);
	//     b
	// }).collect::<Vec<u8>>());

	println!("{:?}", b"btc-usdt");
	let b_vec = b"btc-usdt".map(|x|{
		println!("-{:?}", x);
		x
	}).to_vec();
	println!("{:?}", b_vec);
	println!("{:?}", "btc-usdt".as_bytes().to_vec());

	// let btc_usdc: AccountId = TestPalletId.into_sub_account(b"btc-usdt");
	let btc_usdc: AccountId = TestPalletId.try_into_sub_account("btc-usdt".as_bytes()).unwrap();
	// 6d6f646c70792f6172657374206274632d757364740000000000000000000000 ERROR
	// 6d6f646c70792f61726573746274632d75736474000000000000000000000000 RIGTH

	println!("ENCODE A={:?}", "btc-usdt".encode());
	println!("ENCODE A={:?}", "btc-usdt".as_bytes().encode());

	// ---- success
	let left: AccountId = TestPalletId.try_into_sub_account(b"btc-usdt").unwrap();

	// // -------- failed
	// let right: AccountId = TestPalletId.into_sub_account("btc-usdt");
	// // -------- failed
	// let right: AccountId = TestPalletId.into_sub_account("btc-usdt".as_bytes());
	// // -------- failed
	// let right: AccountId = TestPalletId.into_sub_account("btc-usdt".as_bytes().to_vec());

	// --- ok
	let mut u8list: [u8; 8] = [0; 8]; /// .. "btc-usdt".as_bytes();
	// u8list.copy_from_slice("btc-usdt".as_bytes());
	u8list.copy_from_slice("btc-usdt".as_bytes().to_vec().as_slice());
	let right: AccountId = TestPalletId.try_into_sub_account(u8list).unwrap();
	println!("btc-usdt hex = {:?}", right);
	assert_eq!(
		left,
		right
	);
	// ---------

	// JS:      0x6d6f646c70792f6172657374616176652d757364740000000000000000000000
	// RUST:      6d6f646c70792f6172657374616176652d757364740000000000000000000000
	let left: AccountId = TestPalletId.try_into_sub_account(b"aave-usdt").unwrap();
	let mut u8list: [u8; 20] = [0; 20]; /// .. "btc-usdt".as_bytes();
	// u8list.copy_from_slice("btc-usdt".as_bytes());
	u8list[.."aave-usdt".len()].copy_from_slice("aave-usdt".as_bytes().to_vec().as_slice());
	let right: AccountId = TestPalletId.try_into_sub_account(u8list).unwrap();
	println!("aave-usdt hex = {:?}", right);
	assert_eq!(
		left,
		right
	);

	println!("btc-usdt = {:?}", btc_usdc);

	let aave_usdt: AccountId = TestPalletId.try_into_sub_account("aave-usdt".as_bytes().to_vec()).unwrap();
	println!("ERROR : aave_usdt = {:?}", aave_usdt);

	let aave_usdc: AccountId = TestPalletId.try_into_sub_account("aave-usdc".as_bytes().to_vec()).unwrap();
	println!("ERROR : aave_usdc = {:?}", aave_usdc);

	let aavel_usdc: AccountId = TestPalletId.try_into_sub_account("aavel-usdc".as_bytes().to_vec()).unwrap();
	println!("ERROR : aavel_usdc = {:?}", aavel_usdc);

}

#[test]
fn test_subaccount2() {

	let acc: AccountId = TestPalletId.try_into_account().unwrap();
	println!("ACC: {:?}", acc );

	let symbol="a";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="ab";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="abc";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="abcd";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="abcde";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="abcdef";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="abcdefg";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="abcdefgh";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="abcdefghi";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="abcdefghij";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);

	let symbol="abcdefghijk";
	let show: AccountId = TestPalletId.try_into_sub_account(symbol.as_bytes().to_vec()).unwrap();
	println!("{:?} = {:?}", symbol, show);
}

#[test]
fn test_chain_prcie() {
	// Test create
	let one = ChainPrice::new((10123, 3u32)); // 10.123
	let two = ChainPrice::new((101230, 4u32)); // 10.1230

	assert_eq!(
		<ChainPrice as ConvertChainPrice<Balance, u32>>::try_to_price(one.clone(), 4),
		<ChainPrice as ConvertChainPrice<Balance, u32>>::try_to_price(two.clone(), 4)
	);

	assert_eq!(
		<ChainPrice as ConvertChainPrice<Balance, u32>>::try_to_price(one.clone(), 6),
		<ChainPrice as ConvertChainPrice<Balance, u32>>::try_to_price(two.clone(), 6)
	);

	assert_eq!(
		<ChainPrice as ConvertChainPrice<Balance, u32>>::try_to_price(one.clone(), 3),
		<ChainPrice as ConvertChainPrice<Balance, u32>>::try_to_price(two.clone(), 3)
	);
}

#[test]
fn test_encode_to_slice() {
	let mut output_1 = [0; 4 * 2];
	hex::encode_to_slice(b"kiwi", &mut output_1).unwrap();
	assert_eq!(&output_1, b"6b697769");
}