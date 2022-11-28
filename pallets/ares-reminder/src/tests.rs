use bound_vec_helper::BoundVecHelper;
use codec::Encode;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use frame_system::offchain::SigningTypes;
use sp_runtime::DispatchResult;
use oracle_finance::traits::IForPrice;
use ares_oracle_provider_support::{ChainPrice, OrderIdEnum, PriceKey, IOracleAvgPriceEvents, IStashAndAuthority};
use ares_oracle_provider_support::crypto::sr25519::AuthorityId;
use sp_core::ed25519::Public;
use crate::{MaxPendingKeepBn, MaxWaitingKeepBn, OwnerList, PendingSendList, ReminderCount, ReminderList, SymbolList, WaitingSendList};
use crate::types::{CompletePayload, PriceTrigger, ReminderCallBackUrl, ReminderCondition, ReminderIden, ReminderIdenList, ReminderReceiver, ReminderSendList, ReminderTriggerTip};
use sp_runtime::{
	testing::{Header, TestXt, UintAuthorityId},
	traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
	Perbill,
};
use sp_runtime::offchain::testing::TestOffchainExt;
use sp_runtime::offchain::testing::TestTransactionPoolExt;
use codec::Decode;

#[test]
fn test_add_reminder() {
	new_test_ext(None, None).execute_with(|| {

		let condition: ReminderCondition<PriceKey, ChainPrice> = ReminderCondition::TargetPriceModel {
			price_key: to_bound_vec("btc-usdt"),
			anchor_price: ChainPrice::new((200000000, 4)),
		};
		let receiver: ReminderReceiver = ReminderReceiver::HttpCallBack{
			url: to_bound_vec("http://google.com/a"),
			sign: Default::default()
		};
		let interval: BlockNumber = 100;
		let repeat_count = 2;
		let tip: Option<ReminderTriggerTip> = Some(to_bound_vec("The test 1"));
		let max_fee: Balance = 1 * DOLLARS;

		let ACCOUNT_6 = AccountId::from_raw([6; 32]);
		assert_eq!(Balances::free_balance(ACCOUNT_6), 60 * DOLLARS + 100);
		assert_eq!(ReminderCount::<Test>::get(), None);

		assert_eq!(AresReminder::add_reminder(
			Origin::signed(ACCOUNT_6),
			condition.clone(),
			receiver.clone(),
			interval.clone(),
			repeat_count.clone(),
			tip.clone(),
			max_fee.clone(),
		), DispatchResult::Err(crate::Error::<Test>::EstimatedCostExceedsMaximumFee.into()));

		let max_fee: Balance = 2 * DOLLARS;

		// Add new reminder.
		assert_ok!(AresReminder::add_reminder(
			Origin::signed(ACCOUNT_6),
			condition.clone(),
			receiver.clone(),
			interval.clone(),
			repeat_count.clone(),
			tip.clone(),
			max_fee.clone(),
		));

		// Reserve 2 * DOLLARS
		assert_eq!(Balances::free_balance(ACCOUNT_6), (60 * DOLLARS + 100) - (2 * DOLLARS) );
		assert_eq!(ReminderCount::<Test>::get(), Some(1));
		assert_eq!(ReminderList::<Test>::get(0), Some(PriceTrigger {
			owner: ACCOUNT_6,
			interval_bn: interval,
			repeat_count: repeat_count,
			create_bn: 1,
			update_bn: 0,
			price_snapshot: ChainPrice::new((23164822300, 6)),
			trigger_condition: condition,
			trigger_receiver: receiver,
			last_check_infos: None,
			tip,
		}));

		let mut reminder_iden_list = ReminderIdenList::default();
		reminder_iden_list.try_push(0);
		assert_eq!(SymbolList::<Test>::get(to_bound_vec("btc-usdt")), Some(reminder_iden_list.clone()));
		assert_eq!(OwnerList::<Test>::get(ACCOUNT_6), Some(reminder_iden_list));
		assert_eq!(OracleFinance::get_reserve_fee(&OrderIdEnum::Integer(0)), 2 * DOLLARS);
		assert_eq!(OracleFinance::get_locked_deposit(&OrderIdEnum::Integer(0)), 1 * DOLLARS);
		assert_eq!(OracleFinance::get_all_locked_deposit_with_acc(&ACCOUNT_6), 1 * DOLLARS);
	});
}

