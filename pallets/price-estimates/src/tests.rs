use codec::Encode;
use crate::mock::{AresId, Balances, Call, new_test_ext, run_to_block, AccountId, Test, BlockNumber, Estimates, Extrinsic, Origin, System, TestPalletId, Balance, helper_create_new_estimates_with_deviation, helper_create_new_estimates_with_range, TestSymbolInfo};
use frame_support::{assert_noop, assert_ok};
use sp_keystore::SyncCryptoStore;
use sp_core::Public;
use sp_keystore::KeystoreExt;
use sp_keystore::testing::KeyStore;
use sp_runtime::traits::AccountIdConversion;
use sp_std::sync::Arc;
use bound_vec_helper::BoundVecHelper;
use crate::*;
use crate::types::{BoundedVecOfChooseWinnersPayload, BoundedVecOfConfigRange, BoundedVecOfMultiplierOption, BoundedVecOfSymbol, ChainPrice, ConvertChainPrice};
use sp_core::{
    crypto::key_types::DUMMY,
    offchain::{testing::TestOffchainExt, OffchainDbExt, StorageKind, OffchainWorkerExt, testing, TransactionPoolExt},
};

#[test]
fn test_call_preference() {
    let mut t = new_test_ext();
    let (offchain, _state) = testing::TestOffchainExt::new();
    t.register_extension(OffchainWorkerExt::new(offchain));
    t.execute_with(|| {

        // Get configuration informations form storage.
        let admins = Admins::<Test>::get();
        let white_list = Whitelist::<Test>::get();
        let locked_estimates =LockedEstimates::<Test>::get();
        let min_ticket_price = MinimumTicketPrice::<Test>::get();
        let min_init_reward = MinimumInitReward::<Test>::get();

        // Variables configured in creation.
        assert_eq!(admins, vec![AccountId::from_raw([1; 32])]);
        assert_eq!(white_list, vec![AccountId::from_raw([2; 32])]);
        assert_eq!(locked_estimates, 2);
        assert_eq!(min_ticket_price, 100);
        assert_eq!(min_init_reward, 100);

        // Update configuration.
        assert_ok!(Estimates::preference(
            Origin::root(),
            Some(vec![AccountId::from_raw([7; 32])]), // admins: Option<Vec<T::AccountId>>,
            Some(vec![AccountId::from_raw([8; 32])]), // whitelist: Option<Vec<T::AccountId>>,
            Some(50), // locked_estimates: Option<T::BlockNumber>,
            Some(600), // minimum_ticket_price: Option<BalanceOf<T, I>>,
            Some(700), // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));

        // Get configuration informations form storage.
        let admins = Admins::<Test>::get();
        let white_list = Whitelist::<Test>::get();
        let locked_estimates =LockedEstimates::<Test>::get();
        let min_ticket_price = MinimumTicketPrice::<Test>::get();
        let min_init_reward = MinimumInitReward::<Test>::get();

        // Configuration variables have been updated.
        assert_eq!(admins, vec![AccountId::from_raw([7; 32])]);
        assert_eq!(white_list, vec![AccountId::from_raw([8; 32])]);
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
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

        //
        helper_create_new_estimates_with_deviation (
            5,
            deviation,
            init_reward,
            price,
        );

        // // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(10);
        assert_eq!(System::block_number(), 10) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        let completed = CompletedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );
        assert_eq!(completed.len(), 0);
        let estimate_config = ActiveEstimates::<Test>::get(BoundedVecOfActiveEstimates::create_on_vec(symbol.clone()));
        assert!(estimate_config.is_some());
        let mut estimate_config = estimate_config.unwrap();

        // ######################### SETP 3 ##########################
        // Go to the end block and call cal_winners
        run_to_block(16);

        let tx = pool_state.write().transactions.pop().unwrap();
        let tx = Extrinsic::decode(&mut &*tx).unwrap();
        assert_eq!(tx.signature.unwrap().0, 1);
        // println!("tx.call = {:?}", tx.call);
        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners: BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default(),
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok(),
            }
        }));

        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // Estimate will to end without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        let completed = CompletedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
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
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

        //
        helper_create_new_estimates_with_deviation (
            5,
            deviation,
            init_reward,
            price,
        );

        // // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(10);
        assert_eq!(System::block_number(), 10) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

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
        assert_eq!(tx.signature.unwrap().0, 1);
        // println!("tx.call = {:?}", tx.call);

        // Create winner
        let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
        winners.try_push(account_participate.clone());

        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners,
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok(), // From SymbolInfo Interface.
            }
        }));

        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // ################### Next, wait for the reward.
        // ###############################################
        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check winner free balance , the last value subtracted is the transfer fee.
        assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 + init_reward - 1);

    });
}

