// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
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

use codec::Encode;
use super::Event as AresOcwEvent;
use crate as ares_oracle;
use crate::mock::*;
use crate::{AbnormalPriceDataVec, AresPrice, AresPriceDataVecOf, AuthorTraceData, BlockAuthorTrace, CALCULATION_KIND_AVERAGE, CALCULATION_KIND_MEDIAN, Error, PreCheckTaskList, PreCheckTaskListVec, PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE, PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE, PurchasedOrderPool, PurchasedPriceDataVec, StorageVersion};
use frame_support::{
	assert_ok, assert_noop,
};
use sp_core::{
	offchain::{
		testing::{self},
		OffchainWorkerExt, TransactionPoolExt,
	},
	sr25519::Signature,
	H256,
};
use sp_keystore::{
	testing::KeyStore,
	{KeystoreExt, SyncCryptoStore},
};
use ares_oracle_provider_support::*;
use oracle_finance::traits::*;
use sp_runtime::RuntimeAppPublic;
use crate::{Authorities, AuthorityAresVec, AuthorityId as AuraId, Config, LocalXRay, PurchasedAvgPrice, PurchasedAvgTrace, PurchasedPricePool, PurchasedRequestPool, RequestBaseVecU8};
use frame_support::traits::{ConstU32, Everything, ExtrinsicCall, FindAuthor, OnInitialize, OnRuntimeUpgrade};
use frame_system::offchain::{SendUnsignedTransaction, SignedPayload, Signer, SigningTypes};
use lite_json::NumberValue;
use sp_runtime::offchain::OffchainDbExt;
use sp_std::sync::Arc;
use sp_core::sr25519::Public;
use crate::mock::{new_test_ext};
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{IdentifyAccount, Verify};
use sp_application_crypto::Pair;
use sp_core::crypto::Ss58Codec;
use sp_runtime::{BoundedVec, Percent};
use sp_std::convert::{TryFrom, TryInto};
use ares_oracle_provider_support::{JsonNumberValue, PriceKey, PriceToken, RawSourceKeys, RequestKeys};
use bound_vec_helper::BoundVecHelper;
use crate::traits::ValidatorCount;
use codec::Decode;
use frame_support::instances::Instance1;
use crate::migrations::{OldPurchasedAvgPriceV1, OldPurchasedAvgTraceV1, OldPurchasedOrderPoolV1, OldPurchasedPricePoolV1, OldPurchasedRequestPoolV1, UpdateToV2};
use crate::test_tools::{get_are_json_of_bulk, get_are_json_of_bulk_of_xxxusdt_is_0, get_bug_json_of_20220721, to_purchase_id, to_test_bounded_vec, to_test_vec};
use crate::types::{AresPriceData, AvgPriceData, PricePayload, PricePayloadSubJumpBlockList, PricePayloadSubPrice, PricePayloadSubPriceList, PurchasedAvgPriceData, PurchasedDefaultData, PurchasedPricePayload, PurchasedRequestData, PurchasedSourceRawKeys};


mod test_IAresOraclePreCheck;
mod test_RuntimeUpgrade;

// #[test]
// fn test_finance_order() {
// 	new_test_ext().execute_with(|| {
// 		// let a = FinanceOrder::<OracleFinance, Instance1>::default();
// 		let a: <Test as Config>::OrderId2 = <Test as Config>::OrderId2::default() ;
// 		let b: <Test as Config>::OrderId2 = to_test_vec("Purchased_ID").try_into().unwrap();
//
// 		println!("a === {:?}", &a);
// 		println!("b === {:?}", &b);
// 	});
// }

#[test]
fn test_check_and_clear_expired_purchased_average_price_storage() {
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		System::set_block_number(1);

		let avg_price = PurchasedAvgPriceData {
			create_bn: 1,
			reached_type: 1,
			price_data: (10000, 4),
		};

		PurchasedAvgPrice::<Test>::insert(to_purchase_id("p_id"), PriceKey::create_on_vec(to_test_vec("btc_price")), avg_price);
		PurchasedAvgTrace::<Test>::insert(to_purchase_id("p_id"), 1);

		assert_eq!(
			AresOcw::check_and_clear_expired_purchased_average_price_storage(100),
			false
		);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 1);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 1);

		assert_eq!(
			AresOcw::check_and_clear_expired_purchased_average_price_storage(14402),
			true
		);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);
	});
}


#[test]
fn test_check_and_clean_hostkey_list() {
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		System::set_block_number(5);
		assert_eq!(AresOcw::check_and_clean_hostkey_list(5), 1);
		System::set_block_number(10);
		assert_eq!(AresOcw::check_and_clean_hostkey_list(5), 1);
		System::set_block_number(15);
		assert_eq!(AresOcw::check_and_clean_hostkey_list(5), 1);
		//
		LocalXRay::<Test>::insert(555, (15, RequestBaseVecU8::create_on_vec("house1".as_bytes().to_vec()), AuthorityAresVec::<AuraId>::default(), true));
		LocalXRay::<Test>::insert(666, (16, RequestBaseVecU8::create_on_vec("house2".as_bytes().to_vec()), AuthorityAresVec::<AuraId>::default(), true));
		LocalXRay::<Test>::insert(777, (17, RequestBaseVecU8::create_on_vec("house3".as_bytes().to_vec()), AuthorityAresVec::<AuraId>::default(), true));
		LocalXRay::<Test>::insert(888, (18, RequestBaseVecU8::create_on_vec("house4".as_bytes().to_vec()), AuthorityAresVec::<AuraId>::default(), true));
		assert_eq!(AresOcw::check_and_clean_hostkey_list(5), 1);
		assert_eq!(LocalXRay::<Test>::iter().count(), 4);

		System::set_block_number(20);
		assert_eq!(AresOcw::check_and_clean_hostkey_list(5), 1);
		assert_eq!(LocalXRay::<Test>::iter().count(), 4);

		System::set_block_number(21);
		assert_eq!(AresOcw::check_and_clean_hostkey_list(5), 2);
		assert_eq!(LocalXRay::<Test>::iter().count(), 3);
		assert_eq!(AresOcw::check_and_clean_hostkey_list(5), 1);
		System::set_block_number(25);
		assert_eq!(AresOcw::check_and_clean_hostkey_list(5), 4);
		assert_eq!(LocalXRay::<Test>::iter().count(), 0);
	});
}

#[test]
fn test_get_local_host_key() {
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	let (pool, _) = testing::TestTransactionPoolExt::new();
	t.register_extension(OffchainDbExt::new(offchain.clone()));
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));

	t.execute_with(|| {
		System::set_block_number(100);
		let (_, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();
		let (_, authority_2) = <Authorities<Test>>::get().unwrap()[1].clone();

		let local_host_key = AresOcw::get_local_host_key();
		println!("HOST KEY = {:?}", local_host_key);
		let local_host_key = AresOcw::get_local_host_key();
		println!("HOST KEY = {:?}", local_host_key);

		// let host_u8 = AresOcw::get_local_host_key_bytes();
		// println!("HOST KEY u8 = {:?}", &host_u8);
		// println!("{:?}", &hex::encode(local_host_key.encode().clone()));

		// 0x513f053a # 973,422,417 # 973422417

		println!("{:?}", &hex::encode(973422417u32.encode().clone()));
		println!("{:?}\n{:?}", hex::decode("513f053a"), 973422417u32.encode());
		let mut number =[0u8; 4]; // 0x 3a 05 3f 51
		number[..].copy_from_slice(hex::decode("513f053a").unwrap().as_slice());
		println!("{:?} result = {:?}", number, u32::from_be_bytes(number));

		LocalXRay::<Test>::insert(1, (100, RequestBaseVecU8::create_on_vec("a".encode()),  AuthorityAresVec::<AuraId>::create_on_vec( vec![authority_1.clone()]), true));
		assert_eq!(
			LocalXRay::<Test>::get(1),
			Some((100, RequestBaseVecU8::create_on_vec("a".encode()), AuthorityAresVec::<AuraId>::create_on_vec(vec![authority_1.clone()]), true))
		);

		LocalXRay::<Test>::insert(1, (200, RequestBaseVecU8::create_on_vec("b".encode()), AuthorityAresVec::<AuraId>::create_on_vec(vec![authority_2.clone()]), true));
		assert_eq!(
			LocalXRay::<Test>::get(1),
			Some((200, RequestBaseVecU8::create_on_vec("b".encode()), AuthorityAresVec::<AuraId>::create_on_vec(vec![authority_2.clone()]), true))
		);

	});
}

#[test]
fn test_fixbug_01161954() {
	// create_pre_check_task
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		let (stash_1, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();
		let (stash_2, authority_2) = <Authorities<Test>>::get().unwrap()[1].clone();
		let (stash_3, authority_3) = <Authorities<Test>>::get().unwrap()[2].clone();
		let (stash_4, authority_4) = <Authorities<Test>>::get().unwrap()[3].clone();

		AresOcw::create_pre_check_task(stash_1, authority_1.clone(), 1);
		AresOcw::create_pre_check_task(stash_2, authority_2.clone(), 2);
		let pre_check_list = PreCheckTaskList::<Test>::get().unwrap_or(PreCheckTaskListVec::<AccountId,AuraId, BlockNumber>::default());
		assert_eq!(pre_check_list.len(), 2);
		assert_eq!(
			pre_check_list,
			vec![(stash_1, authority_1.clone(), 1), (stash_2, authority_2.clone(), 2)]
		);

		AresOcw::create_pre_check_task(stash_2, authority_3.clone(), 6);
		let pre_check_list = PreCheckTaskList::<Test>::get().unwrap();
		assert_eq!(
			pre_check_list,
			vec![(stash_1, authority_1.clone(), 1), (stash_2, authority_3.clone(), 6)]
		);

		AresOcw::create_pre_check_task(stash_1, authority_1.clone(), 7); // Not change because stash and authority are all the same.
		AresOcw::create_pre_check_task(stash_2, authority_2.clone(), 8);
		AresOcw::create_pre_check_task(stash_3, authority_3.clone(), 9);
		AresOcw::create_pre_check_task(stash_4, authority_4.clone(), 10);

		let pre_check_list = PreCheckTaskList::<Test>::get().unwrap();
		assert_eq!(
			pre_check_list,
			vec![
				(stash_1, authority_1.clone(), 1),
				(stash_2, authority_2.clone(), 8),
				(stash_3, authority_3.clone(), 9),
				(stash_4, authority_4.clone(), 10),
			]
		);

		System::set_block_number(10);
		// test clean
		AresOcw::check_and_clean_obsolete_task(10);
		let pre_check_list = PreCheckTaskList::<Test>::get().unwrap();
		assert_eq!(
			pre_check_list,
			vec![
				(stash_1, authority_1.clone(), 1),
				(stash_2, authority_2.clone(), 8),
				(stash_3, authority_3.clone(), 9),
				(stash_4, authority_4.clone(), 10),
			]
		);

		System::set_block_number(12);
		// test clean
		AresOcw::check_and_clean_obsolete_task(10);
		let pre_check_list = PreCheckTaskList::<Test>::get().unwrap();
		assert_eq!(
			pre_check_list,
			vec![
				(stash_2, authority_2.clone(), 8),
				(stash_3, authority_3.clone(), 9),
				(stash_4, authority_4.clone(), 10),
			]
		);

		System::set_block_number(20);
		// test clean
		AresOcw::check_and_clean_obsolete_task(10);
		let pre_check_list = PreCheckTaskList::<Test>::get().unwrap();
		assert_eq!(pre_check_list, vec![(stash_4, authority_4.clone(), 10),]);
	});
}

#[test]
fn save_fetch_purchased_price_and_send_payload_signed_end_to_threshold() {
	let mut t = new_test_ext();

	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter3", PHRASE))).unwrap();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter4", PHRASE))).unwrap();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter5", PHRASE))).unwrap();

	let public_key_1: AuraId = get_account_id_from_seed::<AuraId>("hunter1").into();
	let public_key_2: AuraId = get_account_id_from_seed::<AuraId>("hunter2").into();
	let public_key_3: AuraId = get_account_id_from_seed::<AuraId>("hunter3").into();
	let public_key_4: AuraId = get_account_id_from_seed::<AuraId>("hunter4").into();
	let _public_key_5: AuraId = get_account_id_from_seed::<AuraId>("hunter5").into();

	println!("**** public_key_1 = {:?}", public_key_1);
	println!("**** public_key_2 = {:?}", public_key_2);
	println!("**** public_key_3 = {:?}", public_key_3);
	println!("**** public_key_4 = {:?}", public_key_4);

	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	let mut pub_purchase_id = OrderIdEnum::String(PurchaseId::default());
	t.execute_with(|| {
		System::set_block_number(1);

		let authroities = <Authorities<Test>>::get();
		println!("authroities = {:?}", authroities);

		let purchase_id =
			AresOcw::make_purchase_price_id(<Test as SigningTypes>::Public::from(public_key_1.clone()), 0);
		pub_purchase_id = purchase_id.clone();

		let price_payload_b1 = PurchasedPricePayload {
			block_number: 1, // type is BlockNumber
			auth: public_key_1.clone(),
			purchase_id: purchase_id.clone(),
			price: PricePayloadSubPriceList::create_on_vec( vec![PricePayloadSubPrice(
				PriceKey::create_on_vec("btc_price".as_bytes().to_vec()),
				502613720u64,
				4,
				JsonNumberValue {
					integer: 50261,
					fraction: 372,
					fraction_length: 3,
					exponent: 0,
				},
				1629699168,
			)]),
			public: <Test as SigningTypes>::Public::from(public_key_1.clone()),
		};

		// Add purchase price
		// Add purchased request.
		let request_acc = <Test as SigningTypes>::Public::from(public_key_1.clone());
		let offer = 10u64;
		// let submit_threshold = 100u8;
		let submit_threshold = Percent::from_percent(100); // 100u8;
		let max_duration = 3u64;
		let request_keys = RequestKeys::create_on_vec(   vec![ PriceKey::create_on_vec(to_test_vec("btc_price"))] );

		Balances::set_balance(Origin::root(), request_acc, 100000_000000000000, 0);
		assert_eq!(Balances::free_balance(request_acc), 100000_000000000000);

		// TODO:: remove under line.
		let _result =
			OracleFinance::reserve_fee(&request_acc, &purchase_id, request_keys.len() as u32);

		assert_ok!(AresOcw::ask_price(
			request_acc.clone(),
			offer,
			submit_threshold,
			max_duration,
			purchase_id.clone(),
			request_keys.clone()
		));

		let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
			AresOcw::fetch_purchased_request_keys(public_key_1.clone());
		let purchased_key = purchased_key_option.unwrap();
		assert_eq!(
			purchased_key.raw_source_keys,
			vec![(
				PriceKey::create_on_vec( "btc_price".as_bytes().to_vec() ),
				PriceToken::create_on_vec(  "btc".as_bytes().to_vec()),
				4,
			)]
		);

		AresOcw::save_fetch_purchased_price_and_send_payload_signed(1, public_key_1.clone()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 {price_payload: body, signature: signature}) = tx.call
		{
			// println!("signature = {:?}", signature);
			assert_eq!(body.clone(), price_payload_b1);
			let signature_valid =
				<PurchasedPricePayload<
					<Test as SigningTypes>::Public,
					<Test as frame_system::Config>::BlockNumber,
					AuraId, OrderIdEnum,
				> as SignedPayload<Test>>::verify::<crate::ares_crypto::AresCrypto<AuraId>>(&price_payload_b1, signature.clone());
			assert!(signature_valid);

			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_1.clone());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_1.clone());
			assert!(purchased_key_option.is_none());
			assert_eq!(TestAuthorityCount::get_validators_count(), 4);
		}
	});

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};
	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		System::set_block_number(2);
		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_2.clone()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) =
			tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_2.clone());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_2.clone());
			assert!(purchased_key_option.is_none());
			assert_eq!(TestAuthorityCount::get_validators_count(), 4);
		}
	});

	// ----------------------------

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};
	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		System::set_block_number(2);
		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_3.clone()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload:body, signature }
		) =
			tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_3.clone());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_3.clone());
			assert!(purchased_key_option.is_none());
		}

		assert_eq!(
			OracleFinance::get_record_point(
				888,
				&AresOcw::get_stash_id(&public_key_1.clone()).unwrap(),
				&pub_purchase_id,
			),
			None
		);
		assert_eq!(
			OracleFinance::get_record_point(
				888,
				&AresOcw::get_stash_id(&public_key_2.clone()).unwrap(),
				&pub_purchase_id,
			),
			None
		);
		assert_eq!(
			OracleFinance::get_record_point(
				888,
				&AresOcw::get_stash_id(&public_key_3.clone()).unwrap(),
				&pub_purchase_id,
			),
			None
		);

		println!("public_key_3 = {:?} ", public_key_3);
		println!("public_key_4 = {:?}", public_key_4);
		assert_eq!(
			OracleFinance::get_record_point(
				888,
				&AresOcw::get_stash_id(&public_key_4.clone()).unwrap(),
				&pub_purchase_id,
			),
			None
		);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);
	});

	// ------------- --------------

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};
	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		System::set_block_number(2);

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 1);
		assert_eq!(price_pool[0].2.len(), 3);
		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);
		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 3);

		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_4.clone()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) =
			tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_4.clone());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_4.clone());
			assert!(purchased_key_option.is_none());
		}

		assert_eq!(
			OracleFinance::get_record_point(
				OracleFinance::current_era_num(),
				&AresOcw::get_stash_id(&public_key_1).unwrap(),
				&pub_purchase_id,
			),
			Some(1)
		);
		assert_eq!(
			OracleFinance::get_record_point(
				OracleFinance::current_era_num(),
				&AresOcw::get_stash_id(&public_key_2).unwrap(),
				&pub_purchase_id,
			),
			Some(1)
		);
		assert_eq!(
			OracleFinance::get_record_point(
				OracleFinance::current_era_num(),
				&AresOcw::get_stash_id(&public_key_3).unwrap(),
				&pub_purchase_id,
			),
			Some(1)
		);
		assert_eq!(
			OracleFinance::get_record_point(
				OracleFinance::current_era_num(),
				&AresOcw::get_stash_id(&public_key_4).unwrap(),
				&pub_purchase_id,
			),
			Some(1)
		);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 1);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 1);
	});
}

