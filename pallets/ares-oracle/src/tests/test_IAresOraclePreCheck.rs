use ares_oracle_provider_support::*;
use sp_runtime::Percent;
use sp_application_crypto::UncheckedFrom;
use sp_core::offchain::TransactionPoolExt;
use sp_keystore::KeystoreExt;
use sp_keystore::testing::KeyStore;
use sp_runtime::offchain::{OffchainWorkerExt, testing};
use sp_runtime::traits::IdentifyAccount;
use crate::mock::*;
use crate::{AresAvgPrice, AuthorityId as AuraId, FinalPerCheckResult, PreCheckTaskList};
use sp_std::sync::Arc;
use bound_vec_helper::BoundVecHelper;
use sp_keystore::SyncCryptoStore;
use sp_runtime::RuntimeAppPublic;
use crate::test_tools::{get_are_dot_eth_btc, get_are_json_of_bulk, to_test_vec};

#[test]
fn test_create_pre_check_task() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		let current_bn = 1;
		System::set_block_number(current_bn);

		let stash_1 = AccountId::from_raw([1; 32]).into_account();
		let auth_1 = <Test as crate::Config>::AuthorityAres::unchecked_from([101; 32]);

		assert!(AresOcw::create_pre_check_task(
			stash_1.clone(),
			auth_1.clone(),
			current_bn
		));
		assert_eq!(
			<PreCheckTaskList<Test>>::get().unwrap_or(Default::default())[0],
			(stash_1.clone(), auth_1.clone(), current_bn)
		);
		// println!("1111{:?}", <FinalPerCheckResult<Test>>::get(stash_1.clone()));
		assert_eq!(
			<FinalPerCheckResult<Test>>::get(stash_1.clone()).unwrap(),
			Some((1, PreCheckStatus::Review, None, auth_1.clone()))
		);

		assert_eq!(
			AresOcw::create_pre_check_task(stash_1.clone(), auth_1.clone(), current_bn),
			false // Error::<Test>::PerCheckTaskAlreadyExists
		);
		assert_eq!(
			AresOcw::create_pre_check_task(stash_1.clone(), auth_1.clone(), current_bn + 1),
			false // Error::<Test>::PerCheckTaskAlreadyExists
		);
		assert_eq!(
			AresOcw::create_pre_check_task(stash_1.clone(), auth_1.clone(), current_bn + 2),
			false // Error::<Test>::PerCheckTaskAlreadyExists
		);
	});
}

#[test]
fn test_has_pre_check_task() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		let current_bn = 1;
		System::set_block_number(current_bn);

		let stash_1 = AccountId::from_raw([1; 32]).into_account();
		let auth_1 = <Test as crate::Config>::AuthorityAres::unchecked_from([101; 32]);

		let stash_2 = AccountId::from_raw([2; 32]).into_account();

		assert!(!AresOcw::has_pre_check_task(stash_1));
		assert!(AresOcw::create_pre_check_task(stash_1, auth_1.clone(), current_bn));
		assert!(!AresOcw::has_pre_check_task(stash_1));

		assert!(!AresOcw::has_pre_check_task(stash_2));
	});
}

#[test]
fn test_get_pre_task_by_authority_set() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		let current_bn = 1;
		System::set_block_number(current_bn);

		let stash_1 = AccountId::from_raw([1; 32]).into_account();
		let auth_1 = <Test as crate::Config>::AuthorityAres::unchecked_from([101; 32]);

		assert!(!AresOcw::has_pre_check_task(stash_1));
		assert!(AresOcw::create_pre_check_task(stash_1, auth_1.clone(), current_bn));

		// Make Auth id set
		let auth_list = vec![
			<Test as crate::Config>::AuthorityAres::unchecked_from([103; 32]),
			<Test as crate::Config>::AuthorityAres::unchecked_from([101; 32]),
			<Test as crate::Config>::AuthorityAres::unchecked_from([102; 32]),
		];

		assert_eq!(
			AresOcw::get_pre_task_by_authority_set(auth_list),
			Some((stash_1, auth_1, current_bn))
		);
	});
}

//noinspection RsDetachedFile
#[test]
fn test_take_price_for_pre_check() {
	let mut t = new_test_ext();

	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();

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
		let current_bn = 1;
		System::set_block_number(current_bn);

		// create check config
		let check_config = PreCheckTaskConfig {
			check_token_list: TokenList::create_on_vec(vec![PriceToken::create_on_vec( to_test_vec("eth_price")), PriceToken::create_on_vec(to_test_vec("btc_price"))]),
			allowable_offset: Percent::from_percent(10),
		};

		// get check result
		let take_price_list = AresOcw::take_price_for_pre_check(check_config.clone());

		assert_eq!(take_price_list.len(), 2);

		assert_eq!(
			take_price_list[0],
			PreCheckStruct {
				price_key: PriceKey::create_on_vec(to_test_vec("btc_price")),
				number_val: JsonNumberValue {
					integer: 50261,
					fraction: 372,
					fraction_length: 3,
					exponent: 0,
				},
				max_offset: check_config.allowable_offset.clone(),
				timestamp: 1629699168,
			}
		);

		assert_eq!(
			take_price_list[1],
			PreCheckStruct {
				price_key: PriceKey::create_on_vec(to_test_vec("eth_price")),
				number_val: JsonNumberValue {
					integer: 3107,
					fraction: 71,
					fraction_length: 2,
					exponent: 0
				},
				max_offset: check_config.allowable_offset.clone(),
				timestamp: 1630055777,
			}
		);
	});
}