#[test]
fn test_remove_reminder() {
	new_test_ext(None, None).execute_with(|| {
		let condition: ReminderCondition<PriceKey, ChainPrice> = ReminderCondition::TargetPriceModel {
			price_key: to_bound_vec("btc-usdt"),
			anchor_price: ChainPrice::new((200000000, 4)),
		};
		let receiver: ReminderReceiver = ReminderReceiver::HttpCallBack{
			url: to_bound_vec("http://google.com/a"),
			sign: Default::default()
		};
		let interval: BlockNumber = 100;
		let repeat_count = 2;
		let tip: Option<ReminderTriggerTip> = Some(to_bound_vec("The test 1"));
		let max_fee: Balance = 2 * DOLLARS;

		let ACCOUNT_5 = AccountId::from_raw([5; 32]);
		let ACCOUNT_6 = AccountId::from_raw([6; 32]);
		assert_eq!(Balances::free_balance(ACCOUNT_6), 60 * DOLLARS + 100);
		assert_eq!(ReminderCount::<Test>::get(), None);

		// Add new reminder.
		assert_ok!(AresReminder::add_reminder(
			Origin::signed(ACCOUNT_6),
			condition.clone(),
			receiver.clone(),
			interval.clone(),
			repeat_count.clone(),
			tip.clone(),
			max_fee.clone(),
		));

		// remove that reminder
		assert_eq!(AresReminder::remove_reminder(
			Origin::signed(ACCOUNT_5),
			0
		), DispatchResult::Err(crate::Error::<Test>::NotTheOwnerOfReminder.into()));


		// remove that reminder
		assert_eq!(AresReminder::remove_reminder(
			Origin::signed(ACCOUNT_6),
			1
		), DispatchResult::Err(crate::Error::<Test>::ReminderIdNotExists.into()));

		assert_ok!(AresReminder::remove_reminder(
			Origin::signed(ACCOUNT_6),
			0
		));

		// Check storage status
		assert_eq!(Balances::free_balance(ACCOUNT_6), (60 * DOLLARS + 100) );
		assert_eq!(ReminderCount::<Test>::get(), Some(1));
		assert_eq!(ReminderList::<Test>::get(0), None);
		assert_eq!(SymbolList::<Test>::get(to_bound_vec("btc-usdt")), Some(ReminderIdenList::default()));
		assert_eq!(OwnerList::<Test>::get(ACCOUNT_6), Some(ReminderIdenList::default()));
		assert_eq!(OracleFinance::get_reserve_fee(&OrderIdEnum::Integer(0)), 0);
		assert_eq!(OracleFinance::get_locked_deposit(&OrderIdEnum::Integer(0)), 0);
		assert_eq!(OracleFinance::get_all_locked_deposit_with_acc(&ACCOUNT_6), 0);

	});
}

