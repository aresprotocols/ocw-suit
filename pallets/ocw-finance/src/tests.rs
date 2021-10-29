use crate::{mock::*, Error, PaymentTrace, AskPeriodPayment, RewardTrace, AskPeriodPoint};
use frame_support::{assert_noop, assert_ok};
use crate::traits::*;
use crate::types::*;
use frame_support::traits::OnInitialize;

#[test]
fn test_it_works_for_default_value() {
	new_test_ext().execute_with(|| {

		let calculate_result = OcwFinance::calculate_fee_of_ask_quantity(3);
		assert_eq!(calculate_result, 3u64.saturating_mul(DOLLARS));
	});
}

#[test]
fn test_payment_for_ask_quantity() {
	new_test_ext().execute_with(|| {

		let current_bn: u64 = 1;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId: u64 = 3u64;
		const price_count: u32 = 3;

		let calculate_result = OcwFinance::calculate_fee_of_ask_quantity(price_count);
		let payment_result = OcwFinance::payment_for_ask_quantity(AccountId, toVec("Purchased_ID"), price_count);

		assert_eq!(Balances::free_balance(AccountId), 100);
		assert_eq!(payment_result, OcwPaymentResult::Success(toVec("Purchased_ID"), calculate_result));

		// check storage status
		let payment_trace = <PaymentTrace<Test>>::get(toVec("Purchased_ID"), AccountId);
		assert_eq!(payment_trace, PaidValue::<Test> {
			amount: calculate_result,
			create_bn: current_bn,
			is_income: true,
		});
		let ask_payment = <AskPeriodPayment<Test>>::get(0, (AccountId, toVec("Purchased_ID")));
		assert_eq!(ask_payment, calculate_result);
	});
}

#[test]
fn test_refund_ask_paid() {
	let mut t = new_test_ext();
	t.execute_with(|| {

		let current_bn: u64 = 1;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId: u64 = 3u64;
		const price_count: u32 = 3;

		// Direct refund when payment is not
		let refund_result: Result<(), Error<Test>> = OcwFinance::refund_ask_paid(toVec("Purchased_ID"));
		assert!(refund_result.is_err());
		assert_eq!(refund_result.err().unwrap(), Error::<Test>::NotFoundPaymentRecord);

		// paid
		assert_eq!(Balances::free_balance(AccountId), 3000000000100);
		OcwFinance::payment_for_ask_quantity(AccountId, toVec("Purchased_ID"), price_count);
		assert_eq!(Balances::free_balance(AccountId), 100);

		// check period income.
		assert_eq!(
			OcwFinance::get_period_income(OcwFinance::make_period_num(current_bn)),
			3000000000000
		);
		assert_eq!(
			OcwFinance::pot(),
			(OcwFinance::account_id(), 3000000000000)
		);

		// check storage status
		let payment_trace = <PaymentTrace<Test>>::get(toVec("Purchased_ID"), AccountId);
		assert_eq!(payment_trace, PaidValue::<Test> {
			amount: 3000000000000,
			create_bn: current_bn,
			is_income: true,
		});
		let ask_payment = <AskPeriodPayment<Test>>::get(0, (AccountId, toVec("Purchased_ID")));
		assert_eq!(ask_payment, 3000000000000);

		// get ask paid fee.
		assert_ok!(OcwFinance::refund_ask_paid(toVec("Purchased_ID")));
		assert_eq!(Balances::free_balance(AccountId), 3000000000100);

		// check storage status
		let payment_trace = <PaymentTrace<Test>>::try_get(toVec("Purchased_ID"), AccountId);
		assert!(payment_trace.is_err());
		let ask_payment = <AskPeriodPayment<Test>>::try_get(0, (AccountId, toVec("Purchased_ID")));
		assert!(ask_payment.is_err());

	});


	t.execute_with(|| {

		let current_bn: u64 = 10;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId: u64 = 3u64;
		const price_count: u32 = 3;

		// Direct refund when payment is not
		let refund_result: Result<(), Error<Test>> = OcwFinance::refund_ask_paid(toVec("Purchased_ID_2"));
		assert!(refund_result.is_err());
		assert_eq!(refund_result.err().unwrap(), Error::<Test>::NotFoundPaymentRecord);

		// paid
		assert_eq!(Balances::free_balance(AccountId), 3000000000100);
		OcwFinance::payment_for_ask_quantity(AccountId, toVec("Purchased_ID"), price_count);
		assert_eq!(Balances::free_balance(AccountId), 100);

	});

	t.execute_with(|| {

		let previous_bn: u64 = 10;
		let current_bn: u64 = 30;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId: u64 = 4u64;
		const price_count: u32 = 4;

		// paid
		assert_eq!(Balances::free_balance(AccountId), 4000000000100);
		OcwFinance::payment_for_ask_quantity(AccountId, toVec("Purchased_ID_NEW"), price_count);
		assert_eq!(Balances::free_balance(AccountId), 100);

		// check period income.
		assert_eq!(
			OcwFinance::get_period_income(OcwFinance::make_period_num(previous_bn)),
			3000000000000
		);

		// check period income.
		assert_eq!(
			OcwFinance::get_period_income(OcwFinance::make_period_num(current_bn)),
			4000000000000
		);
		assert_eq!(
			OcwFinance::pot(),
			(OcwFinance::account_id(), 7000000000000)
		);

	});
}