#[test]
fn save_fetch_purchased_price_and_send_payload_signed_end_to_duration_with_an_err() {
	let mut t = new_test_ext();

	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(0)
		.unwrap()
		.clone();

	let _public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(1)
		.unwrap()
		.clone();

	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		let current_bn: u64 = 1;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		// Add purchase price
		// Add purchased request.
		let request_acc = AccountId::from_raw([1; 32]);
		let offer = 10u64;
		// let submit_threshold = 60u8;
		let submit_threshold = Percent::from_percent(60); // 60u8;
		let max_duration = 3u64;
		let request_keys = vec![to_test_vec("btc_price")];

		// check finance pallet status.
		assert_eq!(Balances::free_balance(request_acc.into_account()), 100000000000000000);
		let purchase_id = AresOcw::make_purchase_price_id(request_acc.into_account(), 0);
		OracleFinance::reserve_fee(
			&request_acc.into_account(),
			&purchase_id,
			request_keys.len() as u32,
		);
		assert_eq!(
			Balances::free_balance(request_acc.into_account()),
			100000000000000000 - DOLLARS * 1
		);

		assert_ok!(AresOcw::ask_price(
			request_acc.clone(),
			offer,
			submit_threshold, // 1 + 3 = 4
			max_duration,
			purchase_id.clone(),
			RequestKeys::create_on_vec(request_keys.clone().into_iter().map(|x|{ PriceKey::create_on_vec(x) }).collect())
		));

		let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
			AresOcw::fetch_purchased_request_keys(public_key_1.into());
		let purchased_key = purchased_key_option.unwrap();
		assert_eq!(
			purchased_key.raw_source_keys,
			RawSourceKeys::create_on_vec( vec![(
				PriceKey::create_on_vec( "btc_price".as_bytes().to_vec() ),
				PriceToken::create_on_vec( "btc".as_bytes().to_vec() ),
				4)]
			)
		);

		System::set_block_number(2);
		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_1.into()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) =
			tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_1.clone().into());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_1.clone().into());
			assert!(purchased_key_option.is_none());
			assert_eq!(TestAuthorityCount::get_validators_count(), 4);
		}

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 1);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 1);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);
	});

	t.execute_with(|| {
		System::set_block_number(2);

		let expired_purchased_list = AresOcw::get_expired_purchased_transactions();
		assert_eq!(expired_purchased_list.len(), 0);

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 1);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 1);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);
	});

	t.execute_with(|| {

		// System::set_block_number(1);
		// let request_acc = AccountId::from_raw([1; 32]);
		// let test_purchased_id = AresOcw::make_purchase_price_id(request_acc.into_account(), 0);
		System::set_block_number(4);

		let expired_purchased_list = AresOcw::get_expired_purchased_transactions();
		assert_eq!(expired_purchased_list.len(), 1);

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 1);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 1);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);

		// Forced to settle
		AresOcw::save_forced_clear_purchased_price_payload_signed(3, public_key_1.into()).unwrap();

		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_forced_clear_purchased_price_payload_signed
							 { price_payload: body, signature }
		) = tx.call
		{
			// Test purchased submit call
			AresOcw::submit_forced_clear_purchased_price_payload_signed(Origin::none(), body, signature);
		}

		// println!("|{:?}|", System::events());
		System::assert_last_event(Event::AresOcw(
			AresOcwEvent::InsufficientCountOfValidators{purchase_id: OrderIdEnum::String(PurchaseId::try_from(vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap()) }
		));

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 0);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 0);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);

		let request_acc = AccountId::from_raw([1; 32]);
		assert_eq!(Balances::free_balance(request_acc.into_account()), 100000000000000000);
	});
}

#[test]
fn save_fetch_purchased_price_and_send_payload_signed_end_to_duration_with_force_clean() {
	let mut t = new_test_ext();

	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter3", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(0)
		.unwrap()
		.clone();

	let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(1)
		.unwrap()
		.clone();

	let public_key_3 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(2)
		.unwrap()
		.clone();

	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		System::set_block_number(1);

		// Add purchase price
		// Add purchased request.
		let request_acc = AccountId::from_raw([1; 32]);
		let offer = 10u64;
		// let submit_threshold = 60u8;
		let submit_threshold = Percent::from_percent(60) ; //60u8;
		let max_duration = 3u64;
		let request_keys = vec![to_test_vec("btc_price")];
		Balances::set_balance(Origin::root(), request_acc, 100000_000000000000, 0);
		assert_eq!(Balances::free_balance(request_acc), 100000_000000000000);
		let purchase_id = AresOcw::make_purchase_price_id(request_acc.into_account(), 0);
		let _result =
			OracleFinance::reserve_fee(&request_acc, &purchase_id, request_keys.len() as u32);
		assert_ok!(AresOcw::ask_price(
			request_acc.clone(),
			offer,
			submit_threshold, // 1 + 3 = 4
			max_duration,
			purchase_id,
			RequestKeys::create_on_vec(request_keys.clone().into_iter().map(|x|{PriceKey::create_on_vec(x)}).collect())
		));

		let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
			AresOcw::fetch_purchased_request_keys(public_key_1.into());
		let purchased_key = purchased_key_option.unwrap();
		assert_eq!(
			purchased_key.raw_source_keys,
			RawSourceKeys::create_on_vec( vec![(
				PriceKey::create_on_vec( "btc_price".as_bytes().to_vec()),
				PriceToken::create_on_vec( "btc".as_bytes().to_vec()),
				4)]
			)
		);

		System::set_block_number(2);
		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_1.into()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) =
			tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_1.into());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_1.into());
			assert!(purchased_key_option.is_none());
			assert_eq!(TestAuthorityCount::get_validators_count(), 4);
		}

		let price_pool = <PurchasedPricePool<Test>>::get(
			purchased_key.purchase_id.clone(),
			PriceKey::create_on_vec(to_test_vec("btc_price"))
		).unwrap();
		assert_eq!(price_pool.len(), 1);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 1);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);
	});

	// ---------------------
	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		System::set_block_number(2);

		let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
			AresOcw::fetch_purchased_request_keys(public_key_2.into());
		let purchased_key = purchased_key_option.unwrap();
		assert_eq!(
			purchased_key.raw_source_keys,
			vec![(
				PriceKey::create_on_vec("btc_price".as_bytes().to_vec()),
				PriceToken::create_on_vec("btc".as_bytes().to_vec()),
				4)]
		);

		System::set_block_number(2);
		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_2.into()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) =
			tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_2.clone().into());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_2.clone().into());
			assert!(purchased_key_option.is_none());
			assert_eq!(TestAuthorityCount::get_validators_count(), 4);
		}

		let price_pool = <PurchasedPricePool<Test>>::get(
			purchased_key.purchase_id.clone(),
			PriceKey::create_on_vec(to_test_vec("btc_price"))
		).unwrap();
		assert_eq!(price_pool.len(), 2);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 2);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);
	});
	// ---------------------

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		System::set_block_number(2);

		let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
			AresOcw::fetch_purchased_request_keys(public_key_3.into());
		let purchased_key = purchased_key_option.unwrap();
		assert_eq!(
			purchased_key.raw_source_keys,
			RawSourceKeys::create_on_vec(vec![(
				PriceKey::create_on_vec("btc_price".as_bytes().to_vec()),
				PriceToken::create_on_vec("btc".as_bytes().to_vec()),
				4)])
		);

		System::set_block_number(2);
		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_3.into()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) =
			tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_3.clone().into());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_3.clone().into());
			assert!(purchased_key_option.is_none());
			assert_eq!(TestAuthorityCount::get_validators_count(), 4);
		}

		let price_pool = <PurchasedPricePool<Test>>::get(
			purchased_key.purchase_id.clone(),
			PriceKey::create_on_vec(to_test_vec("btc_price"))
		).unwrap();
		assert_eq!(price_pool.len(), 3);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 3);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);
	});
	// ---------------------

	t.execute_with(|| {
		System::set_block_number(2);

		let expired_purchased_list = AresOcw::get_expired_purchased_transactions();
		assert_eq!(expired_purchased_list.len(), 0);

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 1);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 3);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);
	});

	t.execute_with(|| {
		System::set_block_number(4);

		let expired_purchased_list = AresOcw::get_expired_purchased_transactions();
		assert_eq!(expired_purchased_list.len(), 1);

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 1);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 3);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);

		// Forced to settle
		AresOcw::save_forced_clear_purchased_price_payload_signed(4, public_key_1.into()).unwrap();

		println!(" --------- clean down.");
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_forced_clear_purchased_price_payload_signed
							 { price_payload: body, signature }
		) = tx.call
		{
			// Test purchased submit call
			AresOcw::submit_forced_clear_purchased_price_payload_signed(Origin::none(), body, signature);
		}

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 0);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 0);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 1);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 1);
	});
}

#[test]
fn save_fetch_purchased_price_and_send_payload_signed_end_to_part_success() {
	let mut t = new_test_ext();

	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter3", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter4", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter5", PHRASE))).unwrap();

	let public_key_1: AuraId = get_account_id_from_seed::<AuraId>("hunter1").into();
	let public_key_2: AuraId = get_account_id_from_seed::<AuraId>("hunter2").into();
	let public_key_3: AuraId = get_account_id_from_seed::<AuraId>("hunter3").into();
	let public_key_4: AuraId = get_account_id_from_seed::<AuraId>("hunter4").into();
	let public_key_5: AuraId = get_account_id_from_seed::<AuraId>("hunter5").into();

	println!("**** public_key_1 = {:?}", public_key_1);
	println!("**** public_key_2 = {:?}", public_key_2);
	println!("**** public_key_3 = {:?}", public_key_3);
	println!("**** public_key_4 = {:?}", public_key_4);

	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	let mut pub_purchase_id = OrderIdEnum::String(PurchaseId::default());
	t.execute_with(|| {
		System::set_block_number(1);

		let authroities = <Authorities<Test>>::get();
		println!("authroities = {:?}", authroities);

		let purchase_id =
			AresOcw::make_purchase_price_id(<Test as SigningTypes>::Public::from(public_key_1.clone()), 0);
		pub_purchase_id = purchase_id.clone();
		let price_payload_b1 = PurchasedPricePayload {
			block_number: 1, // type is BlockNumber
			auth: public_key_1.clone(),
			purchase_id: purchase_id.clone(),
			price: PricePayloadSubPriceList::create_on_vec( vec![PricePayloadSubPrice(
				PriceKey::create_on_vec("btc_price".as_bytes().to_vec()),
				502613720u64,
				4,
				JsonNumberValue {
					integer: 50261,
					fraction: 372,
					fraction_length: 3,
					exponent: 0,
				},
				1629699168,
			)] ) ,
			public: <Test as SigningTypes>::Public::from(public_key_1.clone()),
		};

		// Add purchase price
		// Add purchased request.
		let request_acc = <Test as SigningTypes>::Public::from(public_key_1.clone());
		let offer = 10u64;
		// let submit_threshold = 100u8;
		let submit_threshold = Percent::from_percent(100);// 100u8;
		let max_duration = 3u64;
		let request_keys = vec![to_test_vec("btc_price"), to_test_vec("doge_price")];

		Balances::set_balance(Origin::root(), request_acc, 100000_000000000000, 0);
		assert_eq!(Balances::free_balance(request_acc), 100000_000000000000);

		// TODO:: remove under line.
		let result =
			OracleFinance::reserve_fee(&request_acc, &purchase_id, request_keys.len() as u32);
		assert_eq!(Balances::free_balance(request_acc), 99998000000000000);

		assert_ok!(AresOcw::ask_price(
			request_acc.clone(),
			offer,
			submit_threshold,
			max_duration,
			purchase_id.clone(),
			RequestKeys::create_on_vec(request_keys.clone().into_iter().map(|x|{PriceKey::create_on_vec(x)}).collect())
		));

		let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
			AresOcw::fetch_purchased_request_keys(public_key_1.clone());
		let purchased_key = purchased_key_option.unwrap();
		assert_eq!(
			purchased_key.raw_source_keys,
			RawSourceKeys::create_on_vec((vec![(
				PriceKey::create_on_vec("btc_price".as_bytes().to_vec()),
				PriceToken::create_on_vec("btc".as_bytes().to_vec()),
				4)]))
		);

		AresOcw::save_fetch_purchased_price_and_send_payload_signed(1, public_key_1.clone()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 {price_payload: body, signature: signature}) =
		tx.call
		{
			assert_eq!(body.clone(), price_payload_b1);
			let signature_valid =
				<PurchasedPricePayload<
					<Test as SigningTypes>::Public,
					<Test as frame_system::Config>::BlockNumber,
					AuraId, OrderIdEnum,
				> as SignedPayload<Test>>::verify::<crate::ares_crypto::AresCrypto<AuraId>>(&price_payload_b1, signature.clone());
			assert!(signature_valid);

			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_1.clone());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_1.clone());
			assert!(purchased_key_option.is_none());
			assert_eq!(TestAuthorityCount::get_validators_count(), 4);
		}
	});

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};
	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		System::set_block_number(2);
		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_2.clone()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) =
		tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_2.clone());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_2.clone());
			assert!(purchased_key_option.is_none());
			assert_eq!(TestAuthorityCount::get_validators_count(), 4);
		}
	});

	// ----------------------------

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};
	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		System::set_block_number(2);
		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_3.clone()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload:body, signature }
		) =
		tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_3.clone());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_3.clone());
			assert!(purchased_key_option.is_none());
		}

		assert_eq!(
			OracleFinance::get_record_point(
				888,
				&AresOcw::get_stash_id(&public_key_1.clone()).unwrap(),
				&pub_purchase_id,
			),
			None
		);
		assert_eq!(
			OracleFinance::get_record_point(
				888,
				&AresOcw::get_stash_id(&public_key_2.clone()).unwrap(),
				&pub_purchase_id,
			),
			None
		);
		assert_eq!(
			OracleFinance::get_record_point(
				888,
				&AresOcw::get_stash_id(&public_key_3.clone()).unwrap(),
				&pub_purchase_id,
			),
			None
		);

		println!("public_key_3 = {:?} ", public_key_3);
		println!("public_key_4 = {:?}", public_key_4);
		assert_eq!(
			OracleFinance::get_record_point(
				888,
				&AresOcw::get_stash_id(&public_key_4.clone()).unwrap(),
				&pub_purchase_id,
			),
			None
		);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);
	});

	// ------------- --------------

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};
	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		System::set_block_number(2);

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 1);
		assert_eq!(price_pool[0].2.len(), 3);
		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);
		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 3);

		AresOcw::save_fetch_purchased_price_and_send_payload_signed(2, public_key_4.clone()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_purchased_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) =
		tx.call
		{
			// Test purchased submit call
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_4.clone());
			assert!(purchased_key_option.is_some());
			AresOcw::submit_purchased_price_unsigned_with_signed_payload(Origin::none(), body, signature);
			let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
				AresOcw::fetch_purchased_request_keys(public_key_4.clone());
			assert!(purchased_key_option.is_none());
		}

		assert_eq!(
			OracleFinance::get_record_point(
				OracleFinance::current_era_num(),
				&AresOcw::get_stash_id(&public_key_1).unwrap(),
				&pub_purchase_id,
			),
			Some(1)
		);
		assert_eq!(
			OracleFinance::get_record_point(
				OracleFinance::current_era_num(),
				&AresOcw::get_stash_id(&public_key_2).unwrap(),
				&pub_purchase_id,
			),
			Some(1)
		);
		assert_eq!(
			OracleFinance::get_record_point(
				OracleFinance::current_era_num(),
				&AresOcw::get_stash_id(&public_key_3).unwrap(),
				&pub_purchase_id,
			),
			Some(1)
		);
		assert_eq!(
			OracleFinance::get_record_point(
				OracleFinance::current_era_num(),
				&AresOcw::get_stash_id(&public_key_4).unwrap(),
				&pub_purchase_id,
			),
			Some(1)
		);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 1);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 1);
	});
}