#[test]
fn test_update_reminder() {
	new_test_ext(None, None).execute_with(|| {
		let condition: ReminderCondition<PriceKey, ChainPrice> = ReminderCondition::TargetPriceModel {
			price_key: to_bound_vec("btc-usdt"),
			anchor_price: ChainPrice::new((200000000, 4)),
		};
		let receiver: ReminderReceiver = ReminderReceiver::HttpCallBack{
			url: to_bound_vec("http://google.com/a"),
			sign: Default::default()
		};
		let interval: BlockNumber = 100;
		let repeat_count = 2;
		let tip: Option<ReminderTriggerTip> = Some(to_bound_vec("The test 1"));
		let max_fee: Balance = 1000 * DOLLARS;

		let ACCOUNT_5 = AccountId::from_raw([5; 32]);
		let ACCOUNT_6 = AccountId::from_raw([6; 32]);
		assert_eq!(Balances::free_balance(ACCOUNT_6), 60 * DOLLARS + 100);
		assert_eq!(ReminderCount::<Test>::get(), None);

		// Add new reminder.
		assert_ok!(AresReminder::add_reminder(
			Origin::signed(ACCOUNT_6),
			condition.clone(),
			receiver.clone(),
			interval.clone(),
			repeat_count.clone(),
			tip.clone(),
			max_fee.clone(),
		));

		// upgrade some attributes.
		let condition: ReminderCondition<PriceKey, ChainPrice> = ReminderCondition::TargetPriceModel {
			price_key: to_bound_vec("eth-usdt"),
			anchor_price: ChainPrice::new((12000000, 4)),
		};
		let receiver: ReminderReceiver = ReminderReceiver::HttpCallBack{
			url: to_bound_vec("http://google.com/b"),
			sign: Default::default()
		};
		let interval: BlockNumber = 200;
		let repeat_count = 4;
		let tip: Option<ReminderTriggerTip> = Some(to_bound_vec("The test 2"));
		let max_fee: Balance = 1 * DOLLARS;

		assert_eq!(AresReminder::add_reminder(
			Origin::signed(ACCOUNT_6),
			condition.clone(),
			receiver.clone(),
			interval.clone(),
			repeat_count.clone(),
			tip.clone(),
			max_fee.clone(),
		), DispatchResult::Err(crate::Error::<Test>::EstimatedCostExceedsMaximumFee.into()));

		let max_fee: Balance = 2 * DOLLARS;

		assert_ok!(AresReminder::update_reminder(
			Origin::signed(ACCOUNT_6),
			0, // ReminderIden = 0
			condition.clone(),
			receiver.clone(),
			interval.clone(),
			repeat_count.clone(),
			tip.clone(),
			max_fee.clone(),
		));

		// Check storage status
		assert_eq!(Balances::free_balance(ACCOUNT_6), (60 * DOLLARS + 100) - (4 * DOLLARS) );
		assert_eq!(ReminderCount::<Test>::get(), Some(1));
		assert_eq!(ReminderList::<Test>::get(0), Some(PriceTrigger {
			owner: ACCOUNT_6,
			interval_bn: interval,
			repeat_count: repeat_count,
			create_bn: 1,
			update_bn: 0,
			price_snapshot: ChainPrice::new((23164822300, 6)),
			trigger_condition: condition,
			trigger_receiver: receiver,
			last_check_infos: None,
			tip,
		}));

		assert_eq!(SymbolList::<Test>::get(to_bound_vec("btc-usdt")), Some(ReminderIdenList::default()));
		let mut reminder_iden_list = ReminderIdenList::default();
		reminder_iden_list.try_push(0);
		assert_eq!(SymbolList::<Test>::get(to_bound_vec("eth-usdt")), Some(reminder_iden_list.clone()));
		assert_eq!(OwnerList::<Test>::get(ACCOUNT_6), Some(reminder_iden_list));
		assert_eq!(OracleFinance::get_reserve_fee(&OrderIdEnum::Integer(0)), 4 * DOLLARS);
		assert_eq!(OracleFinance::get_locked_deposit(&OrderIdEnum::Integer(0)), 1 * DOLLARS);
		assert_eq!(OracleFinance::get_all_locked_deposit_with_acc(&ACCOUNT_6), 1 * DOLLARS);

		// remove that reminder
		assert_ok!(AresReminder::remove_reminder(
			Origin::signed(ACCOUNT_6),
			0
		));

		// Check storage status
		assert_eq!(Balances::free_balance(ACCOUNT_6), (60 * DOLLARS + 100) );
		assert_eq!(ReminderCount::<Test>::get(), Some(1));
		assert_eq!(ReminderList::<Test>::get(0), None);
		assert_eq!(SymbolList::<Test>::get(to_bound_vec("btc-usdt")), Some(ReminderIdenList::default()));
		assert_eq!(SymbolList::<Test>::get(to_bound_vec("eth-usdt")), Some(ReminderIdenList::default()));
		assert_eq!(OwnerList::<Test>::get(ACCOUNT_6), Some(ReminderIdenList::default()));
		assert_eq!(OracleFinance::get_reserve_fee(&OrderIdEnum::Integer(0)), 0);
		assert_eq!(OracleFinance::get_locked_deposit(&OrderIdEnum::Integer(0)), 0);
		assert_eq!(OracleFinance::get_all_locked_deposit_with_acc(&ACCOUNT_6), 0);

	});
}

#[test]
fn test_avg_price_update_1() {
	new_test_ext(None, None).execute_with(|| {

		let condition: ReminderCondition<PriceKey, ChainPrice> = ReminderCondition::TargetPriceModel {
			price_key: to_bound_vec("sol-usdt"),
			anchor_price: ChainPrice::new((15333333, 4)),
		};
		let receiver: ReminderReceiver = ReminderReceiver::HttpCallBack{
			url: to_bound_vec("http://google.com/a"),
			sign: Default::default()
		};
		let interval: BlockNumber = 100;
		let repeat_count = 2;
		let tip: Option<ReminderTriggerTip> = Some(to_bound_vec("The test 1"));
		let max_fee: Balance = 1 * DOLLARS;

		let ACCOUNT_6 = AccountId::from_raw([6; 32]);

		let price_trigger = PriceTrigger {
			owner: ACCOUNT_6,
			interval_bn: interval,
			repeat_count: repeat_count,
			create_bn: 1,
			update_bn: 0,
			price_snapshot: ChainPrice::new((14333333, 4)),
			trigger_condition: condition,
			trigger_receiver: receiver,
			last_check_infos: None,
			tip,
		};

		let rid = 0;
		let mut rid_list = ReminderIdenList::default();
		rid_list.try_push(rid);
		ReminderList::<Test>::insert(rid, price_trigger);
		SymbolList::<Test>::insert(to_bound_vec("sol-usdt"), rid_list.clone());
		OwnerList::<Test>::insert(ACCOUNT_6, rid_list);
		ReminderCount::<Test>::put(rid+1);

		// No met.
		AresReminder::avg_price_update(to_bound_vec("btc-usdt"), 2, 211110000, 4);
		assert_eq!(WaitingSendList::<Test>::get(), None);
		// No met.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 3, 13663333, 4);
		assert_eq!(WaitingSendList::<Test>::get(), None);
		// No met.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 4, 14663333, 4);
		// Ok trigger condition met.
		assert_eq!(WaitingSendList::<Test>::get(), None);
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 5, 16663333, 4);

		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));

		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((16663333, 4)), 5), last_check_infos);
		assert_eq!(trigger.update_bn, 5);

		// If no interval is exceeded.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 6, 17663333, 4);
		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));
		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((16663333, 4)), 5), last_check_infos);
		assert_eq!(trigger.update_bn, 5);

		// if exceed interval.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 106, 17663333, 4);
		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));
		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((17663333, 4)), 106), last_check_infos);
		assert_eq!(trigger.update_bn, 5);

		// lower than initial price.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 107, 14233333, 4);
		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));
		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((14233333, 4)), 107), last_check_infos);
		assert_eq!(trigger.update_bn, 5);

		// meet the conditions again.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 108, 15433333, 4);
		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		res_send_list.try_push((0, 108));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));
		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((15433333, 4)), 108), last_check_infos);
		assert_eq!(trigger.update_bn, 108);

	});
}