#[test]
fn save_pre_check_result_for_success() {
	let mut t = new_test_ext();

	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(0)
		.unwrap()
		.clone();
	let candidate_account = public_key_1.into_account();

	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	let padding_request = testing::PendingRequest {
		method: "GET".into(),
		uri: "http://127.0.0.1:5566/api/getBulkCurrencyPrices?currency=usdt&symbol=btc_eth_dot".into(),
		response: Some(get_are_dot_eth_btc().as_bytes().to_vec()),
		sent: true,
		..Default::default()
	};

	offchain_state.write().expect_request(padding_request);

	t.execute_with(|| {
		let current_bn = 1;
		System::set_block_number(current_bn);

		// get check result
		// create check config
		let check_config = PreCheckTaskConfig {
			check_token_list: TokenList::create_on_vec(vec![
				PriceToken::create_on_vec(to_test_vec("eth_price")),
				PriceToken::create_on_vec(to_test_vec("btc_price")),
				PriceToken::create_on_vec(to_test_vec("dot_price")),
			]),
			allowable_offset: Percent::from_percent(10),
		};

		// get check result
		let take_price_list = AresOcw::take_price_for_pre_check(check_config.clone());

		// Create avg price
		// let btc_avg_price =
		<AresAvgPrice<Test>>::insert(
			PriceKey::create_on_vec(to_test_vec("btc_price")),
			(232861414 - Percent::from_percent(9) * 232861414, 4, System::block_number()),
		);
		<AresAvgPrice<Test>>::insert(
			PriceKey::create_on_vec(to_test_vec("eth_price")),
			(16092625 + Percent::from_percent(9) * 16092625, 4, System::block_number()),
		);
		<AresAvgPrice<Test>>::insert(
			PriceKey::create_on_vec(to_test_vec("dot_price")),
			(77413 + Percent::from_percent(9) * 77413, 4, System::block_number()),
		);

		// check before status
		let get_status: Option<(BlockNumber, PreCheckStatus)> = AresOcw::get_pre_check_status(candidate_account);
		assert_eq!(get_status, None);

		// Check price
		let auth_1 = <Test as crate::Config>::AuthorityAres::unchecked_from([101; 32]);
		AresOcw::save_pre_check_result(candidate_account, current_bn, take_price_list, auth_1);

		//
		let get_status: Option<(BlockNumber, PreCheckStatus)> = AresOcw::get_pre_check_status(candidate_account);
		assert_eq!(get_status, Some((current_bn, PreCheckStatus::Pass)));

		let final_check_result = <FinalPerCheckResult<Test>>::get(candidate_account);

		AresOcw::clean_pre_check_status(candidate_account);
		let get_status: Option<(BlockNumber, PreCheckStatus)> = AresOcw::get_pre_check_status(candidate_account);
		assert_eq!(get_status, None);
	});
}

#[test]
fn save_pre_check_result_for_prohibit() {
	let mut t = new_test_ext();

	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(&keystore, AuraId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();

	let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, AuraId::ID)
		.get(0)
		.unwrap()
		.clone();
	let candidate_account = public_key_1.into_account();

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
		let current_bn = 1;
		System::set_block_number(current_bn);

		// get check result
		// create check config
		let check_config = PreCheckTaskConfig {
			check_token_list: TokenList::create_on_vec(
				vec![
					PriceToken::create_on_vec(to_test_vec("eth_price")),
					PriceToken::create_on_vec(to_test_vec("btc_price")),
				]
			),
			allowable_offset: Percent::from_percent(10),
		};

		// get check result
		let take_price_list = AresOcw::take_price_for_pre_check(check_config.clone());

		// Create avg price
		<AresAvgPrice<Test>>::insert(
			PriceKey::create_on_vec( to_test_vec("btc_price")),
			(502613720 - Percent::from_percent(11) * 502613720, 4, System::block_number()),
		);
		<AresAvgPrice<Test>>::insert(
			PriceKey::create_on_vec(to_test_vec("eth_price")),
			(31077100 + Percent::from_percent(11) * 31077100, 4, System::block_number()),
		);

		// check before status
		let get_status: Option<(BlockNumber, PreCheckStatus)> = AresOcw::get_pre_check_status(candidate_account);
		assert_eq!(get_status, None);

		// Check price
		let auth_1 = <Test as crate::Config>::AuthorityAres::unchecked_from([101; 32]);
		AresOcw::save_pre_check_result(candidate_account, current_bn, take_price_list, auth_1);

		//
		let get_status: Option<(BlockNumber, PreCheckStatus)> = AresOcw::get_pre_check_status(candidate_account);
		assert_eq!(get_status, Some((current_bn, PreCheckStatus::Prohibit)));
	});
}