#[test]
fn test_get_auth_id_from_stash_id() {
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		System::set_block_number(2);
	});
}
#[test]
fn test_submit_ask_price() {
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		System::set_block_number(2);

		AresOcw::submit_ask_price(
			Origin::signed(AccountId::from_raw([1; 32])),
			20_00000000000000,
			to_test_vec("btc_price,eth_price"),
		);

		// Get an authority id
		// let authority_list = <Authorities<Test>>::get();
		// let (_, authority_1) = <Authorities<Test>>::get()[0].clone();
		let (_, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();

		let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
			AresOcw::fetch_purchased_request_keys(authority_1.clone());

		let purchased_key = purchased_key_option.unwrap();

		assert_eq!(
			purchased_key.raw_source_keys,
			vec![
				(PriceKey::create_on_vec("btc_price".as_bytes().to_vec()), PriceToken::create_on_vec("btc".as_bytes().to_vec()), 4),
				(PriceKey::create_on_vec("eth_price".as_bytes().to_vec()), PriceToken::create_on_vec("eth".as_bytes().to_vec()), 4),
			]
		);

		let price_pool = <PurchasedPricePool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(price_pool.len(), 0);

		let request_pool = <PurchasedRequestPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(request_pool.len(), 1);

		let order_pool = <PurchasedOrderPool<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let order_pool = <PurchasedAvgPrice<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(order_pool.len(), 0);

		let avg_trace = <PurchasedAvgTrace<Test>>::iter().collect::<Vec<_>>();
		assert_eq!(avg_trace.len(), 0);

		let request_data: PurchasedRequestData<AccountId, Balance, BlockNumber> =
			<PurchasedRequestPool<Test>>::get(purchased_key.purchase_id.clone()).unwrap();
		assert_eq!(request_data.account_id.clone(), AccountId::from_raw([1; 32]));
		assert_eq!(
			request_data.request_keys.clone(),
			vec![PriceKey::create_on_vec(to_test_vec("btc_price")), PriceKey::create_on_vec(to_test_vec("eth_price")),]
		);
		assert_eq!(
			request_data.max_duration.clone(),
			PurchasedDefaultData::default().max_duration + 2
		);
		assert_eq!(
			request_data.submit_threshold.clone(),
			PurchasedDefaultData::default().submit_threshold
		);

		assert_eq!(
			request_data.offer.clone(),
			BasicDollars::get() * (request_data.request_keys.len() as u64)
		);
	});
}

#[test]
fn update_purchase_avg_price_storage() {
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		System::set_block_number(2);

		let (stash_1, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();
		let (stash_2, authority_2) = <Authorities<Test>>::get().unwrap()[1].clone();
		let (stash_3, authority_3) = <Authorities<Test>>::get().unwrap()[2].clone();
		let (stash_4, authority_4) = <Authorities<Test>>::get().unwrap()[3].clone();

		// add ask
		PurchasedRequestPool::<Test>::insert(
			to_purchase_id("abc"),
			PurchasedRequestData {
				account_id: AccountId::from_raw([0; 32]),
				offer: 0,
				// submit_threshold: 60,
				submit_threshold: Percent::from_percent(60), //60,
				create_bn: Default::default(),
				max_duration: 0,
				request_keys: Default::default(),
			},
		);
		// add ask
		PurchasedRequestPool::<Test>::insert(
			to_purchase_id("bcd"),
			PurchasedRequestData {
				account_id: AccountId::from_raw([0; 32]),
				offer: 0,
				// submit_threshold: 60,
				submit_threshold: Percent::from_percent(60), //60,
				create_bn: Default::default(),
				max_duration: 0,
				request_keys: Default::default(),
			},
		);

		// add price on purchased_id = abc
		PurchasedPricePool::<Test>::insert(
			to_purchase_id("abc"),
			PriceKey::create_on_vec("btc_price".encode()),
			PurchasedPriceDataVec::<AccountId, BlockNumber>::create_on_vec(vec![
				AresPriceData {
					price: 100,
					account_id: stash_1.clone(),
					create_bn: 1,
					fraction_len: 2,
					raw_number: Default::default(),
					timestamp: 0,
					update_bn: 2,
				},
				AresPriceData {
					price: 101,
					account_id: stash_2.clone(),
					create_bn: 1,
					fraction_len: 2,
					raw_number: Default::default(),
					timestamp: 0,
                    update_bn: 2,
                },
				AresPriceData {
					price: 103,
					account_id: stash_3.clone(),
					create_bn: 1,
					fraction_len: 2,
					raw_number: Default::default(),
					timestamp: 0,
					update_bn: 2,
				},
			]),
		);

		// add price on purchased_id = bcd
		PurchasedPricePool::<Test>::insert(
			to_purchase_id("bcd"),
			PriceKey::create_on_vec("btc_price".encode()),
			PurchasedPriceDataVec::<AccountId, BlockNumber>::create_on_vec(vec![
				AresPriceData {
					price: 200,
					account_id: stash_1.clone(),
					create_bn: 1,
					fraction_len: 2,
					raw_number: Default::default(),
					timestamp: 0,
					update_bn: 2,
				},
				AresPriceData {
					price: 201,
					account_id: stash_2.clone(),
					create_bn: 1,
					fraction_len: 2,
					raw_number: Default::default(),
					timestamp: 0,
					update_bn: 2,
				},
				AresPriceData {
					price: 203,
					account_id: stash_3.clone(),
					create_bn: 1,
					fraction_len: 2,
					raw_number: Default::default(),
					timestamp: 0,
					update_bn: 2,
				},
			]),
		);

		// check store status
		let price_pool = <PurchasedPricePool<Test>>::get(
			to_purchase_id("abc"),
			PriceKey::create_on_vec("btc_price".encode())
		).unwrap();
		assert_eq!(price_pool.len(), 3);
		// update
		AresOcw::update_purchase_avg_price_storage(to_purchase_id("abc"), PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE);
		AresOcw::purchased_storage_clean(to_purchase_id("abc"));
		// Get avg
		let avg_price = <PurchasedAvgPrice<Test>>::get(
			to_purchase_id("abc"),
			PriceKey::create_on_vec("btc_price".encode())
		);
		assert_eq!(
			avg_price,
			PurchasedAvgPriceData {
				create_bn: 2,
				reached_type: PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE,
				price_data: (101, 2)
			}
		);
		let price_pool = <PurchasedPricePool<Test>>::get(
			to_purchase_id("abc"),
			PriceKey::create_on_vec("btc_price".encode())
		).unwrap_or(Default::default());
		assert_eq!(price_pool.len(), 0);

		// -----------------

		let price_pool = <PurchasedPricePool<Test>>::get(
			to_purchase_id("bcd"),
			PriceKey::create_on_vec("btc_price".encode())
		).unwrap();
		assert_eq!(price_pool.len(), 3);
		// update
		AresOcw::update_purchase_avg_price_storage(to_purchase_id("bcd"), PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE);
		AresOcw::purchased_storage_clean(to_purchase_id("bcd"));
		// Get avg
		let avg_price = <PurchasedAvgPrice<Test>>::get(
			to_purchase_id("bcd"),
			PriceKey::create_on_vec("btc_price".encode())
		);
		assert_eq!(
			avg_price,
			PurchasedAvgPriceData {
				create_bn: 2,
				reached_type: PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE,
				price_data: (201, 2)
			}
		);
		let price_pool = <PurchasedPricePool<Test>>::get(
			to_purchase_id("bcd"),
			PriceKey::create_on_vec("btc_price".encode())
		).unwrap_or(Default::default());
		assert_eq!(price_pool.len(), 0);

		// -----------------
	});
}

#[test]
fn test_is_validator_purchased_threshold_up_on() {
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		assert_eq!(false, AresOcw::is_validator_purchased_threshold_up_on(to_purchase_id("abc")));
		// add ask
		PurchasedRequestPool::<Test>::insert(
			to_purchase_id("abc"),
			PurchasedRequestData {
				account_id: AccountId::from_raw([0;32]),
				offer: 0,
				// submit_threshold: 60,
				submit_threshold: Percent::from_percent(60), //60,
				create_bn: 1,
				max_duration: 0,
				request_keys: Default::default(),
			},
		);
		// check
		assert_eq!(false, AresOcw::is_validator_purchased_threshold_up_on(to_purchase_id("abc")));

		// Get authority ids.

		let (stash_1, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();
		let (stash_2, authority_2) = <Authorities<Test>>::get().unwrap()[1].clone();
		let (stash_3, authority_3) = <Authorities<Test>>::get().unwrap()[2].clone();
		let (stash_4, authority_4) = <Authorities<Test>>::get().unwrap()[3].clone();

		AresOcw::add_purchased_price(
			to_purchase_id("abc"),
			stash_1.clone(),
			1,
			PricePayloadSubPriceList::create_on_vec(vec![PricePayloadSubPrice(
				PriceKey::create_on_vec("btc_price".encode()),
				300000,
				4,
				Default::default(),
				0,
			)]),
		);
		// check
		assert_eq!(false, AresOcw::is_validator_purchased_threshold_up_on(to_purchase_id("abc")));

		AresOcw::add_purchased_price(
			to_purchase_id("abc"),
			stash_2.clone(),
			1,
			PricePayloadSubPriceList::create_on_vec(vec![PricePayloadSubPrice(
				PriceKey::create_on_vec("btc_price".encode()),
				300000,
				4,
				Default::default(),
				0,
			)]),
		);
		// check
		assert_eq!(false, AresOcw::is_validator_purchased_threshold_up_on(to_purchase_id("abc")));

		AresOcw::add_purchased_price(
			to_purchase_id("abc"),
			stash_3.clone(),
			1,
			PricePayloadSubPriceList::create_on_vec(vec![PricePayloadSubPrice(
				PriceKey::create_on_vec("btc_price".encode()),
				300000,
				4,
				Default::default(),
				0,
			)]),
		);
		// check
		assert_eq!(true, AresOcw::is_validator_purchased_threshold_up_on(to_purchase_id(("abc"))));
	});
}

#[test]
fn test_ask_price() {
	let mut t = new_test_ext();
	let (offchain, _state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		System::set_block_number(1);
		//
		let account_id1 = AccountId::from_raw([1; 32]);
		let offer = 10u64;
		// let submit_threshold = 100u8;
		let submit_threshold = Percent::from_percent(100); //100u8;
		let max_duration = 3u64;
		let request_keys = vec![to_test_vec("btc_price"), to_test_vec("eth_price")];

		let test_purchase_price_id = AresOcw::make_purchase_price_id(account_id1.clone(), 0);
		// request_num will be count in the ask_price function.
		let result = AresOcw::ask_price(
			account_id1.clone(),
			offer,
			submit_threshold,
			max_duration,
			test_purchase_price_id,
			RequestKeys::create_on_vec(request_keys.clone().into_iter().map(|x|{PriceKey::create_on_vec(x)}).collect() ),
		);
		assert!(result.is_ok());
		let purchase_id = if let OrderIdEnum::String(order_str) = result.unwrap() {
			Some(order_str)
		}else{
			None
		};
		assert!(purchase_id.is_some());
		let purchase_id = purchase_id.unwrap();

		// println!("{:?}", &hex::encode(purchase_id.clone()));
		assert_eq!(
			&hex::encode(purchase_id.clone()),
			"0101010101010101010101010101010101010101010101010101010101010101010000000000000000"
		);
		assert_eq!(
			AresOcw::purchased_request_pool(OrderIdEnum::String(purchase_id)).unwrap(),
			PurchasedRequestData {
				account_id: account_id1.clone(),
				offer,
				submit_threshold,
				create_bn: 1,
				max_duration: max_duration + 1,
				request_keys: RequestKeys::create_on_vec(request_keys.clone().into_iter().map(|x|{PriceKey::create_on_vec(x)}).collect() )
			}
		);

		let test_purchase_price_id = AresOcw::make_purchase_price_id(account_id1.clone(), 0);
		let result = AresOcw::ask_price(
			account_id1.clone(),
			offer,
			submit_threshold,
			max_duration,
			test_purchase_price_id.clone(),
			RequestKeys::create_on_vec(request_keys.clone().into_iter().map(|x|{PriceKey::create_on_vec(x)}).collect() ),
		);
		assert!(result.is_ok());
		assert_eq!(&result, &Ok(test_purchase_price_id));
		let purchase_id = if let OrderIdEnum::String(order_str) = result.unwrap() {
			Some(order_str)
		}else{
			None
		};
		assert!(purchase_id.is_some());
		let purchase_id = purchase_id.unwrap();

		assert_eq!(
			&hex::encode(purchase_id.clone()),
			"0101010101010101010101010101010101010101010101010101010101010101010000000000000001"
		);
		assert_eq!(
			AresOcw::purchased_request_pool(OrderIdEnum::String(purchase_id)).unwrap(),
			PurchasedRequestData {
				account_id: account_id1.clone(),
				offer,
				submit_threshold,
				create_bn: 1,
				max_duration: max_duration + 1,
				request_keys: RequestKeys::create_on_vec(request_keys.clone().into_iter().map(|x|{PriceKey::create_on_vec(x)}).collect() )
			}
		);
	});
}