#[test]
fn test_avg_price_update_2() {
	new_test_ext(None, None).execute_with(|| {

		let condition: ReminderCondition<PriceKey, ChainPrice> = ReminderCondition::TargetPriceModel {
			price_key: to_bound_vec("sol-usdt"),
			anchor_price: ChainPrice::new((15333333, 4)),
		};
		let receiver: ReminderReceiver = ReminderReceiver::HttpCallBack{
			url: to_bound_vec("http://google.com/b"),
			sign: Default::default()
		};
		let interval: BlockNumber = 100;
		let repeat_count = 2;
		let tip: Option<ReminderTriggerTip> = Some(to_bound_vec("The test 2"));
		let max_fee: Balance = 1 * DOLLARS;

		let ACCOUNT_6 = AccountId::from_raw([6; 32]);

		let price_trigger = PriceTrigger {
			owner: ACCOUNT_6,
			interval_bn: interval,
			repeat_count: repeat_count,
			create_bn: 1,
			update_bn: 0,
			price_snapshot: ChainPrice::new((16333333, 4)),
			trigger_condition: condition,
			trigger_receiver: receiver,
			last_check_infos: None,
			tip,
		};

		let rid = 0;
		let mut rid_list = ReminderIdenList::default();
		rid_list.try_push(rid);
		ReminderList::<Test>::insert(rid, price_trigger);
		SymbolList::<Test>::insert(to_bound_vec("sol-usdt"), rid_list.clone());
		OwnerList::<Test>::insert(ACCOUNT_6, rid_list);
		ReminderCount::<Test>::put(rid+1);

		// No met.
		AresReminder::avg_price_update(to_bound_vec("btc-usdt"), 2, 211110000, 4);
		assert_eq!(WaitingSendList::<Test>::get(), None);
		// No met.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 3, 17663333, 4);
		assert_eq!(WaitingSendList::<Test>::get(), None);
		// No met.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 4, 16663333, 4);
		// Ok trigger condition met.
		assert_eq!(WaitingSendList::<Test>::get(), None);
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 5, 14663333, 4);

		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));

		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((14663333, 4)), 5), last_check_infos);
		assert_eq!(trigger.update_bn, 5);

		// If no interval is exceeded.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 6, 13663333, 4);
		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));
		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((14663333, 4)), 5), last_check_infos);
		assert_eq!(trigger.update_bn, 5);

		// if exceed interval.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 106, 13663333, 4);
		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));
		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((13663333, 4)), 106), last_check_infos);
		assert_eq!(trigger.update_bn, 5);

		// lower than initial price.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 107, 16663333, 4);
		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));
		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((16663333, 4)), 107), last_check_infos);
		assert_eq!(trigger.update_bn, 5);

		// meet the conditions again.
		AresReminder::avg_price_update(to_bound_vec("sol-usdt"), 108, 11433333, 4);
		let mut res_send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		res_send_list.try_push((0, 5));
		res_send_list.try_push((0, 108));
		assert_eq!(WaitingSendList::<Test>::get(), Some(res_send_list));
		let trigger = ReminderList::<Test>::get(0).unwrap();
		assert!(trigger.last_check_infos.is_some());
		let last_check_infos = trigger.last_check_infos.unwrap();
		assert_eq!((ChainPrice::new((11433333, 4)), 108), last_check_infos);
		assert_eq!(trigger.update_bn, 108);

	});
}