#[test]
fn test_record_submit_point() {
	new_test_ext().execute_with(|| {

		// let current_bn: u64 = 1;
		// System::set_block_number(current_bn);
		// <OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);
		//
		// const AccountId: u64 = 3u64;
		// const price_count: u32 = 3;

		const AccountId_1: u64 = 1;
		const AccountId_2: u64 = 2;
		const AccountId_3: u64 = 3;
		const AccountId_4: u64 = 4;

		let purchase_id_1 = toVec("PurchaseId_1");
		let purchase_id_2 = toVec("PurchaseId_2");

		assert_eq!(OcwFinance::get_period_point(OcwFinance::make_period_num(3)), 0);
		// who: T::AccountId, p_id: PurchaseId, bn: T::BlockNumber, ask_point: u64sum
		assert_ok!(OcwFinance::record_submit_point(AccountId_1, purchase_id_1.clone(), 1, 9));
		assert_eq!(OcwFinance::record_submit_point(AccountId_1, purchase_id_1.clone(), 1, 9), Err(Error::<Test>::PointRecordIsAlreadyExists) );
		assert_ok!(OcwFinance::record_submit_point(AccountId_2, purchase_id_1.clone(), 3, 10));
		assert_eq!(OcwFinance::get_period_point(OcwFinance::make_period_num(3)), 19, "Get all the record points in the first time zone");

	});
}

