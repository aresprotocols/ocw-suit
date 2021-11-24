use super::*;

#[test]
fn test_create_pre_check_task() {
    let mut t = new_test_ext();
    t.execute_with(|| {
        let current_bn = 1;
        System::set_block_number(current_bn);
        let acc_1 = AccountId::from_raw([1;32]).into_account();
        assert_ok!(AresOcw::create_pre_check_task(acc_1, current_bn));
        assert_eq!(<PerCheckTaskList<Test>>::get()[0], (acc_1, current_bn));

        assert_noop!(
			AresOcw::create_pre_check_task(acc_1, current_bn),
			Error::<Test>::PerCheckTaskAlreadyExists
		);
        assert_noop!(
			AresOcw::create_pre_check_task(acc_1, current_bn + 1 ),
			Error::<Test>::PerCheckTaskAlreadyExists
		);
        assert_noop!(
			AresOcw::create_pre_check_task(acc_1, current_bn + 2 ),
			Error::<Test>::PerCheckTaskAlreadyExists
		);
    });
}

#[test]
fn test_has_per_check_task() {
    let mut t = new_test_ext();
    t.execute_with(|| {
        let current_bn = 1;
        System::set_block_number(current_bn);

        assert!(!AresOcw::has_per_check_task());
        let acc_1 = AccountId::from_raw([1;32]).into_account();
        assert_ok!(AresOcw::create_pre_check_task(acc_1, current_bn));
        assert!(AresOcw::has_per_check_task());
    });
}

#[test]
fn test_take_price_for_per_check() {
    let mut t = new_test_ext();

    const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
    let (offchain, offchain_state) = testing::TestOffchainExt::new();
    let (pool, pool_state) = testing::TestTransactionPoolExt::new();

    let keystore = KeyStore::new();
    SyncCryptoStore::sr25519_generate_new(
        &keystore,
        crate::crypto::Public::ID,
        Some(&format!("{}/hunter1", PHRASE))
    ).unwrap();

    let public_key_1 = SyncCryptoStore::sr25519_public_keys(&keystore, crate::crypto::Public::ID)
        .get(0)
        .unwrap()
        .clone();

    t.register_extension(OffchainWorkerExt::new(offchain));
    t.register_extension(TransactionPoolExt::new(pool));
    t.register_extension(KeystoreExt(Arc::new(keystore)));

    let padding_request = testing::PendingRequest {
        method: "GET".into(),
        // uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt_ethusdt_dotusdt_xrpusdt".into(),
        uri: "http://127.0.0.1:5566/api/getBulkPrices?symbol=btcusdt".into(),
        response: Some(get_are_json_of_bulk().as_bytes().to_vec()),
        sent: true,
        ..Default::default()
    };

    offchain_state.write().expect_request(padding_request);


    t.execute_with(|| {
        let current_bn = 1;
        System::set_block_number(current_bn);


    });
}