#[test]
fn test_got_to_pending_list() {

	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();
	new_test_ext(Some(offchain), Some(pool)).execute_with(|| {

		let make_trigger = |who: AccountId, price_key: PriceKey, anchor_pice, snapshot_price, bn| {
			let condition: ReminderCondition<PriceKey, ChainPrice> = ReminderCondition::TargetPriceModel {
				price_key: price_key.clone(),
				anchor_price: anchor_pice,
			};

			let receiver: ReminderReceiver = ReminderReceiver::HttpCallBack{
				url: to_bound_vec("http://google.com/b"),
				sign: Default::default()
			};
			let interval: BlockNumber = 100;
			let repeat_count = 2;
			let tip: Option<ReminderTriggerTip> = Some(to_bound_vec("The test 2"));
			let max_fee: Balance = 2 * DOLLARS;

			assert_ok!(AresReminder::add_reminder(
				Origin::signed(who),
				condition.clone(),
				receiver.clone(),
				interval.clone(),
				repeat_count.clone(),
				tip.clone(),
				max_fee.clone(),
			));

			// let price_trigger = PriceTrigger {
			// 	owner: who.clone(),
			// 	interval_bn: interval,
			// 	repeat_count: repeat_count,
			// 	create_bn: bn,
			// 	update_bn: 0,
			// 	price_snapshot: snapshot_price,
			// 	trigger_condition: condition,
			// 	trigger_receiver: receiver,
			// 	last_check_infos: None,
			// 	tip,
			// };
			//
			// let rid = ReminderCount::<Test>::get().unwrap_or(0);
			// ReminderList::<Test>::insert(rid, price_trigger.clone());
			// let mut old_rid_list = SymbolList::<Test>::get(&price_key).unwrap_or(ReminderIdenList::default());
			// old_rid_list.try_push(rid);
			// SymbolList::<Test>::insert(price_key.clone(), old_rid_list);
			// let mut acc_rid_list = OwnerList::<Test>::get(&who).unwrap_or(ReminderIdenList::default());
			// acc_rid_list.try_push(rid);
			// OwnerList::<Test>::insert(who.clone(), acc_rid_list);
			// ReminderCount::<Test>::put(rid+1);
			// price_trigger
		};

		let ACCOUNT_4 = AccountId::from_raw([4; 32]);
		let ACCOUNT_5 = AccountId::from_raw([5; 32]);
		let ACCOUNT_6 = AccountId::from_raw([6; 32]);

		assert_eq!(Balances::free_balance(ACCOUNT_4), 40 * DOLLARS + 100);
		make_trigger(
			ACCOUNT_4,
			to_bound_vec("sol-usdt"),
			ChainPrice::new((15333333, 4)),
			ChainPrice::new((16333333, 4)),
			1,
		);
		assert_eq!(Balances::free_balance(ACCOUNT_4), 38 * DOLLARS + 100);

		assert_eq!(Balances::free_balance(ACCOUNT_5), 50 * DOLLARS + 100);
		make_trigger(
			ACCOUNT_5,
			to_bound_vec("sol-usdt"),
			ChainPrice::new((15333333, 4)),
			ChainPrice::new((16333333, 4)),
			1,
		);
		assert_eq!(Balances::free_balance(ACCOUNT_5), 48 * DOLLARS + 100);

		assert_eq!(Balances::free_balance(ACCOUNT_6), 60 * DOLLARS + 100);
		make_trigger(
			ACCOUNT_6,
			to_bound_vec("btc-usdt"),
			ChainPrice::new((211110000, 4)),
			ChainPrice::new((191110000, 4)),
			1,
		);
		assert_eq!(Balances::free_balance(ACCOUNT_6), 58 * DOLLARS + 100);

		assert_eq!(Balances::free_balance(ACCOUNT_4), 38 * DOLLARS + 100);
		make_trigger(
			ACCOUNT_4,
			to_bound_vec("eth-usdt"),
			ChainPrice::new((16110000, 4)),
			ChainPrice::new((17110000, 4)),
			1,
		);
		assert_eq!(Balances::free_balance(ACCOUNT_4), 36 * DOLLARS + 100);

		assert_eq!(Balances::free_balance(ACCOUNT_5), 48 * DOLLARS + 100);
		make_trigger(
			ACCOUNT_5,
			to_bound_vec("eth-usdt"),
			ChainPrice::new((16110000, 4)),
			ChainPrice::new((17110000, 4)),
			1,
		);
		assert_eq!(Balances::free_balance(ACCOUNT_5), 46 * DOLLARS + 100);

		assert_eq!(Balances::free_balance(ACCOUNT_6), 58 * DOLLARS + 100);
		make_trigger(
			ACCOUNT_6,
			to_bound_vec("eth-usdt"),
			ChainPrice::new((18110000, 4)),
			ChainPrice::new((17110000, 4)),
			1,
		);
		assert_eq!(Balances::free_balance(ACCOUNT_6), 56 * DOLLARS + 100);

		make_trigger(
			ACCOUNT_6,
			to_bound_vec("eth-usdt"),
			ChainPrice::new((18110000, 4)),
			ChainPrice::new((17110000, 4)),
			1,
		);
		assert_eq!(Balances::free_balance(ACCOUNT_6), 54 * DOLLARS + 100);

		// ReminderList::<Test>::insert(0, price_trigger_0.clone());
		// ReminderList::<Test>::insert(1, price_trigger_1.clone());
		// ReminderList::<Test>::insert(2, price_trigger_2.clone());
		// ReminderList::<Test>::insert(3, price_trigger_3.clone());
		// ReminderList::<Test>::insert(4, price_trigger_4.clone());
		// ReminderList::<Test>::insert(5, price_trigger_5.clone());
		// ReminderList::<Test>::insert(6, price_trigger_6.clone());

		assert_eq!(ReminderCount::<Test>::get().unwrap_or(0), 7);
		assert_eq!(SymbolList::<Test>::get(to_bound_vec("sol-usdt")).unwrap().len(), 2);
		assert_eq!(SymbolList::<Test>::get(to_bound_vec("btc-usdt")).unwrap().len(), 1);
		assert_eq!(SymbolList::<Test>::get(to_bound_vec("eth-usdt")).unwrap().len(), 4);
		assert_eq!(OwnerList::<Test>::get(ACCOUNT_4).unwrap().len(), 2);
		assert_eq!(OwnerList::<Test>::get(ACCOUNT_5).unwrap().len(), 2);
		assert_eq!(OwnerList::<Test>::get(ACCOUNT_6).unwrap().len(), 3);

		// Get pot fee balance.
		let finance_acc = OracleFinance::account_id();
		assert_eq!(Balances::free_balance(finance_acc.unwrap()), 0);

		// Make some Waiting list for test
		let mut send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		send_list.try_push((0, 1));
		send_list.try_push((1, 1));
		send_list.try_push((2, 1));
		send_list.try_push((3, 1));
		send_list.try_push((4, 1));
		send_list.try_push((5, 1));
		send_list.try_push((6, 1));
		WaitingSendList::<Test>::put(send_list.clone());
		assert_eq!(WaitingSendList::<Test>::get().unwrap().len(), 7);

		System::set_block_number(2);

		assert_eq!(AresReminder::get_round(send_list.len(), TestAresAuthority::get_list_of_storage().len()), 3);

		let validator_list: Vec<AccountId> = TestAresAuthority::get_list_of_storage().iter().map(|data|{
			data.0
		}).collect();
		//
		AresReminder::dispatch_waiting_list(validator_list.clone());

		// &((0+2)%6 = &((idx+blocknumber)%length
	    assert_eq!(PendingSendList::<Test>::get(&validator_list[0], &((0+2)%3+0*3, 1)), Some(2)); // Reminder(2,1) *
		assert_eq!(PendingSendList::<Test>::get(&validator_list[1], &((1+2)%3+0*3, 1)), Some(2)); // Reminder(0,1)
		assert_eq!(PendingSendList::<Test>::get(&validator_list[2], &((2+2)%3+0*3, 1)), Some(2)); // Reminder(1,1)
		assert_eq!(PendingSendList::<Test>::get(&validator_list[0], &((0+2)%3+1*3, 1)), Some(2)); // Reminder(5,1)
		assert_eq!(PendingSendList::<Test>::get(&validator_list[1], &((1+2)%3+1*3, 1)), Some(2)); // Reminder(3,1) *
		assert_eq!(PendingSendList::<Test>::get(&validator_list[2], &((2+2)%3+1*3, 1)), Some(2)); // Reminder(4,1) *
		assert_eq!(PendingSendList::<Test>::get(&validator_list[0], &((0+2)%3+2*3, 1)), None); // Reminder(8,1)
		assert_eq!(PendingSendList::<Test>::get(&validator_list[1], &((1+2)%3+2*3, 1)), Some(2)); // Reminder(6,1)

		assert_eq!(WaitingSendList::<Test>::get().unwrap().len(), 0);

		PendingSendList::<Test>::iter().for_each(|x|{
			println!("PendingSendList foreach iter A : {:?}", x);
		});

		// Check before status

		// TODO:: 调用 complete_reminder 关闭第一轮


		let complate_payload_0 = CompletePayload::<_, Vec<u8>, u32, _, _, _,> {
			reminder: (0, 1),
			response_mark: None,
			status: None,
			auth: validator_list[0].clone(),
			public: <Test as SigningTypes>::Public::from(validator_list[0].clone()),
		};

		// -------------------------
		// Reminder id is error, this does not belong to validator 1
		let complete_reminder = AresReminder::save_complete_reminder(
			TestAresAuthority::get_auth_id(&validator_list[1]).unwrap(),
			(2, 1),
			None,
			None,
		);

		// Because (2,1) owner is validator 0
		assert!(complete_reminder.is_err());

		// --- (2, 1)  Validator 0
		AresReminder::save_complete_reminder(
			TestAresAuthority::get_auth_id(&validator_list[0]).unwrap(),
			(2, 1),
			None,
			None,
		).unwrap();

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		assert_eq!(tx.signature, None);
		if let Call::AresReminder(crate::Call::submit_complete_reminder
							 {complete_payload: body, signature: signature}) = tx.call
		{
			let res = AresReminder::submit_complete_reminder(Origin::none(), body.clone(), signature);
			assert_ok!(res);
			assert_eq!(Balances::free_balance(finance_acc.unwrap()), 1 * DOLLARS);
			// PendingSendList::<Test>::get()
		}
		// ----------------
		// --- (3, 1)  Validator 1
		AresReminder::save_complete_reminder(
			TestAresAuthority::get_auth_id(&validator_list[1]).unwrap(),
			(3, 1),
			None,
			None,
		).unwrap();

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		assert_eq!(tx.signature, None);
		if let Call::AresReminder(crate::Call::submit_complete_reminder
								  {complete_payload: body, signature: signature}) = tx.call
		{
			let res = AresReminder::submit_complete_reminder(Origin::none(), body.clone(), signature);
			assert_ok!(res);
			assert_eq!(Balances::free_balance(finance_acc.unwrap()), 2 * DOLLARS);
		}
		// ----------------
		// --- (4, 1)  Validator 2
		AresReminder::save_complete_reminder(
			TestAresAuthority::get_auth_id(&validator_list[2]).unwrap(),
			(4, 1),
			None,
			None,
		).unwrap();

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		assert_eq!(tx.signature, None);
		if let Call::AresReminder(crate::Call::submit_complete_reminder
								  {complete_payload: body, signature: signature}) = tx.call
		{
			let res = AresReminder::submit_complete_reminder(Origin::none(), body.clone(), signature);
			assert_ok!(res);
			assert_eq!(Balances::free_balance(finance_acc.unwrap()), 3 * DOLLARS);
		}
		// ----------------

		// Check trigger status.
	    assert_eq!(ReminderList::<Test>::iter().count(), 7);

		let trigger_0 = ReminderList::<Test>::get(0).unwrap();
		assert_eq!(trigger_0.repeat_count, 2);

		let trigger_1 = ReminderList::<Test>::get(1).unwrap();
		assert_eq!(trigger_1.repeat_count, 2);

		let trigger_2 = ReminderList::<Test>::get(2).unwrap();
		assert_eq!(trigger_2.repeat_count, 1);

		let trigger_3 = ReminderList::<Test>::get(3).unwrap();
		assert_eq!(trigger_3.repeat_count, 1);

		let trigger_4 = ReminderList::<Test>::get(4).unwrap();
		assert_eq!(trigger_4.repeat_count, 1);

		let trigger_5 = ReminderList::<Test>::get(5).unwrap();
		assert_eq!(trigger_5.repeat_count, 2);

		let trigger_6 = ReminderList::<Test>::get(6).unwrap();
		assert_eq!(trigger_6.repeat_count, 2);

		assert_eq!(PendingSendList::<Test>::iter().count(), 4);

		// ---

		let mut send_list = ReminderSendList::<ReminderIden, BlockNumber>::default();
		send_list.try_push((0, 2));
		send_list.try_push((1, 2));
		send_list.try_push((2, 2));
		send_list.try_push((3, 2));
		send_list.try_push((4, 2));
		send_list.try_push((5, 2));
		send_list.try_push((6, 2));
		WaitingSendList::<Test>::put(send_list.clone());
		assert_eq!(WaitingSendList::<Test>::get().unwrap().len(), 7);

		System::set_block_number(3);

		PendingSendList::<Test>::iter().for_each(|x|{
			println!("PendingSendList foreach iter B : {:?}", x);
		});

		AresReminder::dispatch_waiting_list(validator_list.clone());

		assert_eq!(PendingSendList::<Test>::get(&validator_list[0], &((0+3)%3+0*3, 2)), Some(3)); 	// Reminder(0,2)
		assert_eq!(PendingSendList::<Test>::get(&validator_list[1], &((1+3)%3+0*3, 2)), Some(3)); 	// Reminder(1,2)
		assert_eq!(PendingSendList::<Test>::get(&validator_list[2], &((2+3)%3+0*3, 2)), Some(3)); 	// Reminder(2,2) // To close
		assert_eq!(PendingSendList::<Test>::get(&validator_list[0], &((0+3)%3+1*3, 2)), Some(3)); 	// Reminder(3,2) // To close
		assert_eq!(PendingSendList::<Test>::get(&validator_list[1], &((1+3)%3+1*3, 2)), Some(3)); 	// Reminder(4,2) // To close
		assert_eq!(PendingSendList::<Test>::get(&validator_list[2], &((2+3)%3+1*3, 2)), Some(3)); 	// Reminder(5,2)
		assert_eq!(PendingSendList::<Test>::get(&validator_list[0], &((0+3)%3+2*3, 2)), Some(3)); 	// Reminder(6,2)
		assert_eq!(PendingSendList::<Test>::get(&validator_list[1], &((1+3)%3+2*3, 2)), None); 		// Reminder(7,2)

		assert_eq!(PendingSendList::<Test>::get(&validator_list[1], &(0, 1)).unwrap(), 2); // -
		assert_eq!(PendingSendList::<Test>::get(&validator_list[2], &(1, 1)).unwrap(), 2);
		assert_eq!(PendingSendList::<Test>::get(&validator_list[0], &(5, 1)).unwrap(), 2);
		assert_eq!(PendingSendList::<Test>::get(&validator_list[1], &(6, 1)).unwrap(), 2);


		// --- (2, 1)  Validator 0
		AresReminder::save_complete_reminder(
			TestAresAuthority::get_auth_id(&validator_list[2]).unwrap(),
			(2, 2),
			None,
			None,
		).unwrap();

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		assert_eq!(tx.signature, None);
		if let Call::AresReminder(crate::Call::submit_complete_reminder
								  {complete_payload: body, signature: signature}) = tx.call
		{
			let res = AresReminder::submit_complete_reminder(Origin::none(), body.clone(), signature);
			assert_ok!(res);
			assert_eq!(Balances::free_balance(finance_acc.unwrap()), 4 * DOLLARS);
		}
		// ----------------
		// --- (3, 1)  Validator 1
		AresReminder::save_complete_reminder(
			TestAresAuthority::get_auth_id(&validator_list[0]).unwrap(),
			(3, 2),
			None,
			None,
		).unwrap();

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		assert_eq!(tx.signature, None);
		if let Call::AresReminder(crate::Call::submit_complete_reminder
								  {complete_payload: body, signature: signature}) = tx.call
		{
			let res = AresReminder::submit_complete_reminder(Origin::none(), body.clone(), signature);
			assert_ok!(res);
			assert_eq!(Balances::free_balance(finance_acc.unwrap()), 5 * DOLLARS);
		}
		// ----------------

		// Check trigger status.
		assert_eq!(ReminderList::<Test>::iter().count(), 5);

		let trigger_0 = ReminderList::<Test>::get(0).unwrap();
		assert_eq!(trigger_0.repeat_count, 2);

		let trigger_1 = ReminderList::<Test>::get(1).unwrap();
		assert_eq!(trigger_1.repeat_count, 2);

		let trigger_4 = ReminderList::<Test>::get(4).unwrap();
		assert_eq!(trigger_4.repeat_count, 1);

		let trigger_5 = ReminderList::<Test>::get(5).unwrap();
		assert_eq!(trigger_5.repeat_count, 2);

		let trigger_6 = ReminderList::<Test>::get(6).unwrap();
		assert_eq!(trigger_6.repeat_count, 2);

	});
}