#[test]
fn test_fetch_purchased_request_keys() {
	let mut t = new_test_ext();
	let (offchain, state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		System::set_block_number(1);

		let (_, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();

		// Initial status request_keys is empty.
		let request_keys = AresOcw::fetch_purchased_request_keys(authority_1.clone());
		assert_eq!(request_keys, None);

		// Add purchased request.
		let request_acc = AccountId::from_raw([1; 32]);
		let offer = 10u64;
		// let submit_threshold = 100u8;
		let submit_threshold = Percent::from_percent(100); //100u8;
		let max_duration = 3u64;
		let request_keys = vec![to_test_vec("btc_price"), to_test_vec("eth_price")];

		let test_purchased_id = AresOcw::make_purchase_price_id(request_acc.clone(), 0);
		assert_ok!(AresOcw::ask_price(
			request_acc,
			offer,
			submit_threshold,
			max_duration,
			test_purchased_id,
			RequestKeys::create_on_vec(request_keys.clone().into_iter().map(|x|{PriceKey::create_on_vec(x)}).collect() )
		));

		// Get purchased request list
		let mut expect_format = RawSourceKeys::default();
		expect_format.try_push((PriceKey::create_on_vec("btc_price".as_bytes().to_vec()), PriceToken::create_on_vec("btc".as_bytes().to_vec()), 4));
		expect_format.try_push((PriceKey::create_on_vec("eth_price".as_bytes().to_vec()), PriceToken::create_on_vec("eth".as_bytes().to_vec()), 4));

		let purchased_key_option: Option<PurchasedSourceRawKeys<OrderIdEnum>> =
			AresOcw::fetch_purchased_request_keys(authority_1.clone());
		let purchased_key = purchased_key_option.unwrap();
		assert_eq!(purchased_key.raw_source_keys, expect_format);
		let purchased_id = purchased_key.purchase_id;
		let purchased_id = if let OrderIdEnum::String(order_id) = purchased_id {
			Some(order_id)
		}else{
			None
		};
		assert!(purchased_id.is_some());
		let purchased_id = purchased_id.unwrap();
		assert_eq!(
			&hex::encode(purchased_id),
			"0101010101010101010101010101010101010101010101010101010101010101010000000000000000"
		);
	});
}

#[test]
fn test_extract_purchased_request() {
	let mut t = new_test_ext();
	let (offchain, state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		// extract str
		let extract_str = "btc_price,eth_price,";
		assert_eq!(
			RequestKeys::create_on_vec(vec![
				PriceKey::create_on_vec(to_test_vec("btc_price")),
				PriceKey::create_on_vec(to_test_vec("eth_price")),
			]),
			AresOcw::extract_purchased_request(to_test_vec(extract_str))
		);

		let extract_str = "btc_price,eth_price";
		assert_eq!(
			RequestKeys::create_on_vec(vec![
				PriceKey::create_on_vec(to_test_vec("btc_price")),
				PriceKey::create_on_vec(to_test_vec("eth_price")),
			]),
			AresOcw::extract_purchased_request(to_test_vec(extract_str))
		);

		let extract_str = "btc_price,eth_price,eth_price,";
		assert_eq!(
			RequestKeys::create_on_vec(vec![
				PriceKey::create_on_vec(to_test_vec("btc_price")),
				PriceKey::create_on_vec(to_test_vec("eth_price")),
			]),
			AresOcw::extract_purchased_request(to_test_vec(extract_str))
		);

		let extract_str = "btc_price,eth_price,btc_price,";
		assert_eq!(
			RequestKeys::create_on_vec(vec![
				PriceKey::create_on_vec(to_test_vec("btc_price")),
				PriceKey::create_on_vec(to_test_vec("eth_price")),
			]),
			AresOcw::extract_purchased_request(to_test_vec(extract_str))
		);
	});
}

#[test]
fn test_update_purchased_param() {
	let mut t = new_test_ext();
	let (offchain, state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		assert_eq!(AresOcw::purchased_default_setting(), PurchasedDefaultData::default());
		// submit_threshold: u8, max_duration: u64, unit_price: u64
		// assert_ok!(AresOcw::update_purchased_param(Origin::root(), 10, 20, 30));
		assert_ok!(AresOcw::update_purchased_param(Origin::root(), Percent::from_percent(10), 20, 30));
		assert_eq!(
			AresOcw::purchased_default_setting(),
			PurchasedDefaultData {
				// submit_threshold: 10,
				submit_threshold: Percent::from_percent(10), // 10,
				max_duration: 20,
				avg_keep_duration: 30,
				// unit_price: 30,
			}
		);
	});
}

#[test]
fn test_calculation_average_price() {
	let (offchain, _state) = testing::TestOffchainExt::new();
	let mut t = sp_io::TestExternalities::default();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		assert_eq!(
			Some(6),
			AresOcw::calculation_average_price(vec![6, 2, 1, 18, 3], CALCULATION_KIND_AVERAGE)
		);
		assert_eq!(
			Some(3),
			AresOcw::calculation_average_price(vec![6, 2, 1, 18, 3], CALCULATION_KIND_MEDIAN)
		);
		assert_eq!(
			Some(17),
			AresOcw::calculation_average_price(vec![3, 45, 18, 3], CALCULATION_KIND_AVERAGE)
		);
		assert_eq!(
			Some(10),
			AresOcw::calculation_average_price(vec![3, 45, 18, 3], CALCULATION_KIND_MEDIAN)
		);
		assert_eq!(
			Some(5),
			AresOcw::calculation_average_price(vec![6, 5, 5], CALCULATION_KIND_AVERAGE)
		);
		assert_eq!(
			Some(5),
			AresOcw::calculation_average_price(vec![6, 5, 5], CALCULATION_KIND_MEDIAN)
		);
		assert_eq!(
			Some(70),
			AresOcw::calculation_average_price(vec![70], CALCULATION_KIND_AVERAGE)
		);
		assert_eq!(
			Some(70),
			AresOcw::calculation_average_price(vec![70], CALCULATION_KIND_MEDIAN)
		);
	});
}

#[test]
fn addprice_of_ares() {
	let (offchain, _state) = testing::TestOffchainExt::new();
	// let mut t = sp_io::TestExternalities::default();
	let mut t = new_test_ext();
	t.register_extension(OffchainWorkerExt::new(offchain));

	t.execute_with(|| {
		// The request key must be configured, otherwise you cannot submit the price. so you need =>
		// new_test_ext()

		System::set_block_number(1);
		// when
		let price_key = "btc_price".as_bytes().to_vec(); // PriceKey::PriceKeyIsBTC ;
		AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 8888, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);
		AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 9999, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);

		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		assert_eq!(
			0,
			btc_price_list.len(),
			"Price list will be empty when the average calculation."
		);

		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone()));
		assert_eq!( bet_avg_price, Some(((8888 + 9999) / 2, 4, System::block_number())), "Only test ares_avg_prices ");

		// Add a new value beyond the array boundary.
		AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 3333, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);
		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());

		// (price_val, accountid, block_num, fraction_num)
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
				// (3333, Default::default(), 1, 4, Default::default())
				AresPriceData {
					price: 3333,
					account_id: AccountId::from_raw([0;32]),
					create_bn: 1,
					fraction_len: 4,
					raw_number: Default::default(),
					timestamp: 0,
					update_bn: 1
				}
			]),
			btc_price_list
		);

		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec()).clone());
		assert_eq!(bet_avg_price, Some(((8888 + 9999) / 2, 4, System::block_number())));

		// when
		let price_key = "eth_price".as_bytes().to_vec(); // PriceKey::PriceKeyIsETH ;
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			7777,
			PriceKey::create_on_vec(price_key.clone()),
			4,
			Default::default(),
			2, 0, 1);
		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		// (price_val, accountid, block_num, fraction_num)
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
				// (7777, Default::default(), 1, 4, Default::default())
				AresPriceData {
					price: 7777,
					account_id: AccountId::from_raw([0;32]),
					create_bn: 1,
					fraction_len: 4,
					raw_number: Default::default(),
					timestamp: 0,
					update_bn: 1
				}
			]),
			btc_price_list
		);

		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone()));
		assert_eq!(bet_avg_price, None, "Price pool is not full.");

		AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 6666, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);
		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		assert_eq!(0, btc_price_list.len());

		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone()));
		assert_eq!(bet_avg_price, Some(((7777 + 6666) / 2, 4, System::block_number())));

		//
		AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 1111, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);
		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
				// (1111, Default::default(), 1, 4, Default::default())
				AresPriceData {
					price: 1111,
					account_id: AccountId::from_raw([0;32]),
					create_bn: 1,
					fraction_len: 4,
					raw_number: Default::default(),
					timestamp: 0,
					update_bn: 1,
				}
			]),
			btc_price_list
		);

		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone()));
		assert_eq!(bet_avg_price, Some(((7777 + 6666) / 2, 4, System::block_number())));
	});
}

#[test]
fn test_addprice_of_exceeded_the_maximum_interval() {
	let (offchain, _state) = testing::TestOffchainExt::new();
	// let mut t = sp_io::TestExternalities::default();
	let mut t = new_test_ext();
	t.register_extension(OffchainWorkerExt::new(offchain));

	t.execute_with(|| {
		// The request key must be configured, otherwise you cannot submit the price. so you need =>
		// new_test_ext()

		System::set_block_number(1);
		// when
		// let price_key = "btc_price".as_bytes().to_vec(); // PriceKey::PriceKeyIsBTC ;
		let price_key = to_test_vec("btc_price");
		AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 8888, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);

		let pool = AresPrice::<Test>::get(PriceKey::create_on_vec(price_key.clone()));
		assert!(pool.is_some());
		assert_eq!(pool.unwrap().len(), 1);

		System::set_block_number(102);
		AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 9999, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);
		let pool = AresPrice::<Test>::get(PriceKey::create_on_vec(price_key.clone()));
		assert!(pool.is_some());
		assert_eq!(pool.unwrap().len(), 1);

		//
		// println!("------1 :{:?}", AresOcw::ares_prices(PriceKey::create_on_vec(price_key.clone())));
		// AresPrice::<Test>::iter_keys().into_iter().any(|x|{
		// 	println!("------2 {:?}", x);
		// 	false
		// });
		//
		// // Check price poll depth.
		// assert!(AresPrice::<Test>::contains_key(PriceKey::create_on_vec(price_key.clone()) ));
		// let pool = AresPrice::<Test>::get(PriceKey::create_on_vec(price_key.clone()));
		// println!("----- ===== {:?}", pool);




		// let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		// assert_eq!(
		// 	0,
		// 	btc_price_list.len(),
		// 	"Price list will be empty when the average calculation."
		// );
		//
		// let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone()));
		// assert_eq!( bet_avg_price, Some(((8888 + 9999) / 2, 4, System::block_number())), "Only test ares_avg_prices ");
		//
		// // Add a new value beyond the array boundary.
		// AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 3333, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);
		// let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		//
		// // (price_val, accountid, block_num, fraction_num)
		// assert_eq!(
		// 	AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
		// 		// (3333, Default::default(), 1, 4, Default::default())
		// 		AresPriceData {
		// 			price: 3333,
		// 			account_id: AccountId::from_raw([0;32]),
		// 			create_bn: 1,
		// 			fraction_len: 4,
		// 			raw_number: Default::default(),
		// 			timestamp: 0,
		// 			update_bn: 1
		// 		}
		// 	]),
		// 	btc_price_list
		// );
		//
		// let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec()).clone());
		// assert_eq!(bet_avg_price, Some(((8888 + 9999) / 2, 4, System::block_number())));
		//
		// // when
		// let price_key = "eth_price".as_bytes().to_vec(); // PriceKey::PriceKeyIsETH ;
		// AresOcw::add_price_and_try_to_agg(
		// 	AccountId::from_raw([0;32]),
		// 	7777,
		// 	PriceKey::create_on_vec(price_key.clone()),
		// 	4,
		// 	Default::default(),
		// 	2, 0, 1);
		// let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		// // (price_val, accountid, block_num, fraction_num)
		// assert_eq!(
		// 	AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
		// 		// (7777, Default::default(), 1, 4, Default::default())
		// 		AresPriceData {
		// 			price: 7777,
		// 			account_id: AccountId::from_raw([0;32]),
		// 			create_bn: 1,
		// 			fraction_len: 4,
		// 			raw_number: Default::default(),
		// 			timestamp: 0,
		// 			update_bn: 1
		// 		}
		// 	]),
		// 	btc_price_list
		// );
		//
		// let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone()));
		// assert_eq!(bet_avg_price, None, "Price pool is not full.");
		//
		// AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 6666, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);
		// let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		// assert_eq!(0, btc_price_list.len());
		//
		// let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone()));
		// assert_eq!(bet_avg_price, Some(((7777 + 6666) / 2, 4, System::block_number())));
		//
		// //
		// AresOcw::add_price_and_try_to_agg(AccountId::from_raw([0;32]), 1111, PriceKey::create_on_vec(price_key.clone()), 4, Default::default(), 2, 0, 1);
		// let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		// assert_eq!(
		// 	AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
		// 		// (1111, Default::default(), 1, 4, Default::default())
		// 		AresPriceData {
		// 			price: 1111,
		// 			account_id: AccountId::from_raw([0;32]),
		// 			create_bn: 1,
		// 			fraction_len: 4,
		// 			raw_number: Default::default(),
		// 			timestamp: 0,
		// 			update_bn: 1,
		// 		}
		// 	]),
		// 	btc_price_list
		// );
		//
		// let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("eth_price".as_bytes().to_vec().clone()));
		// assert_eq!(bet_avg_price, Some(((7777 + 6666) / 2, 4, System::block_number())));
	});
}

#[test]
fn test_abnormal_price_despose() {
	let (offchain, _state) = testing::TestOffchainExt::new();
	// let mut t = sp_io::TestExternalities::default();
	let mut t = new_test_ext();
	t.register_extension(OffchainWorkerExt::new(offchain));

	t.execute_with(|| {
		let BN: u64 = 2;
		System::set_block_number(BN);

		// In the genesis config default pool depth is 3.
		assert_eq!(3, AresOcw::get_price_pool_depth());

		let price_key = PriceKey::create_on_vec("btc_price".as_bytes().to_vec());

		// Test normal price list, Round 1
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			1000,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			1010,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(2 as usize, AresOcw::ares_prices(price_key.clone()).unwrap_or(Default::default()).len());
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			1020,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(
			0 as usize,
			AresOcw::ares_prices(price_key.clone()).unwrap_or(Default::default()).len(),
			"The price pool is cleared after full."
		);
		let bet_avg_price = AresOcw::ares_avg_prices(price_key.clone());
		assert_eq!(bet_avg_price, Some((1010, 4, System::block_number())));

		// Test normal price list, Round 2
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			1030,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(1 as usize, AresOcw::ares_prices(price_key.clone()).unwrap_or(Default::default()).len());
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			1040,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(2 as usize, AresOcw::ares_prices(price_key.clone()).unwrap_or(Default::default()).len());
		let bet_avg_price = AresOcw::ares_avg_prices(price_key.clone());
		assert_eq!(
			bet_avg_price,
			Some((1010, 4, System::block_number())),
			"If the price pool not full, the average price is old value."
		);
		// Add a new one price pool is full, the average price will be update.
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			1010,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		// Check price pool.
		assert_eq!(
			0 as usize,
			AresOcw::ares_prices(price_key.clone()).unwrap_or(Default::default()).len(),
			"The price pool is cleared after full."
		);
		let bet_avg_price = AresOcw::ares_avg_prices(price_key.clone());
		assert_eq!(bet_avg_price, Some((1030, 4, System::block_number())));

		// Test abnormal price list, Round 3
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			1020,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(1 as usize, AresOcw::ares_prices(price_key.clone()).unwrap_or(Default::default()).len());
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			1030,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(2 as usize, AresOcw::ares_prices(price_key.clone()).unwrap_or(Default::default()).len());
		let bet_avg_price = AresOcw::ares_avg_prices(price_key.clone());
		assert_eq!(
			bet_avg_price,
			Some((1030, 4, System::block_number())),
			"If the price pool not full, the average price is old value."
		);
		// Add a new one price pool is full, the average price will be update.
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			2000,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		// Check price pool.
		assert_eq!(
			0 as usize,
			AresOcw::ares_prices(price_key.clone()).unwrap_or(Default::default()).len(),
			"The price pool is cleared after full."
		);
		let bet_avg_price = AresOcw::ares_avg_prices(price_key.clone());
		assert_eq!(bet_avg_price, Some(((1030 + 1020) / 2, 4, System::block_number())));

		// Check abnormal price list.
		assert_eq!(1 as usize, AresOcw::ares_abnormal_prices(price_key.clone()).unwrap_or(Default::default()).len());
		// price, account, bolcknumber, FractionLength, JsonNumberValue
		// (types::AresPriceData<sp_application_crypto::sr25519::Public, u64>, types::AvgPriceData)>
		// (types::AresPriceData<ares_oracle_provider_support::crypto::sr25519::app_sr25519::Public, u64>,
		// types::AvgPriceData)` <sp_application_crypto::Vec<(types::AresPriceData<sp_application_crypto::
		// sr25519::Public, u64>, types::AvgPriceData)>>` for `sp_application_crypto::Vec<(types::
		// AresPriceData<ares_oracle_provider_support::crypto::sr25519::app_sr25519::Public, u64>,
		// types::AvgPriceData)>`
		assert_eq!(
			AbnormalPriceDataVec::<AccountId, BlockNumber>::create_on_vec(vec![(
				ares_price_data_from_tuple((2000, AccountId::from_raw([0;32]), 1, 4, Default::default(), 0, BN)),
				AvgPriceData {
					integer: 1030,
					fraction_len: 4
				}
			)]) ,
			AresOcw::ares_abnormal_prices(price_key.clone()).unwrap()
		);
	});
}