#[test]
fn test_call_new_estimates_with_DEVIATION_with_invalid_price() {
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
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

        run_to_block(50);

        //
        helper_create_new_estimates_with_deviation (
            80,
            deviation,
            init_reward,
            price,
        );

        // // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(85);
        assert_eq!(System::block_number(), 85) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            Some(vec![AccountId::from_raw([6; 32])]),
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

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

        // Check
        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Not input CompletedEstimates
        assert_eq!(0 , CompletedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ).len());

        // Check UnresolvedEstimates
        assert!(UnresolvedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Use admin go to force completed.
        assert_noop!(
		   Estimates::force_complete(
                Origin::signed(AccountId::from_raw([3; 32])),
                symbol.clone(),
                231648223,
                4,
            ),
		   Error::<Test>::NotMember
		);

        assert_ok!(
		   Estimates::force_complete(
                Origin::signed(AccountId::from_raw([6; 32])),
                symbol.clone(),
                231648223,
                4,
            )
		);

        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // ################### Next, wait for the reward.
        // ###############################################
        assert_eq!(1, CompletedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ).len());

        // Check winner free balance , the last value subtracted is the transfer fee.
        assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 + init_reward - 1);

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
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

        //
        helper_create_new_estimates_with_deviation (
            5,
            deviation,
            init_reward,
            price,
        );

        // // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(10);
        assert_eq!(System::block_number(), 10) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

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
        assert_eq!(tx.signature.unwrap().0, 1);
        // println!("tx.call = {:?}", tx.call);

        // Create winner
        let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
        winners.try_push(account_participate1.clone());
        winners.try_push(account_participate2.clone());

        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners,
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok() , // From SymbolInfo Interface.
            }
        }));

        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // ################### Next, wait for the reward.
        // ###############################################
        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
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
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

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
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));

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
        assert_eq!(tx.signature.unwrap().0, 1);
        // println!("tx.call = {:?}", tx.call);

        // Create winner, No one is winner.
        let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
        // winners.try_push(account_participate.clone());

        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners,
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok(), // From SymbolInfo Interface.
            }
        }));


        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // ################### Next, wait for the reward.
        // ###############################################
        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check winner free balance , the last value subtracted is the transfer fee.
        assert_eq!(Balances::free_balance(&account_participate.account.clone()), 3000000000100 - price * 3);
    });
}