#[test]
fn test_check_and_slash_expired_rewards() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		let purchased_submit_bn: u64 = 50;
		let current_bn: u64 = 50;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId_1: u64 = 1;
		const AccountId_2: u64 = 2;
		const AccountId_3: u64 = 3;

		OcwFinance::payment_for_ask_quantity(AccountId_2, toVec("Purchased_ID_BN_55"), 2);
		assert_ok!(OcwFinance::record_submit_point(AccountId_1, toVec("Purchased_ID_BN_55"), purchased_submit_bn ,2 ));
		assert_ok!(OcwFinance::record_submit_point(AccountId_3, toVec("Purchased_ID_BN_55"), purchased_submit_bn ,2 ));

		// check pot
		assert_eq!(OcwFinance::pot(),(OcwFinance::account_id(), 2000000000000));

		// check none
		assert_eq!(OcwFinance::check_and_slash_expired_rewards(OcwFinance::make_period_num(current_bn)), None);

	});


	t.execute_with(|| {
		let purchased_submit_bn: u64 = 50;
		let current_bn: u64 = 90;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId_1: u64 = 1;
		const AccountId_2: u64 = 2;
		const AccountId_3: u64 = 3;

		//
		assert_eq!(
			OcwFinance::take_reward(5, AccountId_1),
			Err(Error::<Test>::RewardPeriodHasExpired)
		);

		// check storage struct.
		assert!(<PaymentTrace<Test>>::contains_key(toVec("Purchased_ID_BN_55"), AccountId_2));
		assert!(<AskPeriodPayment<Test>>::contains_key(5, (AccountId_2, toVec("Purchased_ID_BN_55"))));

		assert_eq!(false, <RewardTrace<Test>>::contains_key(5, AccountId_1));
		assert_eq!(false, <RewardTrace<Test>>::contains_key(5, AccountId_3));
		assert!(<AskPeriodPoint<Test>>::contains_key(5, (AccountId_1, toVec("Purchased_ID_BN_55"))));
		assert!(<AskPeriodPoint<Test>>::contains_key(5, (AccountId_3, toVec("Purchased_ID_BN_55"))));

		assert_eq!(OcwFinance::pot(),(OcwFinance::account_id(), 2000000000000));

		// if reward is expired
		assert_eq!(
			OcwFinance::check_and_slash_expired_rewards(OcwFinance::make_period_num(current_bn)),
			Some(2000000000000),
		);

		// store clean
		assert_eq!(false, <AskPeriodPayment<Test>>::contains_key(5, (AccountId_2, toVec("Purchased_ID_BN_55"))));
		assert_eq!(false, <RewardTrace<Test>>::contains_key(5, AccountId_1));
		assert_eq!(false, <RewardTrace<Test>>::contains_key(5, AccountId_3));
		assert_eq!(false, <AskPeriodPoint<Test>>::contains_key(5, (AccountId_1, toVec("Purchased_ID_BN_55"))));
		assert_eq!(false, <AskPeriodPoint<Test>>::contains_key(5, (AccountId_3, toVec("Purchased_ID_BN_55"))));


		// assert_eq!(Balances::usable_balance(OcwFinance::account_id()),0);
		assert_eq!(OcwFinance::pot(),(OcwFinance::account_id(), 0));

	});


}