#[test]
fn test_get_price_pool_depth() {
	let (offchain, _state) = testing::TestOffchainExt::new();
	// let mut t = sp_io::TestExternalities::default();
	let mut t = new_test_ext();
	t.register_extension(OffchainWorkerExt::new(offchain));

	t.execute_with(|| {
		// let BN:u64 = 2;
		// System::set_block_number(BN);
		println!("RUN AA0 ");
		// In the genesis config default pool depth is 3.
		assert_eq!(3, AresOcw::get_price_pool_depth());
		println!("RUN AA1 ");
		// Test input some price
		let price_key = PriceKey::create_on_vec("btc_price".as_bytes().to_vec());
		println!("RUN AA2 ");
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			6660,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		println!("RUN AA3 ");
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			8880,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		println!("RUN AA4 ");
		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone()));
		println!("RUN AA5 ");
		assert_eq!(bet_avg_price, None);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			7770,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		println!("RUN AA6 ");
		assert_eq!(
			0 as usize,
			AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default()).len()
		);
		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone()));
		// Why price is 7770 , Because average value is 7770 :
		// and (7770 - 6660) * 100 / 7770 = 14 , pick out 6660
		// and (8880 - 7770) * 100 / 7770 = 14 , pick out 8880
		// and (7770 - 7770) * 100 / 7770 = 0
		assert_eq!(bet_avg_price, Some((7770, 4, System::block_number())));
		assert_eq!(
			2 as usize,
			AresOcw::ares_abnormal_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec())).unwrap_or(Default::default()).len()
		);

		// Update depth to 5
		// if parse version change
		assert_ok!(AresOcw::update_pool_depth_propose(Origin::root(), 5));
		assert_eq!(5, AresOcw::get_price_pool_depth());
		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone()));
		assert_eq!(bet_avg_price, Some((7770, 4, System::block_number())), " Pool expansion, but average has not effect.");

		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			5550,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			6660,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(
			2 as usize,
			AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default()).len()
		);
		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone()));
		assert_eq!(bet_avg_price, Some((7770, 4, System::block_number())), "Old value yet.");

		// fill price list.
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			5350,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			5500,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			5400,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(
			0 as usize,
			AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default()).len()
		);
		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone()));
		assert_eq!(bet_avg_price, Some(((5550 + 5350 + 5500 + 5400) / 4, 4, System::block_number())), "Average update.");

		// Fall back depth to 3
		assert_ok!(AresOcw::update_pool_depth_propose(Origin::root(), 3));
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			4440,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(3, AresOcw::get_price_pool_depth());
		assert_eq!(
			1 as usize,
			AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default()).len()
		);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			4430,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			4420,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(
			0 as usize,
			AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default()).len()
		);
		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone()));
		assert_eq!(bet_avg_price, Some((4430, 4, System::block_number())));
		//
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			4440,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			4440,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		assert_eq!(
			2 as usize,
			AresOcw::ares_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone())).unwrap_or(Default::default()).len(),
			"Should 4440"
		);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			4340,
			price_key.clone(),
			4,
			Default::default(),
			AresOcw::get_price_pool_depth(),
			0,
			1,
		);
		let bet_avg_price = AresOcw::ares_avg_prices(PriceKey::create_on_vec("btc_price".as_bytes().to_vec().clone()));
		assert_eq!(bet_avg_price, Some((4440, 4, System::block_number())));
	});
}

#[test]
fn test_json_number_value_to_price() {
	// number =
	let number1 = JsonNumberValue {
		integer: 8,
		fraction: 87654,
		fraction_length: 5,
		exponent: 0,
	};
	assert_eq!(8876540, number1.to_price(6));
	assert_eq!(887654, number1.to_price(5));
	assert_eq!(88765, number1.to_price(4));
	assert_eq!(8876, number1.to_price(3));
	assert_eq!(887, number1.to_price(2));
	assert_eq!(88, number1.to_price(1));
	assert_eq!(8, number1.to_price(0));

	let number3 = JsonNumberValue {
		integer: 6,
		fraction: 654,
		fraction_length: 3,
		exponent: 0,
	};
	assert_eq!(6654000, number3.to_price(6));
	assert_eq!(665400, number3.to_price(5));
	assert_eq!(66540, number3.to_price(4));
	assert_eq!(6654, number3.to_price(3));
	assert_eq!(665, number3.to_price(2));
	assert_eq!(66, number3.to_price(1));
	assert_eq!(6, number3.to_price(0));


	let number4 = JsonNumberValue {
		integer: 8,
		fraction: 934,
		fraction_length: 3,
		exponent: -7,
	};
	// 00000008934
	// 00000008934
	assert_eq!(00000008934, number4.to_price(10));
	assert_eq!(0000000893, number4.to_price(9));
	assert_eq!(000000089, number4.to_price(8));
	assert_eq!(00000008, number4.to_price(7));
	assert_eq!(0000000, number4.to_price(6));
	assert_eq!(000000, number4.to_price(5));
	assert_eq!(00000, number4.to_price(4));
}

#[test]
fn test_request_price_update_then_the_price_list_will_be_update_if_the_fractioin_length_changed() {
	let (offchain, _state) = testing::TestOffchainExt::new();
	// let mut t = sp_io::TestExternalities::default();
	let mut t = new_test_ext();
	t.register_extension(OffchainWorkerExt::new(offchain));

	t.execute_with(|| {
		let BN: u64 = 2;
		System::set_block_number(BN);

		// number =
		let number1 = JsonNumberValue {
			integer: 8,
			fraction: 87654,
			fraction_length: 5,
			exponent: 0,
		};
		let number2 = JsonNumberValue {
			integer: 8,
			fraction: 76543,
			fraction_length: 5,
			exponent: 0,
		};
		let number3 = JsonNumberValue {
			integer: 8,
			fraction: 654,
			fraction_length: 3,
			exponent: 0,
		};

		// when
		let price_key = PriceKey::create_on_vec("btc_price".as_bytes().to_vec()); // PriceKey::PriceKeyIsBTC ;
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			number1.to_price(4),
			price_key.clone(),
			4,
			number1.clone(),
			4,
			0,
			1,
		);
		let btc_price_list = AresOcw::ares_prices(price_key.clone()).unwrap();
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![ares_price_data_from_tuple((
				number1.to_price(4),
				AccountId::from_raw([0;32]),
				1,
				4,
				number1.clone(),
				0,
				BN,
			))]),
			btc_price_list
		);
		let bet_avg_price = AresOcw::ares_avg_prices(price_key.clone());
		assert_eq!(bet_avg_price, None);

		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			number2.to_price(4),
			price_key.clone(),
			4,
			number2.clone(),
			4,
			0,
			1,
		);
		let btc_price_list = AresOcw::ares_prices(price_key.clone()).unwrap();
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
				ares_price_data_from_tuple((number1.to_price(4), AccountId::from_raw([0;32]), 1, 4, number1.clone(), 0, BN,)),
				ares_price_data_from_tuple((number2.to_price(4), AccountId::from_raw([0;32]), 1, 4, number2.clone(), 0, BN,))
			]),
			btc_price_list
		);

		// When fraction length change, list will be update new fraction.
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			number3.to_price(3),
			price_key.clone(),
			3,
			number3.clone(),
			4,
			0,
			1,
		);
		let btc_price_list = AresOcw::ares_prices(price_key.clone()).unwrap();
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
				ares_price_data_from_tuple((number1.to_price(3), AccountId::from_raw([0;32]), 1, 3, number1.clone(), 0, BN,)),
				ares_price_data_from_tuple((number2.to_price(3), AccountId::from_raw([0;32]), 1, 3, number2.clone(), 0, BN,)),
				ares_price_data_from_tuple((number3.to_price(3), AccountId::from_raw([0;32]), 1, 3, number3.clone(), 0, BN,)),
			]),
			btc_price_list
		);

		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0;32]),
			number1.to_price(5),
			price_key.clone(),
			5,
			number1.clone(),
			4,
			0,
			1,
		);
		let abnormal_vec = AresOcw::ares_abnormal_prices(price_key.clone()).unwrap_or(Default::default());
		assert_eq!(0, abnormal_vec.len());

		let bet_avg_price = AresOcw::ares_avg_prices(price_key.clone());
		// assert_eq!(bet_avg_price, (0, 0));
		assert_eq!(bet_avg_price, Some(((number1.to_price(5) + number2.to_price(5)) / 2, 5, System::block_number())));

		// let bet_avg_price = AresOcw::ares_avg_prices("btc_price".as_bytes().to_vec().clone());
		// assert_eq!(bet_avg_price, ((number1.to_price(4) + number2.to_price(4)) / 2, 4));
		//
		// // then fraction changed, old price list will be update to new fraction length.
		// AresOcw::add_price_and_try_to_agg(Default::default(), number3.to_price(3), price_key.clone(), 3,
		// number3.clone(), 2); let btc_price_list =
		// AresOcw::ares_prices("btc_price".as_bytes().to_vec().clone()); assert_eq!(vec![
		// (number2.to_price(3),Default::default(), BN, 3, number2.clone()),
		// (number3.to_price(3), Default::default(), BN, 3, number3.clone())                 ],
		// btc_price_list); let bet_avg_price =
		// AresOcw::ares_avg_prices("btc_price".as_bytes().to_vec().clone()); // assert_eq!
		// (bet_avg_price, (0, 0)); assert_eq!(bet_avg_price, ((
		//                             number2.to_price(3) +
		//                             number3.to_price(3)
		//                            ) / 2, 3));
	});
}

#[test]
fn bulk_parse_price_ares_works() {
	let FRACTION_NUM_2: u32 = 2;
	let FRACTION_NUM_3: u32 = 3;
	let FRACTION_NUM_4: u32 = 4;
	let FRACTION_NUM_5: u32 = 5;
	let FRACTION_NUM_6: u32 = 6;

	let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
	let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));
	let price_key_dot_price = PriceKey::create_on_vec(to_test_vec("dot_price"));
	let price_key_xrp_price = PriceKey::create_on_vec(to_test_vec("xrp_price"));
	let price_key_xxx_price = PriceKey::create_on_vec(to_test_vec("xxx_price"));

	let price_token_btc = PriceKey::create_on_vec(to_test_vec("btc"));
	let price_token_eth = PriceKey::create_on_vec(to_test_vec("eth"));
	let price_token_dot = PriceKey::create_on_vec(to_test_vec("dot"));
	let price_token_xrp = PriceKey::create_on_vec(to_test_vec("xrp"));
	let price_token_xxx = PriceKey::create_on_vec(to_test_vec("xxx"));

	// Bulk parse
	// defined parse format
	let mut format = RawSourceKeys::default();
	format.try_push((price_key_btc_price.clone(), price_token_btc.clone(), FRACTION_NUM_2));
	format.try_push((price_key_eth_price.clone(), price_token_eth.clone(), FRACTION_NUM_3));
	format.try_push((price_key_dot_price.clone(), price_token_dot.clone(), FRACTION_NUM_4));
	format.try_push((price_key_xrp_price.clone(), price_token_xrp.clone(), FRACTION_NUM_5));
	// xxx_price not exist, so what up ?
	format.try_push((price_key_xxx_price.clone(), price_token_xxx.clone(), FRACTION_NUM_6));

	let result_bulk_parse = AresOcw::bulk_parse_price_of_ares(get_are_json_of_bulk(), to_test_vec("usdt"), format);

	let test_number_value = NumberValue {
		integer: 0,
		fraction: 0,
		fraction_length: 0,
		exponent: 0,
	};

	let mut bulk_expected = Vec::new();
	bulk_expected.push((
		price_key_btc_price.clone(),
		Some(5026137),
		FRACTION_NUM_2,
		NumberValue {
			integer: 50261,
			fraction: 372,
			fraction_length: 3,
			exponent: 0,
		},
		1629699168,
	));
	bulk_expected.push((
		price_key_eth_price.clone(),
		Some(3107710),
		FRACTION_NUM_3,
		NumberValue {
			integer: 3107,
			fraction: 71,
			fraction_length: 2,
			exponent: 0,
		},
		1630055777,
	));
	bulk_expected.push((
		price_key_dot_price.clone(),
		Some(359921),
		FRACTION_NUM_4,
		NumberValue {
			integer: 35,
			fraction: 9921,
			fraction_length: 4,
			exponent: 0,
		},
		1631497660,
	));
	bulk_expected.push((
		price_key_xrp_price.clone(),
		Some(109272),
		FRACTION_NUM_5,
		NumberValue {
			integer: 1,
			fraction: 9272,
			fraction_length: 5,
			exponent: 0,
		},
		1631497987,
	));

	assert_eq!(result_bulk_parse, bulk_expected);

	// The above looks normal. Next, test the return value of 0
	// Bulk parse
	// defined parse format
	let mut format = RawSourceKeys::default();
	format.try_push((price_key_btc_price.clone(), price_token_btc.clone(), FRACTION_NUM_2));
	format.try_push((price_key_eth_price.clone(), price_token_eth.clone(), FRACTION_NUM_3));
	format.try_push((price_key_dot_price.clone(), price_token_dot.clone(), FRACTION_NUM_4));
	format.try_push((price_key_xrp_price.clone(), price_token_xrp.clone(), FRACTION_NUM_5));
	// xxx_price not exist, so what up ?
	format.try_push((price_key_xxx_price.clone(), price_token_xxx.clone(), FRACTION_NUM_6));

	let result_bulk_parse =
		AresOcw::bulk_parse_price_of_ares(get_are_json_of_bulk_of_xxxusdt_is_0(), to_test_vec("usdt"), format);

	let mut bulk_expected = Vec::new();
	bulk_expected.push((
		price_key_btc_price,
		Some(5026137),
		FRACTION_NUM_2,
		NumberValue {
			integer: 50261,
			fraction: 372,
			fraction_length: 3,
			exponent: 0,
		},
		1629699168,
	));
	bulk_expected.push((
		price_key_eth_price,
		Some(3107710),
		FRACTION_NUM_3,
		NumberValue {
			integer: 3107,
			fraction: 71,
			fraction_length: 2,
			exponent: 0,
		},
		1630055777,
	));
	bulk_expected.push((
		price_key_dot_price,
		Some(359921),
		FRACTION_NUM_4,
		NumberValue {
			integer: 35,
			fraction: 9921,
			fraction_length: 4,
			exponent: 0,
		},
		1631497660,
	));
	bulk_expected.push((
		price_key_xrp_price,
		Some(109272),
		FRACTION_NUM_5,
		NumberValue {
			integer: 1,
			fraction: 9272,
			fraction_length: 5,
			exponent: 0,
		},
		1631497987,
	));

	assert_eq!(result_bulk_parse, bulk_expected);
}