#[test]
fn test_http_request() {
	let (offchain, offchain_state) = TestOffchainExt::new();
	let (pool, pool_state) = TestTransactionPoolExt::new();
	new_test_ext(Some(offchain), Some(pool)).execute_with(|| {


		let validator_list: Vec<AccountId> = TestAresAuthority::get_list_of_storage().iter().map(|data|{
			data.0
		}).collect();

		System::set_block_number(10);

		let condition: ReminderCondition<PriceKey, ChainPrice> = ReminderCondition::TargetPriceModel {
			price_key: to_bound_vec("sol-usdt"),
			anchor_price: ChainPrice::new((153333, 4)),
		};
		let receiver: ReminderReceiver = ReminderReceiver::HttpCallBack{
			url: to_bound_vec("http://localhost/http_call/?sender=1"),
			sign: Default::default()
		};
		let interval: BlockNumber = 100;
		let repeat_count = 2;
		let tip: Option<ReminderTriggerTip> = Some(to_bound_vec("The test 2"));
		let max_fee: Balance = 1 * DOLLARS;
		let rid: ReminderIden = 0;

		let ACCOUNT_6 = AccountId::from_raw([6; 32]);

		let price_trigger = PriceTrigger {
			owner: ACCOUNT_6,
			interval_bn: interval,
			repeat_count: repeat_count,
			create_bn: 1,
			update_bn: 0,
			price_snapshot: ChainPrice::new((163333, 4)),
			trigger_condition: condition,
			trigger_receiver: receiver,
			last_check_infos: None,
			tip,
		};

		let mut rid_list = ReminderIdenList::default();
		rid_list.try_push(rid);

		ReminderList::<Test>::insert(rid, price_trigger);
		SymbolList::<Test>::insert(to_bound_vec("sol-usdt"), rid_list.clone());
		OwnerList::<Test>::insert(ACCOUNT_6, rid_list);
		ReminderCount::<Test>::put(rid+1);
		PendingSendList::<Test>::insert(validator_list[0], (rid, 10),10);

		payload_response(&mut offchain_state.write(),"http://localhost/http_call/?sender=1&sign=");

		AresReminder::call_reminder();

		let tx = pool_state.write().transactions.pop().unwrap();
		let tx = Extrinsic::decode(&mut &*tx).unwrap();

		assert_eq!(tx.signature, None);
		if let Call::AresReminder(crate::Call::submit_complete_reminder
								  {complete_payload: body, signature: signature}) = tx.call
		{
			assert_eq!(&body.reminder, &(0,10));
			assert!(&body.response_mark.is_some());
		}else{
			assert!(false, "Can not find TX");
		}
	});
}