#[test]
fn test_take_reward() {
	let mut t = new_test_ext();
	t.execute_with(|| {

		let current_bn: u64 = 50;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId_1: u64 = 1;
		const AccountId_2: u64 = 2;
		const AccountId_3: u64 = 3;

		assert_eq!(
			OcwFinance::take_reward(5, AccountId_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);

		assert_eq!(
			OcwFinance::take_reward(1, AccountId_1),
			Err(Error::<Test>::RewardPeriodHasExpired)
		);


		assert_eq!(
			OcwFinance::take_reward(2, AccountId_1),
			Err(Error::<Test>::NoRewardPoints)
		);

		assert_eq!(
			OcwFinance::take_reward(3, AccountId_1),
			Err(Error::<Test>::NoRewardPoints)
		);

		assert_eq!(
			OcwFinance::take_reward(4, AccountId_1),
			Err(Error::<Test>::NoRewardPoints)
		);
	});


	t.execute_with(|| {

		let current_bn: u64 = 55;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId_1: u64 = 1;
		assert_eq!(Balances::free_balance(AccountId_1), 1000000000100);

		const AccountId_2: u64 = 2;
		assert_eq!(Balances::free_balance(AccountId_2), 2000000000100);

		const AccountId_3: u64 = 3;
		assert_eq!(Balances::free_balance(AccountId_3), 3000000000100);

		// ask paid.
		OcwFinance::payment_for_ask_quantity(AccountId_2, toVec("Purchased_ID_BN_55"), 2);
		assert_eq!(OcwFinance::pot(),(OcwFinance::account_id(), 2000000000000));

		//
		assert_eq!(
			OcwFinance::take_reward(5, AccountId_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);
	});

	t.execute_with(|| {
		let purchased_submit_bn: u64 = 55;
		let current_bn: u64 = 57;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId_1: u64 = 1;
		assert_eq!(Balances::free_balance(AccountId_1), 1000000000100);

		const AccountId_2: u64 = 2;
		assert_eq!(Balances::free_balance(AccountId_2), 100);

		const AccountId_3: u64 = 3;
		assert_eq!(Balances::free_balance(AccountId_3), 3000000000100);

		assert_ok!(OcwFinance::record_submit_point(AccountId_1, toVec("Purchased_ID_BN_55"), purchased_submit_bn ,2 ));
		assert_ok!(OcwFinance::record_submit_point(AccountId_3, toVec("Purchased_ID_BN_55"), purchased_submit_bn ,2 ));

		//
		assert_eq!(
			OcwFinance::take_reward(5, AccountId_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);
	});

	//
	t.execute_with(|| {
		let purchased_submit_bn: u64 = 55;
		let current_bn: u64 = 65;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId_1: u64 = 1;

		assert_eq!(OcwFinance::pot(),(OcwFinance::account_id(), 2000000000000));
		assert_eq!(OcwFinance::take_reward(5, AccountId_1), Ok(2000000000000/2));
		//
		assert_eq!(
			OcwFinance::take_reward(5, AccountId_1),
			Err(Error::<Test>::RewardHasBeenClaimed)
		);
		assert_eq!(OcwFinance::pot(),(OcwFinance::account_id(), 1000000000000));
		assert_eq!(Balances::free_balance(AccountId_1), 2000000000100);
	});

	//
	t.execute_with(|| {
		let purchased_submit_bn: u64 = 55;
		let current_bn: u64 = 75;
		System::set_block_number(current_bn);
		<OcwFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const AccountId_3: u64 = 3;

		assert_eq!(OcwFinance::pot(),(OcwFinance::account_id(), 1000000000000));
		assert_eq!(OcwFinance::take_reward(5, AccountId_3), Ok(1000000000000));

		//
		assert_eq!(
			OcwFinance::take_reward(5, AccountId_3),
			Err(Error::<Test>::RewardHasBeenClaimed)
		);
		assert_eq!(OcwFinance::pot(),(OcwFinance::account_id(), 0));
		assert_eq!(Balances::free_balance(AccountId_3), 4000000000100);
	});
}

#[test]
fn test_get_earliest_reward_period() {
	new_test_ext().execute_with(|| {

		assert_eq!(OcwFinance::get_earliest_reward_period(1), 0);
		assert_eq!(OcwFinance::get_earliest_reward_period(5), 0);
		assert_eq!(OcwFinance::get_earliest_reward_period(10), 0);
		assert_eq!(OcwFinance::get_earliest_reward_period(15), 0);
		assert_eq!(OcwFinance::get_earliest_reward_period(20), 0);
		assert_eq!(OcwFinance::get_earliest_reward_period(25), 0);
		assert_eq!(OcwFinance::get_earliest_reward_period(30), 1 - 1, "Because of the `RewardSlot` so `-1` ");
		assert_eq!(OcwFinance::get_earliest_reward_period(35), 1 - 1);
		assert_eq!(OcwFinance::get_earliest_reward_period(40), 2 - 1);
		assert_eq!(OcwFinance::get_earliest_reward_period(45), 2 - 1);
		assert_eq!(OcwFinance::get_earliest_reward_period(50), 3 - 1);
		assert_eq!(OcwFinance::get_earliest_reward_period(55), 3 - 1);
		assert_eq!(OcwFinance::get_earliest_reward_period(60), 4 - 1);
		assert_eq!(OcwFinance::get_earliest_reward_period(65), 4 - 1);
	});
}

#[test]
fn test_make_period_num() {
	new_test_ext().execute_with(|| {
		assert_eq!(OcwFinance::make_period_num(0), 0);
		assert_eq!(OcwFinance::make_period_num(5), 0);
		assert_eq!(OcwFinance::make_period_num(10), 1);
		assert_eq!(OcwFinance::make_period_num(15), 1);
		assert_eq!(OcwFinance::make_period_num(20), 2);
		assert_eq!(OcwFinance::make_period_num(25), 2);
		assert_eq!(OcwFinance::make_period_num(30), 3);
	});
}