// This test will be discarded.
// #[test]
// fn parse_price_ares_works() {
// 	let test_data = vec![
// 		(get_are_json_of_btc(), Some(50261)),
// 		(get_are_json_of_eth(), Some(3107)),
// 		(get_are_json_of_dot(), Some(35)),
// 		(get_are_json_of_xrp(), Some(1)),
// 	];
//
// 	for (json, expected) in test_data {
// 		let second = AresOcw::parse_price_of_ares(json, 0);
// 		assert_eq!(expected, second);
// 	}
//
// 	let test_data = vec![
// 		(get_are_json_of_btc(), Some(5026137)),
// 		(get_are_json_of_eth(), Some(310771)),
// 		(get_are_json_of_dot(), Some(3599)),
// 		(get_are_json_of_xrp(), Some(109)),
// 	];
//
// 	for (json, expected) in test_data {
// 		let second = AresOcw::parse_price_of_ares(json, 2);
// 		assert_eq!(expected, second);
// 	}
//
// 	let test_data = vec![
// 		(get_are_json_of_btc(), Some(50261372)),
// 		(get_are_json_of_eth(), Some(3107710)),
// 		(get_are_json_of_dot(), Some(35992)),
// 		(get_are_json_of_xrp(), Some(1092)),
// 	];
//
// 	for (json, expected) in test_data {
// 		let second = AresOcw::parse_price_of_ares(json, 3);
// 		assert_eq!(expected, second);
// 	}
//
// 	let test_data = vec![
// 		(get_are_json_of_btc(), Some(50261372000)),
// 		(get_are_json_of_eth(), Some(3107710000)),
// 		(get_are_json_of_dot(), Some(35992100)),
// 		(get_are_json_of_xrp(), Some(1092720)),
// 	];
//
// 	for (json, expected) in test_data {
// 		let second = AresOcw::parse_price_of_ares(json, 6);
// 		assert_eq!(expected, second);
// 	}
// }

#[test]
fn test_get_raw_price_source_list() {
	let mut t = new_test_ext();
	let (offchain, state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		let raw_price_source_list = AresOcw::get_raw_price_source_list();
		println!("raw_price_source_list = {:?}", raw_price_source_list);
	});
}


#[test]
fn test_make_bulk_price_format_data() {
	let mut t = new_test_ext();

	let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
	let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));
	let price_key_dot_price = PriceKey::create_on_vec(to_test_vec("dot_price"));
	let price_key_xrp_price = PriceKey::create_on_vec(to_test_vec("xrp_price"));

	let price_token_btc = PriceKey::create_on_vec(to_test_vec("btc"));
	let price_token_eth = PriceKey::create_on_vec(to_test_vec("eth"));
	let price_token_dot = PriceKey::create_on_vec(to_test_vec("dot"));
	let price_token_xrp = PriceKey::create_on_vec(to_test_vec("xrp"));

	let (offchain, state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));

	let mut expect_format = Vec::new();
	expect_format.push((price_key_btc_price.clone(), price_token_btc.clone(), 4, 1));

	t.execute_with(|| {
		let price_format = AresOcw::make_bulk_price_format_data(1);
		assert_eq!(expect_format, price_format);
	});

	// When block number is 2
	let mut expect_format = Vec::new();
	expect_format.push((price_key_btc_price.clone(), price_token_btc.clone(), 4, 1));
	expect_format.push((price_key_eth_price.clone(), price_token_eth.clone(), 4, 2));
	t.execute_with(|| {
		let price_format = AresOcw::make_bulk_price_format_data(2);
		assert_eq!(expect_format, price_format);
	});

	// When block number is 3
	let mut expect_format = Vec::new();
	expect_format.push((price_key_btc_price.clone(), price_token_btc.clone(), 4, 1));
	expect_format.push((price_key_dot_price.clone(), price_token_dot.clone(), 4, 3));
	t.execute_with(|| {
		let price_format = AresOcw::make_bulk_price_format_data(3);
		assert_eq!(expect_format, price_format);
	});

	// When block number is 4
	let mut expect_format = Vec::new();
	expect_format.push((price_key_btc_price.clone(), price_token_btc.clone(), 4, 1));
	expect_format.push((price_key_eth_price.clone(), price_token_eth.clone(), 4, 2));
	expect_format.push((price_key_xrp_price.clone(), price_token_xrp.clone(), 4, 4));
	t.execute_with(|| {
		let price_format = AresOcw::make_bulk_price_format_data(4);
		assert_eq!(expect_format, price_format);
	});

	// When block number is 5
	let mut expect_format = Vec::new();
	expect_format.push((price_key_btc_price.clone(), price_token_btc.clone(), 4, 1));
	t.execute_with(|| {
		let price_format = AresOcw::make_bulk_price_format_data(1);
		assert_eq!(expect_format, price_format);
	});
}

// This BUG will cause the AURA authority block producer to fix pricer all the time.
#[test]
fn fix_bug_make_bulk_price_format_data() {
	let mut t = new_test_ext();

	let (offchain, state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));

	// Exists
	// btcusdt -> 1u8 , 1,2,3,4
	// ethusdt -> 2u8 , 2,4,
	// dotusdt -> 3u8 , 3,
	// xrpusdt -> 4u8 , 4
	t.execute_with(|| {});
}

#[test]
fn make_bulk_price_request_url() {
	let mut t = new_test_ext();
	t.execute_with(|| {

		let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
		let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));
		let price_key_dot_price = PriceKey::create_on_vec(to_test_vec("dot_price"));
		let price_key_xrp_price = PriceKey::create_on_vec(to_test_vec("xrp_price"));

		let price_token_btc = PriceKey::create_on_vec(to_test_vec("btc"));
		let price_token_eth = PriceKey::create_on_vec(to_test_vec("eth"));
		let price_token_dot = PriceKey::create_on_vec(to_test_vec("dot"));
		let price_token_xrp = PriceKey::create_on_vec(to_test_vec("xrp"));

		let mut expect_format = RawSourceKeys::default();
		expect_format.try_push((price_key_btc_price.clone(), price_token_btc.clone(), 4));
		expect_format.try_push((price_key_eth_price.clone(), price_token_eth.clone(), 4));

		let bulk_request = AresOcw::make_bulk_price_request_url(expect_format);
		// assert_eq!("http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt".as_bytes().to_vec(), bulk_request);
		assert_eq!(
			(
				"http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc_eth"
					.as_bytes()
					.to_vec(),
				to_test_vec("usdt")
			),
			bulk_request
		);

		let mut expect_format = RawSourceKeys::default();
		expect_format.try_push((price_key_btc_price.clone(), price_token_btc.clone(), 4));
		expect_format.try_push((price_key_eth_price.clone(), price_token_eth.clone(), 4));
		expect_format.try_push((price_key_btc_price.clone(), price_token_dot.clone(), 4));
		expect_format.try_push((price_key_eth_price.clone(), price_token_xrp.clone(), 4));

		let bulk_request = AresOcw::make_bulk_price_request_url(expect_format);
		// assert_eq!("http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".as_bytes().to_vec(), bulk_request);
		assert_eq!(
			(
				"http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc_eth_dot_xrp"
					.as_bytes()
					.to_vec(),
				to_test_vec("usdt")
			),
			bulk_request
		);
	});
}

#[test]
fn save_fetch_ares_price_and_send_payload_signed() {
	let mut t = new_test_ext();

	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	let keystore = KeyStore::new();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();

	let public_key = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(0)
		.unwrap()
		.clone();

	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	let price_payload_b1 = PricePayload {
		block_number: 1, // type is BlockNumber
		jump_block: Default::default(),
		auth: public_key.clone().into(),
		price: PricePayloadSubPriceList::create_on_vec(vec![
			PricePayloadSubPrice(
				PriceKey::create_on_vec("btc_price".as_bytes().to_vec()),
				502613720u64,
				4,
				JsonNumberValue {
					integer: 50261,
					fraction: 372,
					fraction_length: 3,
					exponent: 0,
				},
				1629699168,
			),
		]),
		public: <Test as SigningTypes>::Public::from(public_key),
	};

	t.execute_with(|| {
		AresOcw::save_fetch_ares_price_and_send_payload_signed(1, public_key.into()).unwrap();
		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		if let Call::AresOcw(crate::Call::submit_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) = tx.call {
			// println!("signature = {:?}", signature);
			assert_eq!(body.clone(), price_payload_b1);
			let signature_valid = <PricePayload<
				<Test as SigningTypes>::Public,
				<Test as frame_system::Config>::BlockNumber,
				AuraId,
			> as SignedPayload<Test>>::verify::<crate::ares_crypto::AresCrypto<AuraId>>(
				&price_payload_b1, signature.clone()
			);
			assert!(signature_valid);
		}
	});
}

// handleCalculateJumpBlock((interval,jump_block))
// (2,0)=>(2,1)	(3,0)=>(3,2)	(4,0)=>(4,3)
// (2,1)=>(2,0)	(3,2)=>(3,1)	(4,3)=>(4,2)
//              (3,1)=>(3,0)	(4,2)=>(4,1)
//                              (4,1)=>(4,0)
#[test]
fn test_handle_calculate_jump_block() {
	let mut t = new_test_ext();
	let (offchain, state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {
		assert_eq!((1, 0), AresOcw::handle_calculate_jump_block((1, 0)));

		assert_eq!((2, 1), AresOcw::handle_calculate_jump_block((2, 0)));
		assert_eq!((2, 0), AresOcw::handle_calculate_jump_block((2, 1)));

		assert_eq!((3, 2), AresOcw::handle_calculate_jump_block((3, 0)));
		assert_eq!((3, 1), AresOcw::handle_calculate_jump_block((3, 2)));
		assert_eq!((3, 0), AresOcw::handle_calculate_jump_block((3, 1)));

		assert_eq!((4, 3), AresOcw::handle_calculate_jump_block((4, 0)));
		assert_eq!((4, 2), AresOcw::handle_calculate_jump_block((4, 3)));
		assert_eq!((4, 1), AresOcw::handle_calculate_jump_block((4, 2)));
		assert_eq!((4, 0), AresOcw::handle_calculate_jump_block((4, 1)));

		assert_eq!((5, 4), AresOcw::handle_calculate_jump_block((5, 0)));
		assert_eq!((5, 3), AresOcw::handle_calculate_jump_block((5, 4)));
		assert_eq!((5, 2), AresOcw::handle_calculate_jump_block((5, 3)));
		assert_eq!((5, 1), AresOcw::handle_calculate_jump_block((5, 2)));
		assert_eq!((5, 0), AresOcw::handle_calculate_jump_block((5, 1)));
	});
}

#[test]
fn test_increase_jump_block_number() {
	let mut t = new_test_ext();
	let (offchain, state) = testing::TestOffchainExt::new();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.execute_with(|| {

		let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
		let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));
		let price_key_dot_price = PriceKey::create_on_vec(to_test_vec("dot_price"));
		let price_key_xrp_price = PriceKey::create_on_vec(to_test_vec("xrp_price"));

		let price_token_btc = PriceKey::create_on_vec(to_test_vec("btc"));
		let price_token_eth = PriceKey::create_on_vec(to_test_vec("eth"));
		let price_token_dot = PriceKey::create_on_vec(to_test_vec("dot"));
		let price_token_xrp = PriceKey::create_on_vec(to_test_vec("xrp"));

		assert_eq!(
			0,
			AresOcw::get_jump_block_number(price_key_btc_price.clone()),
			"The default value of jump block number is 0."
		);
		assert_eq!(
			0,
			AresOcw::get_jump_block_number(price_key_eth_price.clone()),
			"The default value of jump block number is 0."
		);

		let mut expect_format = Vec::new();
		expect_format.push((price_key_btc_price.clone(), price_token_btc.clone(), 4, 1)); // (1,0)
		expect_format.push((price_key_eth_price.clone(), price_token_eth.clone(), 4, 2)); // (2,0)
		let price_format = AresOcw::make_bulk_price_format_data(2);
		assert_eq!(expect_format, price_format);

		// Increase jump block number, PARAM:: price_key, interval
		assert_eq!((1, 0), AresOcw::increase_jump_block_number(price_key_btc_price.clone(), 1));
		assert_eq!((2, 1), AresOcw::increase_jump_block_number(price_key_eth_price.clone(), 2));

		assert_eq!(0, AresOcw::get_jump_block_number(price_key_btc_price.clone()));
		assert_eq!(1, AresOcw::get_jump_block_number(price_key_eth_price.clone()));

		let mut expect_format = Vec::new();
		expect_format.push((price_key_btc_price.clone(), price_token_btc.clone(), 4, 1)); // (1,0)
		let price_format = AresOcw::make_bulk_price_format_data(2);
		assert_eq!(expect_format, price_format);

		let mut expect_format = Vec::new();
		expect_format.push((price_key_btc_price.clone(), price_token_btc.clone(), 4, 1));
		expect_format.push((price_key_eth_price.clone(), price_token_eth.clone(), 4, 2)); // (2,0)
		expect_format.push((price_key_dot_price.clone(), price_token_dot.clone(), 4, 3));
		let price_format = AresOcw::make_bulk_price_format_data(3);
		assert_eq!(expect_format, price_format);
	});
}

// updateLastPriceListForAuthor(price_key_list,account_id)
#[test]
fn test_update_last_price_list_for_author() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		init_aura_enging_digest();

		let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
		let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));
		let price_key_dot_price = PriceKey::create_on_vec(to_test_vec("dot_price"));
		let price_key_xrp_price = PriceKey::create_on_vec(to_test_vec("xrp_price"));

		let price_token_btc = PriceKey::create_on_vec(to_test_vec("btc"));
		let price_token_eth = PriceKey::create_on_vec(to_test_vec("eth"));
		let price_token_dot = PriceKey::create_on_vec(to_test_vec("dot"));
		let price_token_xrp = PriceKey::create_on_vec(to_test_vec("xrp"));

		let (stash_1, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();
		let (stash_2, authority_2) = <Authorities<Test>>::get().unwrap()[1].clone();

		assert_eq!(None, AresOcw::get_last_price_author(price_key_btc_price.clone()));
		assert_eq!(None, AresOcw::get_last_price_author(price_key_eth_price.clone()));
		assert_eq!(None, AresOcw::get_last_price_author(price_key_dot_price.clone()));

		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_btc_price.clone(), stash_1.clone()),
			false
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_eth_price.clone(), stash_1.clone()),
			false
		);

		AresOcw::update_last_price_list_for_author(
			vec![price_key_btc_price.clone(), price_key_eth_price.clone(),],
			stash_1.clone(),
			2
		);

		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_btc_price.clone(), stash_1.clone()),
			true
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_eth_price.clone(), stash_1.clone()),
			true
		);

		assert_eq!(
			Some((stash_1.clone(), 2)),
			AresOcw::get_last_price_author(price_key_btc_price.clone())
		);
		assert_eq!(
			Some((stash_1.clone(),2)),
			AresOcw::get_last_price_author(price_key_eth_price.clone())
		);

		assert_eq!(None, AresOcw::get_last_price_author(price_key_dot_price.clone()));
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_dot_price.clone(), stash_1.clone()),
			false
		);

		AresOcw::update_last_price_list_for_author(
			vec![price_key_dot_price.clone(), price_key_eth_price.clone()],
			stash_2.clone(),
			4,
		);

		assert_eq!(
			Some((stash_1.clone(), 2)),
			AresOcw::get_last_price_author(price_key_btc_price.clone())
		);
		assert_eq!(
			Some((stash_2.clone(), 4)),
			AresOcw::get_last_price_author(price_key_eth_price.clone())
		);
		assert_eq!(
			Some((stash_2.clone(), 4)),
			AresOcw::get_last_price_author(price_key_dot_price.clone())
		);

		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_btc_price.clone(), stash_1.clone()),
			true
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_eth_price.clone(), stash_1.clone()),
			false
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_dot_price.clone(), stash_1.clone()),
			false
		);

		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_btc_price.clone(), stash_2.clone()),
			false
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_eth_price.clone(), stash_2.clone()),
			true
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_dot_price.clone(), stash_2.clone()),
			true
		);
	});
}