#[test]
fn test_enviroment() {
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

    let public_key_2 = SyncCryptoStore::sr25519_public_keys(&keystore, AresId::ID)
        .get(1)
        .unwrap()
        .clone();

    t.register_extension(OffchainWorkerExt::new(offchain));
    t.register_extension(TransactionPoolExt::new(pool));
    t.register_extension(KeystoreExt(Arc::new(keystore)));

    t.execute_with(|| {
        run_to_block(5);

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
        // let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
        let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
        let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
        // let multiplier: Vec<MultiplierOption> = vec![
        //     MultiplierOption::Base(1),
        //     MultiplierOption::Base(3),
        //     MultiplierOption::Base(5),
        // ]; //     multiplier: Vec<MultiplierOption>,
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

        // ######################### SETP 2 ##########################
        // Create new estimates.
        helper_create_new_estimates_with_range (
            5,
            vec![21481_3055u64, 23481_3055u64, 27481_3055u64, 29481_3055u64],
            4,
            init_reward,
            price,
        );

        // // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(10);
        assert_eq!(System::block_number(), 10) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

        // ######################### SETP 3 ##########################
        // Go to the end block and call cal_winners
        run_to_block(16);

        let tx = pool_state.write().transactions.pop().unwrap();
        let tx = Extrinsic::decode(&mut &*tx).unwrap();
        assert_eq!(tx.signature.unwrap().0, 1);
        println!("tx.call = {:?}", tx.call);
        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners: BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default(),
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok(),
            }
        }));

        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // The estimate will force to end without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
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
        let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
        let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
        let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

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

        // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(10);
        assert_eq!(System::block_number(), 10) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

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
        assert_eq!(tx.signature.unwrap().0, 1);
        // println!("tx.call = {:?}", tx.call);

        // Create winner
        let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
        winners.try_push(account_participate.clone());

        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners,
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok(), // From SymbolInfo Interface.
            }
        }));

        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // ################### Next, wait for the reward.
        // ###############################################
        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
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
        let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
        let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
        let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

        // Create new estimates. (23164_8223)
        helper_create_new_estimates_with_range (
            5,
            // 23164_8223 <= 23481_3055u64 YES 0
            vec![23481_3055u64, 27481_3055u64, 29481_3055u64, 31481_3055u64],
            4,
            init_reward,
            price,
        );

        // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(10);
        assert_eq!(System::block_number(), 10) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

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
        assert_eq!(tx.signature.unwrap().0, 1);
        // println!("tx.call = {:?}", tx.call);

        // Create winner
        let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
        winners.try_push(account_participate.clone());

        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners,
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok(), // From SymbolInfo Interface.
            }
        }));

        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // ################### Next, wait for the reward.
        // ###############################################
        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
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
        let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
        let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
        let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

        // Create new estimates. (23164_8223)
        helper_create_new_estimates_with_range (
            5,
            // 23164_8223 > 23064_8223u64 YES. so select index 4
            vec![20064_8223u64, 21064_8223u64, 22064_8223u64, 23064_8223u64],
            4,
            init_reward,
            price,
        );

        // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(10);
        assert_eq!(System::block_number(), 10) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

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
        assert_eq!(tx.signature.unwrap().0, 1);
        // println!("tx.call = {:?}", tx.call);

        // Create winner
        let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
        winners.try_push(account_participate.clone());

        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners,
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok(), // From SymbolInfo Interface.
            }
        }));

        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // ################### Next, wait for the reward.
        // ###############################################
        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
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
        let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
        let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
        let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

        // Create new estimates. (23164_8223)
        helper_create_new_estimates_with_range (
            5,
            // 23164_8223 > 23064_8223u64 YES. so select index 4
            vec![20064_8223u64, 21064_8223u64, 22064_8223u64, 23064_8223u64],
            4,
            init_reward,
            price,
        );

        // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(10);
        assert_eq!(System::block_number(), 10) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

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
        assert_eq!(tx.signature.unwrap().0, 1);
        // println!("tx.call = {:?}", tx.call);

        // Create winner
        let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
        winners.try_push(account_participate1.clone());
        winners.try_push(account_participate2.clone());

        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners,
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok(), // From SymbolInfo Interface.
            }
        }));

        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // ################### Next, wait for the reward.
        // ###############################################
        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
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
        let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
        let deviation = Permill::from_percent(10); //     deviation: Option<Permill>,
        let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
        let init_reward: BalanceOf<Test,()> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T, I>,
        let price: BalanceOf<Test,()> = 500; //     #[pallet::compact] price: BalanceOf<T, I>,

        // Create new estimates. (23164_8223)
        helper_create_new_estimates_with_range (
            5,
            // 23164_8223 > 23064_8223u64 YES. so select index 4
            vec![20064_8223u64, 21064_8223u64, 22064_8223u64, 23064_8223u64],
            4,
            init_reward,
            price,
        );

        // Check estimate.
        let estimate = PreparedEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        );

        // ######################### SETP 2 ##########################
        // The arrival start block ActiveEstimates will fill new value.

        run_to_block(10);
        assert_eq!(System::block_number(), 10) ;
        assert!(ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        let mut active_estimate = estimate.clone().unwrap();
        active_estimate.state = EstimatesState::Active;
        assert_eq!(Some(active_estimate), ActiveEstimates::<Test>::get(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));
        // Then PreparedEstimates is empty.
        assert!(!PreparedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // Check that the white list account is valid? No!!!!!
        assert!(!Estimates::can_send_signed(), "Not valid");
        // So need to set it up first.
        assert_ok!(Estimates::preference(
            Origin::root(),
            None, // admins: Option<Vec<T::AccountId>>,
            Some(vec![public_key_1]), // whitelist: Option<Vec<T::AccountId>>,
            None, // locked_estimates: Option<T::BlockNumber>,
            None, // minimum_ticket_price: Option<BalanceOf<T, I>>,
            None, // minimum_init_reward: Option<BalanceOf<T, I>>,
        ));
        // Check that the white list account is valid ? Yes, it's set.
        assert!(Estimates::can_send_signed(), "Yes, it's set");

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
        assert_eq!(tx.signature.unwrap().0, 1);
        // println!("tx.call = {:?}", tx.call);

        // Create winner
        let mut winners = BoundedVecOfChooseWinnersPayload::<AccountId, BlockNumber>::default();
        winners.try_push(account_participate1.clone());

        assert_eq!(tx.call, Call::Estimates(crate::Call::choose_winner {
            winner_payload: ChooseWinnersPayload {
                block_number: 16, // Why not 15?
                winners,
                public: Some(public_key_1.clone()),
                estimates_id: 0,
                symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone() ),
                price: TestSymbolInfo::price(&symbol).ok(), // From SymbolInfo Interface.
            }
        }));

        if let Call::Estimates(
            crate::Call::choose_winner {
                winner_payload: body,
            }) = tx.call {
            assert_ok!(Estimates::choose_winner(
                Origin::signed(public_key_1.clone()),
                body
            ));
        }

        // You can't end the event without WINNER.
        assert!(!ActiveEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
        ));

        // ################### Next, wait for the reward.
        // ###############################################
        assert!(CompletedEstimates::<Test>::contains_key(
            BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
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
fn test_subaccount() {

    let acc: AccountId = TestPalletId.into_account();
    println!("ACC: {:?}", acc );

    let uni_usdt: AccountId = TestPalletId.into_sub_account("uni-usdt".as_bytes().to_vec());
    // 6d6f 64 6c70792f61 7265737424 20756e692d75736474 00000000000000000000
    // 6d6f646c70792f61726573742420756e692d7573647400000000000000000000
    println!("uni_usdt 1 = {:?}, {:?}", uni_usdt, "uni-usdt".as_bytes().to_vec());

    let uni_usdt: AccountId = TestPalletId.into_sub_account(b"uni-usdt");
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
    let btc_usdc: AccountId = TestPalletId.into_sub_account("btc-usdt".as_bytes());
    // 6d6f646c70792f6172657374206274632d757364740000000000000000000000 ERROR
    // 6d6f646c70792f61726573746274632d75736474000000000000000000000000 RIGTH

    println!("ENCODE A={:?}", "btc-usdt".encode());
    println!("ENCODE A={:?}", "btc-usdt".as_bytes().encode());

    // ---- success
    let left: AccountId = TestPalletId.into_sub_account(b"btc-usdt");

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
    let right: AccountId = TestPalletId.into_sub_account(u8list);
    println!("btc-usdt hex = {:?}", right);
    assert_eq!(
        left,
        right
    );
    // ---------

    // JS:      0x6d6f646c70792f6172657374616176652d757364740000000000000000000000
    // RUST:      6d6f646c70792f6172657374616176652d757364740000000000000000000000
    let left: AccountId = TestPalletId.into_sub_account(b"aave-usdt");
    let mut u8list: [u8; 20] = [0; 20]; /// .. "btc-usdt".as_bytes();
    // u8list.copy_from_slice("btc-usdt".as_bytes());
    u8list[.."aave-usdt".len()].copy_from_slice("aave-usdt".as_bytes().to_vec().as_slice());
    let right: AccountId = TestPalletId.into_sub_account(u8list);
    println!("aave-usdt hex = {:?}", right);
    assert_eq!(
        left,
        right
    );

    println!("btc-usdt = {:?}", btc_usdc);

    let aave_usdt: AccountId = TestPalletId.into_sub_account("aave-usdt".as_bytes().to_vec());
    println!("ERROR : aave_usdt = {:?}", aave_usdt);

    let aave_usdc: AccountId = TestPalletId.into_sub_account("aave-usdc".as_bytes().to_vec());
    println!("ERROR : aave_usdc = {:?}", aave_usdc);

    let aavel_usdc: AccountId = TestPalletId.into_sub_account("aavel-usdc".as_bytes().to_vec());
    println!("ERROR : aavel_usdc = {:?}", aavel_usdc);

}

#[test]
fn test_subaccount2() {

    let acc: AccountId = TestPalletId.into_account();
    println!("ACC: {:?}", acc );

    let symbol="a";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="ab";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abc";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcd";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcde";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdef";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefg";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefgh";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefghi";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefghij";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefghijk";
    let show: AccountId = TestPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);
}

#[test]
fn test_ChainPrcie() {
    // Test create
    let one = ChainPrice::new((10123, 3u32)); // 10.123
    // println!("{:?}", <ChainPrice as ConvertChainPrice<Balance, u32>>::try_to_price(one, 4));
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