#[test]
fn test_jump_block_submit() {
	let mut t = new_test_ext();

	let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
	let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));
	let price_key_dot_price = PriceKey::create_on_vec(to_test_vec("dot_price"));
	let price_key_xrp_price = PriceKey::create_on_vec(to_test_vec("xrp_price"));

	let price_token_btc = PriceKey::create_on_vec(to_test_vec("btc"));
	let price_token_eth = PriceKey::create_on_vec(to_test_vec("eth"));
	let price_token_dot = PriceKey::create_on_vec(to_test_vec("dot"));
	let price_token_xrp = PriceKey::create_on_vec(to_test_vec("xrp"));


	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(0)
		.unwrap()
		.clone();

	let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(1)
		.unwrap()
		.clone();

	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc_eth".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		init_aura_enging_digest();

		let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
		let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));

		let (stash_1, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();
		let (stash_2, authority_2) = <Authorities<Test>>::get().unwrap()[1].clone();

		// assert_eq!(&authority_1.clone().encode(), &public_key_1.into_account().encode());
		// assert_eq!(&authority_2.clone().encode(), &public_key_2.into_account().encode());

		assert_eq!(AresOcw::get_last_price_author(price_key_btc_price.clone()), None);
		assert_eq!(AresOcw::get_last_price_author(price_key_eth_price.clone()), None);

		let stash_id = AresOcw::get_stash_id(&authority_1.clone());
		let mut trace_data = AuthorTraceData::<AccountId, BlockNumber>::default();
		trace_data.try_push((stash_id.unwrap(), 2));
		BlockAuthorTrace::<Test>::put(trace_data);

		AresOcw::save_fetch_ares_price_and_send_payload_signed(2, authority_1.clone()).unwrap();
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let Call::AresOcw(crate::Call::submit_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) = tx.call {
			AresOcw::submit_price_unsigned_with_signed_payload(Origin::none(), body, signature);
		}

		assert_eq!(BlockAuthorTrace::<Test>::get().unwrap().len(), 0usize);

		assert_eq!(
			AresOcw::get_last_price_author(price_key_btc_price.clone()),
			Some((stash_1.clone(),2))
		);
		assert_eq!(
			AresOcw::get_last_price_author(price_key_eth_price.clone()),
			Some((stash_1.clone(),2))
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_btc_price.clone(), stash_1.clone()),
			true
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_eth_price.clone(), stash_1.clone()),
			true
		);
	});

	// No http request.
	t.execute_with(|| {
		init_aura_enging_digest();

		let (stash_1, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();

		assert_eq!(AresOcw::get_jump_block_number(PriceKey::create_on_vec(to_test_vec("btc_price"))), 0);
		assert_eq!(AresOcw::get_jump_block_number(PriceKey::create_on_vec(to_test_vec("eth_price"))), 0);

		let stash_id = AresOcw::get_stash_id(&authority_1.clone());
		let mut trace_data = AuthorTraceData::<AccountId, BlockNumber>::default();
		trace_data.try_push((stash_id.unwrap(), 2));
		BlockAuthorTrace::<Test>::put(trace_data);

		AresOcw::save_fetch_ares_price_and_send_payload_signed(2, authority_1.clone()).unwrap();
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let Call::AresOcw(crate::Call::submit_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) = tx.call {
			AresOcw::submit_price_unsigned_with_signed_payload(Origin::none(), body, signature);
		}
		assert_eq!(AresOcw::get_jump_block_number(PriceKey::create_on_vec(to_test_vec("btc_price"))), 0);
		assert_eq!(AresOcw::get_jump_block_number(PriceKey::create_on_vec(to_test_vec("eth_price"))), 1);
	});

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		// uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc_eth_dot".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};
	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		init_aura_enging_digest();

		let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
		let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));
		let price_key_dot_price = PriceKey::create_on_vec(to_test_vec("dot_price"));


		let (stash_1, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();
		let (stash_2, authority_2) = <Authorities<Test>>::get().unwrap()[1].clone();

		assert_eq!(AresOcw::get_jump_block_number(price_key_btc_price.clone()), 0);
		assert_eq!(AresOcw::get_jump_block_number(price_key_eth_price.clone()), 1);
		assert_eq!(AresOcw::get_jump_block_number(price_key_dot_price.clone()), 0);

		let stash_id = AresOcw::get_stash_id(&authority_2.clone());
		let mut trace_data = AuthorTraceData::<AccountId, BlockNumber>::default();
		trace_data.try_push((stash_id.unwrap(), 3));
		BlockAuthorTrace::<Test>::put(trace_data);

		AresOcw::save_fetch_ares_price_and_send_payload_signed(3, authority_2.clone()).unwrap();
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let Call::AresOcw(crate::Call::submit_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) = tx.call {
			AresOcw::submit_price_unsigned_with_signed_payload(Origin::none(), body, signature);
		}
		assert_eq!(AresOcw::get_jump_block_number(price_key_btc_price.clone()), 0);
		assert_eq!(AresOcw::get_jump_block_number(price_key_eth_price.clone()), 1);
		assert_eq!(AresOcw::get_jump_block_number(price_key_dot_price.clone()), 0);

		assert_eq!(
			AresOcw::get_last_price_author(price_key_btc_price.clone()),
			Some((stash_2.clone(),3))
		);
		assert_eq!(
			AresOcw::get_last_price_author(price_key_eth_price.clone()),
			Some((stash_2.clone(),3))
		);
		assert_eq!(
			AresOcw::get_last_price_author(price_key_dot_price.clone()),
			Some((stash_2.clone(),3))
		);

		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_btc_price.clone(), stash_1.clone()),
			false
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_eth_price.clone(), stash_1.clone()),
			false
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_dot_price.clone(), stash_1.clone()),
			false
		);

		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_btc_price.clone(), stash_2.clone()),
			true
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_eth_price.clone(), stash_2.clone()),
			true
		);
		assert_eq!(
			AresOcw::is_need_update_jump_block(price_key_dot_price.clone(), stash_2.clone()),
			true
		);
	});
}


#[test]
fn test_debug_old_block_attack() {
	let mut t = new_test_ext();

	let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
	let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));
	let price_key_dot_price = PriceKey::create_on_vec(to_test_vec("dot_price"));
	let price_key_xrp_price = PriceKey::create_on_vec(to_test_vec("xrp_price"));

	let price_token_btc = PriceKey::create_on_vec(to_test_vec("btc"));
	let price_token_eth = PriceKey::create_on_vec(to_test_vec("eth"));
	let price_token_dot = PriceKey::create_on_vec(to_test_vec("dot"));
	let price_token_xrp = PriceKey::create_on_vec(to_test_vec("xrp"));


	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();

	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(0)
		.unwrap()
		.clone();

	let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(1)
		.unwrap()
		.clone();

	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc_eth".into(),
		response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		init_aura_enging_digest();

		let price_key_btc_price = PriceKey::create_on_vec(to_test_vec("btc_price"));
		let price_key_eth_price = PriceKey::create_on_vec(to_test_vec("eth_price"));

		let (stash_1, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();
		let (stash_2, authority_2) = <Authorities<Test>>::get().unwrap()[1].clone();
		let (stash_3, authority_3) = <Authorities<Test>>::get().unwrap()[2].clone();
		let (stash_4, authority_4) = <Authorities<Test>>::get().unwrap()[3].clone();

		assert_eq!(AresOcw::get_last_price_author(price_key_btc_price.clone()), None);
		assert_eq!(AresOcw::get_last_price_author(price_key_eth_price.clone()), None);

		let mut trace_data = AuthorTraceData::<AccountId, BlockNumber>::default();
		trace_data.try_push((stash_2.clone(), 19));
		trace_data.try_push((stash_2.clone(), 20));
		trace_data.try_push((stash_2.clone(), 21));
		trace_data.try_push((stash_1.clone(), 22));
		trace_data.try_push((stash_3.clone(), 23));
		trace_data.try_push((stash_2.clone(), 24));
		BlockAuthorTrace::<Test>::put(trace_data);

		AresOcw::save_fetch_ares_price_and_send_payload_signed(22, authority_1.clone()).unwrap();
		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		use sp_runtime::transaction_validity::{
			InvalidTransaction, TransactionPriority, TransactionSource, TransactionValidity,
			ValidTransaction,
		};


		if let Call::AresOcw(crate::Call::submit_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) = tx.call {
			AresOcw::submit_price_unsigned_with_signed_payload(Origin::none(), body, signature);
		}

		assert_eq!(BlockAuthorTrace::<Test>::get().unwrap().len(), 5usize);

		let mut price_list = PricePayloadSubPriceList::default();
		// (pub PriceKey, pub u64, pub FractionLength, pub JsonNumberValue, pub u64);
		price_list.try_push(PricePayloadSubPrice{
			0: PriceKey::create_on_vec(to_test_vec("btc_price")),
			1: 0,
			2: 0,
			3: Default::default(),
			4: 0,
		});

		// Self::handler_get_sign_public_keys(account_id.clone());
		let (acc_id, result) = Signer::<Test, <Test as Config>::OffchainAppCrypto>::any_account()
			.with_filter(vec![public_key_2.try_into().unwrap()])
			.send_unsigned_transaction(
				|account| PricePayload {
					price: price_list.clone(),
					jump_block: PricePayloadSubJumpBlockList::default(),
					block_number: 21,
					auth: authority_2.clone(),
					public: account.public.clone(),
				},
				|payload, signature| {
					let call = ares_oracle::Call::submit_price_unsigned_with_signed_payload {
						price_payload: payload,
						signature,
					} ;
					let validate_unsigned = <AresOcw as sp_runtime::traits::ValidateUnsigned>::validate_unsigned(
						TransactionSource::External,
						&call,
					);
					assert!(!validate_unsigned.is_ok());
					call
				},
			).unwrap();

		let (acc_id, result) = Signer::<Test, <Test as Config>::OffchainAppCrypto>::any_account()
			.with_filter(vec![public_key_2.try_into().unwrap()])
			.send_unsigned_transaction(
				|account| PricePayload {
					price: price_list.clone(),
					jump_block: PricePayloadSubJumpBlockList::default(),
					block_number: 24,
					auth: authority_2.clone(),
					public: account.public.clone(),
				},
				|payload, signature| {
					let call = ares_oracle::Call::submit_price_unsigned_with_signed_payload {
						price_payload: payload,
						signature,
					} ;
					let validate_unsigned = <AresOcw as sp_runtime::traits::ValidateUnsigned>::validate_unsigned(
						TransactionSource::External,
						&call,
					);
					if(validate_unsigned.is_ok()) {

					}
					assert!(validate_unsigned.is_ok());
					call
				},
			).unwrap();

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		if let Call::AresOcw(crate::Call::submit_price_unsigned_with_signed_payload
							 { price_payload: body, signature }
		) = tx.call {
			AresOcw::submit_price_unsigned_with_signed_payload(Origin::none(), body, signature);
		}

		assert_eq!(BlockAuthorTrace::<Test>::get().unwrap().len(), 4usize);
		println!("={:?}", AresPrice::<Test>::get(PriceKey::create_on_vec(to_test_vec("btc_price"))));

	});

}

#[test]
fn test_update_allowable_offset_propose() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		assert_eq!(AresOcw::price_allowable_offset(), Percent::from_percent(10));
		assert_ok!(AresOcw::update_allowable_offset_propose(Origin::root(), Percent::from_percent(20)));
		assert_eq!(AresOcw::price_allowable_offset(), Percent::from_percent(20));
	});
}

#[test]
fn test_self() {
	use sp_application_crypto::sr25519;
	use sp_runtime::MultiSignature;
	use sp_runtime::MultiSigner;

	// sp_core::sr25519::Pair(schnorrkel::Keypair).;

	// let result = AuthorityPair::verify(signature.into(), signature.into(), test_address.into());
	// assert!(result, "Result is true.")

	let msg = &b"test-message"[..];
	let (pair, _) = sr25519::Pair::generate();

	let signature = pair.sign(&msg);
	assert!(sr25519::Pair::verify(&signature, msg, &pair.public()));

	println!("msg = {:?}", &msg);
	println!("signature = {:?}", &signature);
	println!("pair.public() = {:?}", &pair.public());
	// println!("multi_signer.into_account() = {:?}", &multi_signer.into_account());

	let multi_sig = MultiSignature::from(signature); // OK
	let multi_signer = MultiSigner::from(pair.public());
	assert!(multi_sig.verify(msg, &multi_signer.into_account()));

	let multi_signer = MultiSigner::from(pair.public());
	assert!(multi_sig.verify(msg, &multi_signer.into_account()));

	//---------

	let test_signature = &hex::decode("2aeaa98e26062cf65161c68c5cb7aa31ca050cb5bdd07abc80a475d2a2eebc7b7a9c9546fbdff971b29419ddd9982bf4148c81a49df550154e1674a6b58bac84").expect("Hex invalid")[..];
	let signature = Signature::try_from(test_signature);
	let signature = signature.unwrap();
	println!(" signature = {:?}", signature);

	// let account_result =
	// AccountId::from_ss58check("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty");
	//
	// let account_id = account_result.unwrap();
	// println!(" account_id = {:?} ", account_id);

	let public_id = Public::from_ss58check("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty");
	let public_id = public_id.unwrap();
	println!(" public_id = {:?} ", public_id);

	let multi_sig = MultiSignature::from(signature); // OK
	let multi_signer = MultiSigner::from(public_id);
	assert!(multi_sig.verify("This is a text message".as_bytes(), &multi_signer.into_account()));

	// let account_encode =  "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty".encode();
	// println!(" account = {:?}, {:?}", &account_encode, account_encode.len());

	// let signedMessage_u8 = "This is a text message".as_bytes();
	// let signature_u8 =
	// &hex::decode("
	// 0x2aeaa98e26062cf65161c68c5cb7aa31ca050cb5bdd07abc80a475d2a2eebc7b7a9c9546fbdff971b29419ddd9982bf4148c81a49df550154e1674a6b58bac84"
	// );// .as_bytes();
	//
	// let test_address_u8 = "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty".as_bytes();
	//
	// let signature = Signature::try_from(signature_u8);
	// let signature = signature.unwrap();
	// println!(" signature = {:?}", signature);

	// let mut a =[0u8; 64];
	// a[..].copy_from_slice(&signature);
	// let multi_sig = MultiSignature::from(Signature(a));

	//
	// let multi_signer = MultiSigner::from(pair.public());
}

#[test]
fn test_request_propose_submit_and_revoke_propose() {
	let mut t = new_test_ext();

	t.execute_with(|| {
		assert_eq!(AresOcw::prices_requests().len(), 4);
		assert_ok!(AresOcw::update_request_propose(
			Origin::root(),
			to_test_vec("xxx_price"),
			to_test_vec("http://xxx.com"),
			2,
			4,
			1
		));
		assert_eq!(AresOcw::prices_requests().len(), 5);

		//TODO:: test not attach.
		// System::assert_last_event(tests::Event::AresOcw(AresOcwEvent::AddPriceRequest(to_test_vec("xxx_price"), to_test_vec("http://xxx.com"), 2, 4)));

		let tmp_result = AresOcw::prices_requests();
		assert_eq!(
			tmp_result[4],
			(PriceKey::create_on_vec(to_test_vec("xxx_price")), PriceToken::create_on_vec(to_test_vec("http://xxx.com")), 2, 4, 1)
		);

		assert_ok!(AresOcw::update_request_propose(
			Origin::root(),
			to_test_vec("xxx_price"),
			to_test_vec("http://aaa.com"),
			3,
			3,
			2
		));
		assert_eq!(AresOcw::prices_requests().len(), 5);
		let tmp_result = AresOcw::prices_requests();
		// price_key are same will be update .
		assert_eq!(
			tmp_result[4],
			(PriceKey::create_on_vec(to_test_vec("xxx_price")), PriceToken::create_on_vec(to_test_vec("http://aaa.com")), 3, 3, 2)
		);

		// Test revoke.
		assert_ok!(AresOcw::revoke_update_request_propose(
			Origin::root(),
			"xxx_price".as_bytes().to_vec()
		));
		assert_eq!(AresOcw::prices_requests().len(), 4);
		let tmp_result = AresOcw::prices_requests();
		// price_key are same will be update .
		// println!("== {:?}", sp_std::str::from_utf8( &tmp_result[3].1) );
		assert_eq!(tmp_result[3], (PriceKey::create_on_vec(to_test_vec("xrp_price")), PriceToken::create_on_vec(to_test_vec("xrp")), 2, 4, 4));
	});
}

#[test]
fn test_request_propose_submit_impact_on_the_price_pool() {
	let mut t = new_test_ext();

	t.execute_with(|| {
		System::set_block_number(3);

		assert_eq!(AresOcw::prices_requests().len(), 4);

		assert_ok!(AresOcw::update_request_propose(
			Origin::root(),
			to_test_vec("xxx_price"),
			to_test_vec("http://xxx.com"),
			2,
			4,
			1
		));
		assert_eq!(AresOcw::prices_requests().len(), 5);
		let tmp_result = AresOcw::prices_requests();
		assert_eq!(
			tmp_result[4],
			(
				PriceKey::create_on_vec(to_test_vec("xxx_price")),
				PriceToken::create_on_vec(to_test_vec("http://xxx.com")),
				2, 4, 1)
		);

		// Add some price
		let price_key = "xxx_price".as_bytes().to_vec(); //
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0; 32]),
			8888,
			PriceKey::create_on_vec(price_key.clone()),
			4,
			Default::default(),
			100,
			0,
			1,
		);
		AresOcw::add_price_and_try_to_agg(
			AccountId::from_raw([0; 32]),
			7777,
			PriceKey::create_on_vec(price_key.clone()),
			4,
			Default::default(),
			100,
			0,
			1,
		);
		// Get save price
		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("xxx_price".as_bytes().to_vec().clone())).unwrap();
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
				ares_price_data_from_tuple((8888, AccountId::from_raw([0; 32]), 1, 4, Default::default(), 0, 3)),
				ares_price_data_from_tuple((7777, AccountId::from_raw([0; 32]), 1, 4, Default::default(), 0, 3))
			]),
			btc_price_list
		);

		// if parse version change
		assert_ok!(AresOcw::update_request_propose(
			Origin::root(),
			to_test_vec("xxx_price"),
			to_test_vec("http://xxx.com"),
			8,
			4,
			1
		));
		assert_eq!(AresOcw::prices_requests().len(), 5);
		let tmp_result = AresOcw::prices_requests();
		assert_eq!(
			tmp_result[4],
			(
				PriceKey::create_on_vec(to_test_vec("xxx_price")),
				PriceToken::create_on_vec(to_test_vec("http://xxx.com"))
				, 8, 4, 1)
		);
		// Get old price list, Unaffected
		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("xxx_price".as_bytes().to_vec().clone())).unwrap();
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
				ares_price_data_from_tuple((8888, AccountId::from_raw([0; 32]), 1, 4, Default::default(), 0, 3)),
				ares_price_data_from_tuple((7777, AccountId::from_raw([0; 32]), 1, 4, Default::default(), 0, 3))
			]),
			btc_price_list
		);

		// Other price request get in
		assert_ok!(AresOcw::update_request_propose(
			Origin::root(),
			to_test_vec("zzz_price"),
			to_test_vec("http://zzz.com"),
			8,
			4,
			1
		));
		assert_eq!(AresOcw::prices_requests().len(), 6);
		let tmp_result = AresOcw::prices_requests();
		assert_eq!(
			tmp_result[5],
			(
				PriceKey::create_on_vec(to_test_vec("zzz_price")),
				PriceToken::create_on_vec(to_test_vec("http://zzz.com")),
				8, 4, 1)
		);
		// Get old price list, Unaffected
		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("xxx_price".as_bytes().to_vec().clone())).unwrap();
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
				ares_price_data_from_tuple((8888, AccountId::from_raw([0; 32]), 1, 4, Default::default(), 0, 3)),
				ares_price_data_from_tuple((7777, AccountId::from_raw([0; 32]), 1, 4, Default::default(), 0, 3))
			]),
			btc_price_list
		);

		// Other price request fraction number change.
		assert_ok!(AresOcw::update_request_propose(
			Origin::root(),
			to_test_vec("zzz_price"),
			to_test_vec("http://zzz.com"),
			8,
			5,
			1
		));
		assert_eq!(AresOcw::prices_requests().len(), 6);
		let tmp_result = AresOcw::prices_requests();
		assert_eq!(
			tmp_result[5],
			(
				PriceKey::create_on_vec(to_test_vec("zzz_price")),
				PriceToken::create_on_vec(to_test_vec("http://zzz.com")),
				8, 5, 1)
		);
		// Get old price list, Unaffected
		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("xxx_price".as_bytes().to_vec().clone())).unwrap();
		assert_eq!(
			AresPriceDataVecOf::<AccountId, BlockNumber>::create_on_vec(vec![
				ares_price_data_from_tuple((8888, AccountId::from_raw([0; 32]), 1, 4, Default::default(), 0, 3)),
				ares_price_data_from_tuple((7777, AccountId::from_raw([0; 32]), 1, 4, Default::default(), 0, 3))
			]),
			btc_price_list
		);

		// Current price request fraction number change. (xxx_price)
		assert_ok!(AresOcw::update_request_propose(
			Origin::root(),
			to_test_vec("xxx_price"),
			to_test_vec("http://xxx.com"),
			2,
			5,
			1
		));
		assert_eq!(AresOcw::prices_requests().len(), 6);
		let tmp_result = AresOcw::prices_requests();
		assert_eq!(
			tmp_result[5],
			(
				PriceKey::create_on_vec(to_test_vec("xxx_price")),
				PriceToken::create_on_vec(to_test_vec("http://xxx.com")),
				2, 5, 1)
		);
		// Get old price list, Unaffected
		let btc_price_list = AresOcw::ares_prices(PriceKey::create_on_vec("xxx_price".as_bytes().to_vec().clone())).unwrap_or(Default::default());
		// price will be empty.
		assert_eq!(0, btc_price_list.len());
	});
}

#[test]
fn test_rpc_request() {
	// "{\"id\":1, \"jsonrpc\":\"2.0\", \"method\": \"offchain_localStorageSet\",
	// \"params\":[\"PERSISTENT\", \ "0x746172652d6f63773a3a70726963655f726571756573745f646f6d61696e\",
	// \"0x68687474703a2f2f3134312e3136342e35382e3234313a35353636\"]}"

	// Try title : Vec<u8> encode 746172652d6f63773a3a70726963655f726571756573745f646f6d61696e
	// Try body : Vec<u8> encode 68687474703a2f2f3134312e3136342e35382e3234313a35353636
	//                             687474703a2f2f3134312e3136342e35382e3234313a35353838

	let target_json = "are-ocw::price_request_domain";
	let target_json_v8 = target_json.encode();
	println!("Try title : Vec<u8> encode {:?} ", HexDisplay::from(&target_json_v8));

	let target_json = "http://127.0.0.1:5566";
	let target_json_v8 = target_json.encode();
	println!("Try body : Vec<u8> encode {:?} ", HexDisplay::from(&target_json_v8));

	// let target_json = "are-ocw::make_price_request_pool";
	// println!("Old title : Vec<u8> encode {:?} ", HexDisplay::from(target_json));
	assert!(true);
}

#[test]
fn test_submit_ask_price_filter_request_keys() {
	let mut t = new_test_ext();
	t.execute_with(|| {

		let request_acc = AccountId::from_raw([1; 32]);
		// let request_keys = vec![to_test_vec("btc_price")];
		Balances::set_balance(Origin::root(), request_acc, 100000_000000000000, 0);
		assert_eq!(Balances::free_balance(request_acc), 100000_000000000000);
		let purchase_id = AresOcw::make_purchase_price_id(request_acc.into_account(), 0);

		assert_ok!(AresOcw::submit_ask_price(Origin::signed(request_acc), 2_000000000000, "btc_price".as_bytes().to_vec() ));
		assert_ok!(AresOcw::submit_ask_price(Origin::signed(request_acc), 2_000000000000, "btc_price,eth_price".as_bytes().to_vec() ));
		assert_noop!(
		   AresOcw::submit_ask_price(Origin::signed(request_acc), 1_000000000000, "btc_price,eth_price".as_bytes().to_vec() ),
		   Error::<Test>::InsufficientMaxFee
		);
		assert_ok!(AresOcw::submit_ask_price(Origin::signed(request_acc), 1_000000000000, "btc_price,error_price".as_bytes().to_vec() ));
		assert_noop!(
		   AresOcw::submit_ask_price(Origin::signed(request_acc), 1_000000000000, "error_price".as_bytes().to_vec() ),
		   Error::<Test>::NoPricePairsAvailable
		);

	});
}

#[test]
fn test_is_aura() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		init_aura_enging_digest();
		assert!(AresOcw::is_aura());
	});
}

#[test]
fn test_boundvec_q1 () {
	type DebugLength = ConstU32<3>;
	type DebugBoundVec = BoundedVec<u8, DebugLength>;
	let mut a = DebugBoundVec::default();

	assert_eq!(a.len(), 0);
	a.try_push(b'a');
	a.try_push(b'b');
	a.try_push(b'c');
	assert_eq!(a.len(), 3);
}

#[test]
fn test_ToBoundVec () {
	type TipLength = ConstU32<3>;
	type TestBoundVec = BoundedVec<u8, TipLength> ;

	let origin_v = vec![b'A', b'B'];
	let mut origin_b = TestBoundVec::create_on_vec(origin_v);
	assert_eq!(origin_b.len(), 2);
	origin_b.check_push(b'C');
	assert_eq!(origin_b.len(), 3);
	// println!("--- {:?}", origin_b);
	assert_eq!(origin_b[0], b'A');
	assert_eq!(origin_b[1], b'B');
	assert_eq!(origin_b[2], b'C');
}

#[test]
fn test_debug_20220721_JsonNumberValue_new () {

	let FRACTION_NUM_4: u32 = 4;

	// ftt_xlm_vet_icp_theta_algo_xmr_xtz_egld_axs_iota_ftm_ksm_hbar_neo_waves_mkr_near_btt_chz_stx_dcr_xem_omg_zec_sushi_enj_mana_yfi_iost_qtum_bat_zil_icx_grt_celo_zen_ren_sc_zrx_ont_nano_crv_bnt_fet_uma_iotx_lrc_sand_srm_kava_knc
	let mut coin_keys = Vec::new();
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("ftt")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("xlm")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("vet")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("icp")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("theta")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("algo")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("xmr")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("xtz")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("egld")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("axs")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("iota")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("ftm")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("ksm")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("hbar")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("neo")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("waves")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("mkr")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("near")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("btt")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("chz")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("stx")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("dcr")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("xem")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("omg")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("zec")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("sushi")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("enj")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("mana")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("yfi")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("iost")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("qtum")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("bat")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("zil")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("icx")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("grt")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("celo")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("zen")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("ren")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("sc")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("zrx")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("ont")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("nano")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("crv")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("bnt")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("fet")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("uma")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("iotx")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("lrc")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("sand")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("srm")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("kava")));
	coin_keys.push(PriceKey::create_on_vec(to_test_vec("knc")));

	// defined parse format
	let mut format = RawSourceKeys::default();
	for coin_key in coin_keys {
		format.try_push((coin_key.clone(), coin_key.clone(), FRACTION_NUM_4));
	}

	let result_bulk_parse = AresOcw::bulk_parse_price_of_ares(get_bug_json_of_20220721(), to_test_vec("usdt"), format);
	// println!("=={:?}", result_bulk_parse);
	assert_eq!(result_bulk_parse.len(), 51)
}


#[test]
fn test_migrate_of_UpdateToV2() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		StorageVersion::<Test>::put(crate::Releases::V1_2_0);

		let (stash_1, authority_1) = <Authorities<Test>>::get().unwrap()[0].clone();

		let old_purchase_id: PurchaseId = "aaa".as_bytes().to_vec().try_into().unwrap();

		let request_data = PurchasedRequestData {
			account_id: AccountId::from_raw([0; 32]),
			offer: 0,
			// submit_threshold: 60,
			submit_threshold: Percent::from_percent(60), //60,
			create_bn: Default::default(),
			max_duration: 0,
			request_keys: Default::default(),
		};

		let price_key = PriceKey::create_on_vec(to_test_vec("knc"));
		let purchase_data = PurchasedPriceDataVec::<AccountId, BlockNumber>::create_on_vec(vec![
			AresPriceData {
				price: 200,
				account_id: stash_1.clone(),
				create_bn: 1,
				fraction_len: 2,
				raw_number: Default::default(),
				timestamp: 0,
				update_bn: 2,
			},
		]);

		let avg_price = PurchasedAvgPriceData {
			create_bn: 1,
			reached_type: 1,
			price_data: (10000, 4),
		};

		OldPurchasedRequestPoolV1::<Test>::insert(old_purchase_id.clone(), request_data.clone());
		assert_eq!(OldPurchasedRequestPoolV1::<Test>::contains_key(&old_purchase_id), true);
		assert_eq!(OldPurchasedRequestPoolV1::<Test>::get(&old_purchase_id), Some(request_data.clone()));
		assert_eq!(PurchasedRequestPool::<Test>::contains_key(&OrderIdEnum::String(old_purchase_id.clone())), false);

		OldPurchasedPricePoolV1::<Test>::insert(&old_purchase_id, &price_key, purchase_data.clone());
		assert_eq!(OldPurchasedPricePoolV1::<Test>::contains_key(&old_purchase_id, &price_key), true);
		assert_eq!(OldPurchasedPricePoolV1::<Test>::get(&old_purchase_id, &price_key), Some(purchase_data.clone()));
		assert_eq!(PurchasedPricePool::<Test>::contains_key(&OrderIdEnum::String(old_purchase_id.clone()), &price_key), false);

		OldPurchasedAvgPriceV1::insert(&old_purchase_id, &price_key, avg_price.clone());
		assert_eq!(OldPurchasedAvgPriceV1::contains_key(&old_purchase_id, &price_key), true);
		assert_eq!(OldPurchasedAvgPriceV1::get(&old_purchase_id, &price_key), avg_price.clone());
		assert_eq!(PurchasedAvgPrice::<Test>::contains_key(&OrderIdEnum::String(old_purchase_id.clone()), &price_key), false);

		OldPurchasedAvgTraceV1::<Test>::insert(&old_purchase_id, 1000);
		assert_eq!(OldPurchasedAvgTraceV1::<Test>::contains_key(&old_purchase_id), true);
		assert_eq!(OldPurchasedAvgTraceV1::<Test>::get(&old_purchase_id), 1000);
		assert_eq!(PurchasedAvgTrace::<Test>::contains_key(&OrderIdEnum::String(old_purchase_id.clone())), false);

		OldPurchasedOrderPoolV1::<Test>::insert(&old_purchase_id, &stash_1, 2000);
		assert_eq!(OldPurchasedOrderPoolV1::<Test>::contains_key(&old_purchase_id, &stash_1), true);
		assert_eq!(OldPurchasedOrderPoolV1::<Test>::get(&old_purchase_id, &stash_1), Some(2000));
		assert_eq!(PurchasedOrderPool::<Test>::contains_key(&OrderIdEnum::String(old_purchase_id.clone()), &stash_1), false);

		UpdateToV2::<Test>::on_runtime_upgrade();

		assert_eq!(OldPurchasedRequestPoolV1::<Test>::contains_key(&old_purchase_id), false);
		assert_eq!(PurchasedRequestPool::<Test>::get(&OrderIdEnum::String(old_purchase_id.clone())), Some(request_data.clone()));

		assert_eq!(OldPurchasedPricePoolV1::<Test>::contains_key(&old_purchase_id, &price_key), false);
		assert_eq!(PurchasedPricePool::<Test>::get(&OrderIdEnum::String(old_purchase_id.clone()), &price_key), Some(purchase_data.clone()));

		assert_eq!(OldPurchasedAvgPriceV1::contains_key(&old_purchase_id, &price_key), false);
		assert_eq!(PurchasedAvgPrice::<Test>::get(&OrderIdEnum::String(old_purchase_id.clone()), &price_key), avg_price.clone());

		assert_eq!(OldPurchasedAvgTraceV1::<Test>::contains_key(&old_purchase_id), false);
		assert_eq!(PurchasedAvgTrace::<Test>::get(&OrderIdEnum::String(old_purchase_id.clone())), 1000);

		assert_eq!(OldPurchasedOrderPoolV1::<Test>::contains_key(&old_purchase_id, &stash_1), false);
		assert_eq!(PurchasedOrderPool::<Test>::get(&OrderIdEnum::String(old_purchase_id.clone()), &stash_1), Some(2000));

		assert_eq!(StorageVersion::<Test>::get(), crate::Releases::V2);
	});
}