use crate::{mock::*, Error, PaymentTrace, AskPeriodPayment, RewardTrace, AskPeriodPoint, RewardPeriod};
use frame_support::{assert_ok};
use crate::traits::*;
use crate::types::*;
use frame_support::traits::OnInitialize;

#[test]
fn test_it_works_for_default_value() {
	new_test_ext().execute_with(|| {

		let calculate_result = OracleFinance::calculate_fee_of_ask_quantity(3);
		assert_eq!(calculate_result, 3u64.saturating_mul(DOLLARS));
	});
}

#[test]
fn test_reserve_for_ask_quantity() {
	new_test_ext().execute_with(|| {

		let current_bn: u64 = 1;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID: u64 = 3u64;
		const PRICE_COUNT: u32 = 3;

		let calculate_result = OracleFinance::calculate_fee_of_ask_quantity(PRICE_COUNT);
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 3000000000100);
		let payment_result = OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID, to_test_vec("Purchased_ID"), PRICE_COUNT);
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 100);
		assert_eq!(Balances::reserved_balance(ACCOUNT_ID), 3000000000000);
		assert_eq!(payment_result, OcwPaymentResult::Success(to_test_vec("Purchased_ID"), calculate_result));

		// check storage status
		let payment_trace = <PaymentTrace<Test>>::get(to_test_vec("Purchased_ID"), ACCOUNT_ID);
		assert_eq!(payment_trace, PaidValue::<Test> {
			amount: calculate_result,
			create_bn: current_bn,
			is_income: true,
		});
		// let ask_payment = <AskPeriodPayment<Test>>::get(0, (AccountId, to_test_vec("Purchased_ID")));
		// assert_eq!(ask_payment, calculate_result);
	});
}

#[test]
fn test_unreserve_ask() {
	let mut t = new_test_ext();
	t.execute_with(|| {

		let current_bn: u64 = 1;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID: u64 = 3u64;
		const PRICE_COUNT: u32 = 3;

		// Direct refund when payment is not
		let refund_result: Result<(), Error<Test>> = OracleFinance::unreserve_ask(to_test_vec("Purchased_ID"));
		assert!(refund_result.is_err());
		assert_eq!(refund_result.err().unwrap(), Error::<Test>::NotFoundPaymentRecord);

		// paid
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 3000000000100);
		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID, to_test_vec("Purchased_ID"), PRICE_COUNT);
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 100);
		assert_eq!(Balances::reserved_balance(ACCOUNT_ID), 3000000000000);

		// check period income.
		assert_eq!(
			OracleFinance::get_period_income(OracleFinance::make_period_num(current_bn)),
			0
		);
		assert_eq!(
			OracleFinance::pot(),
			(OracleFinance::account_id(), 0)
		);

		// check storage status
		let payment_trace = <PaymentTrace<Test>>::get(to_test_vec("Purchased_ID"), ACCOUNT_ID);
		assert_eq!(payment_trace, PaidValue::<Test> {
			amount: 3000000000000,
			create_bn: current_bn,
			is_income: true,
		});

		let ask_payment = <AskPeriodPayment<Test>>::get(0, (ACCOUNT_ID, to_test_vec("Purchased_ID")));
		assert_eq!(ask_payment, 0, "only reserve so payment still be 0.");

		// get ask paid fee.
		assert_ok!(OracleFinance::unreserve_ask(to_test_vec("Purchased_ID")));
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 3000000000100);

		// check storage status
		let payment_trace = <PaymentTrace<Test>>::try_get(to_test_vec("Purchased_ID"), ACCOUNT_ID);
		assert!(payment_trace.is_err());
		let ask_payment = <AskPeriodPayment<Test>>::try_get(0, (ACCOUNT_ID, to_test_vec("Purchased_ID")));
		assert!(ask_payment.is_err());

	});


	t.execute_with(|| {

		let current_bn: u64 = 10;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID: u64 = 3u64;
		const PRICE_COUNT: u32 = 3;

		// Direct refund when payment is not
		let refund_result: Result<(), Error<Test>> = OracleFinance::unreserve_ask(to_test_vec("Purchased_ID_2"));
		assert!(refund_result.is_err());
		assert_eq!(refund_result.err().unwrap(), Error::<Test>::NotFoundPaymentRecord);

		// paid
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 3000000000100);
		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID, to_test_vec("Purchased_ID"), PRICE_COUNT);
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 100);

	});

	t.execute_with(|| {

		let previous_bn: u64 = 10;
		let current_bn: u64 = 30;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID: u64 = 4u64;
		const PRICE_COUNT: u32 = 4;

		// paid
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 4000000000100);
		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID, to_test_vec("Purchased_ID_NEW"), PRICE_COUNT);
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 100);

		// check period income.
		assert_eq!(
			OracleFinance::get_period_income(OracleFinance::make_period_num(previous_bn)),
			0
		);

		// check period income.
		assert_eq!(
			OracleFinance::get_period_income(OracleFinance::make_period_num(current_bn)),
			0
		);
		assert_eq!(
			OracleFinance::pot(),
			(OracleFinance::account_id(), 0) // because use reserve
		);

	});
}

#[test]
fn test_record_submit_point() {
	new_test_ext().execute_with(|| {

		// let current_bn: u64 = 1;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);
		//
		// const AccountId: u64 = 3u64;
		// const price_count: u32 = 3;

		const ACCOUNT_ID_1: u64 = 1;
		const ACCOUNT_ID_2: u64 = 2;
		// const AccountId_3: u64 = 3;
		// const AccountId_4: u64 = 4;

		let purchase_id_1 = to_test_vec("PurchaseId_1");
		// let purchase_id_2 = to_test_vec("PurchaseId_2");

		assert_eq!(OracleFinance::get_period_point(OracleFinance::make_period_num(3)), 0);
		// who: T::AccountId, p_id: PurchaseId, bn: T::BlockNumber, ask_point: u64sum
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_1, purchase_id_1.clone(), 1, 9));
		assert_eq!(OracleFinance::record_submit_point(ACCOUNT_ID_1, purchase_id_1.clone(), 1, 9), Err(Error::<Test>::PointRecordIsAlreadyExists) );
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_2, purchase_id_1.clone(), 3, 10));
		assert_eq!(OracleFinance::get_period_point(OracleFinance::make_period_num(3)), 19, "Get all the record points in the first time zone");
		assert_eq!(<RewardPeriod<Test>>::get(ACCOUNT_ID_1), vec![
			(0, 9, purchase_id_1.clone())
		]);
		assert_eq!(<RewardPeriod<Test>>::get(ACCOUNT_ID_2), vec![
			(0, 10, purchase_id_1.clone())
		]);

	});
}

#[test]
fn test_check_and_slash_expired_rewards() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		let purchased_submit_bn: u64 = 50;
		let current_bn: u64 = 50;
		System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID_1: u64 = 1;
		const ACCOUNT_ID_2: u64 = 2;
		const ACCOUNT_ID_3: u64 = 3;

		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID_2, to_test_vec("Purchased_ID_BN_55"), 2);
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_55"), purchased_submit_bn ,2 ));
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_55"), purchased_submit_bn ,2 ));
		assert_ok!(OracleFinance::pay_to_ask(to_test_vec("Purchased_ID_BN_55")));
		// check pot
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));

		assert_eq!(<RewardPeriod<Test>>::get(ACCOUNT_ID_1), vec![
			(5, 2, to_test_vec("Purchased_ID_BN_55"))
		]);
		assert_eq!(<RewardPeriod<Test>>::get(ACCOUNT_ID_3), vec![
			(5, 2, to_test_vec("Purchased_ID_BN_55"))
		]);

		// check none
		assert_eq!(OracleFinance::check_and_slash_expired_rewards(OracleFinance::make_period_num(current_bn)), None);

		// let current_period_num = OracleFinance::make_period_num(current_bn);
		// check none
		// assert_eq!(OracleFinance::check_and_slash_expired_rewards(
		// 	current_period_num + RewardPeriodCycle::get() + RewardSlot::get() + 1
		// ), Some(2000000000000));
		//
		// // check pot
		// assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 0));

	});


	t.execute_with(|| {
		let _purchased_submit_bn: u64 = 50;
		let current_bn: u64 = 90;
		System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID_1: u64 = 1;
		const ACCOUNT_ID_2: u64 = 2;
		const ACCOUNT_ID_3: u64 = 3;

		//
		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardPeriodHasExpired)
		);

		// check storage struct.
		assert!(<PaymentTrace<Test>>::contains_key(to_test_vec("Purchased_ID_BN_55"), ACCOUNT_ID_2));
		assert!(<AskPeriodPayment<Test>>::contains_key(5, (ACCOUNT_ID_2, to_test_vec("Purchased_ID_BN_55"))));

		assert_eq!(false, <RewardTrace<Test>>::contains_key(5, ACCOUNT_ID_1));
		assert_eq!(false, <RewardTrace<Test>>::contains_key(5, ACCOUNT_ID_3));
		assert!(<AskPeriodPoint<Test>>::contains_key(5, (ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_55"))));
		assert!(<AskPeriodPoint<Test>>::contains_key(5, (ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_55"))));

		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));

		// if reward is expired
		assert_eq!(
			OracleFinance::check_and_slash_expired_rewards(OracleFinance::make_period_num(current_bn)),
			Some(2000000000000),
		);

		// store clean
		assert_eq!(false, <AskPeriodPayment<Test>>::contains_key(5, (ACCOUNT_ID_2, to_test_vec("Purchased_ID_BN_55"))));
		assert_eq!(false, <RewardTrace<Test>>::contains_key(5, ACCOUNT_ID_1));
		assert_eq!(false, <RewardTrace<Test>>::contains_key(5, ACCOUNT_ID_3));
		assert_eq!(false, <AskPeriodPoint<Test>>::contains_key(5, (ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_55"))));
		assert_eq!(false, <AskPeriodPoint<Test>>::contains_key(5, (ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_55"))));


		// assert_eq!(Balances::usable_balance(OracleFinance::account_id()),0);
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 0));

		assert_eq!(<RewardPeriod<Test>>::get(ACCOUNT_ID_1), vec![]);
		assert_eq!(<RewardPeriod<Test>>::get(ACCOUNT_ID_3), vec![]);

	});


}

#[test]
fn test_take_reward() {
	let mut t = new_test_ext();
	t.execute_with(|| {

		let current_bn: u64 = 50;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID_1: u64 = 1;
		// const AccountId_2: u64 = 2;
		// const ACCOUNT_ID_3: u64 = 3;

		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);

		assert_eq!(
			OracleFinance::take_reward(1, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardPeriodHasExpired)
		);


		assert_eq!(
			OracleFinance::take_reward(2, ACCOUNT_ID_1),
			Err(Error::<Test>::NoRewardPoints)
		);

		assert_eq!(
			OracleFinance::take_reward(3, ACCOUNT_ID_1),
			Err(Error::<Test>::NoRewardPoints)
		);

		assert_eq!(
			OracleFinance::take_reward(4, ACCOUNT_ID_1),
			Err(Error::<Test>::NoRewardPoints)
		);
	});


	t.execute_with(|| {

		let current_bn: u64 = 55;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID_1: u64 = 1;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_1), 1000000000100);

		const ACCOUNT_ID_2: u64 = 2;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_2), 2000000000100);

		const ACCOUNT_ID_3: u64 = 3;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_3), 3000000000100);

		// ask paid.
		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID_2, to_test_vec("Purchased_ID_BN_55"), 2);
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 0));
		assert_ok!(OracleFinance::pay_to_ask(to_test_vec("Purchased_ID_BN_55")));
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));


		//
		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);
	});

	t.execute_with(|| {
		let purchased_submit_bn: u64 = 55;
		let current_bn: u64 = 57;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID_1: u64 = 1;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_1), 1000000000100);

		const ACCOUNT_ID_2: u64 = 2;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_2), 100);

		const ACCOUNT_ID_3: u64 = 3;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_3), 3000000000100);

		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_55"), purchased_submit_bn ,2 ));
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_55"), purchased_submit_bn ,2 ));

		//
		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);
	});

	//
	t.execute_with(|| {
		let _purchased_submit_bn: u64 = 55;
		let current_bn: u64 = 65;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID_1: u64 = 1;

		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));
		assert_eq!(OracleFinance::take_reward(5, ACCOUNT_ID_1), Ok(2000000000000/2));
		//
		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardHasBeenClaimed)
		);
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 1000000000000));
		assert_eq!(Balances::free_balance(ACCOUNT_ID_1), 2000000000100);
	});

	//
	t.execute_with(|| {
		let _purchased_submit_bn: u64 = 55;
		let current_bn: u64 = 75;
		System::set_block_number(current_bn);
		<OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID_3: u64 = 3;

		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 1000000000000));
		assert_eq!(OracleFinance::take_reward(5, ACCOUNT_ID_3), Ok(1000000000000));

		//
		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_3),
			Err(Error::<Test>::RewardHasBeenClaimed)
		);
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 0));
		assert_eq!(Balances::free_balance(ACCOUNT_ID_3), 4000000000100);
	});
}

#[test]
fn test_get_earliest_reward_period() {
	new_test_ext().execute_with(|| {

		assert_eq!(OracleFinance::get_earliest_reward_period(1), 0);
		assert_eq!(OracleFinance::get_earliest_reward_period(5), 0);
		assert_eq!(OracleFinance::get_earliest_reward_period(10), 0);
		assert_eq!(OracleFinance::get_earliest_reward_period(15), 0);
		assert_eq!(OracleFinance::get_earliest_reward_period(20), 0);
		assert_eq!(OracleFinance::get_earliest_reward_period(25), 0);
		assert_eq!(OracleFinance::get_earliest_reward_period(30), 1 - 1, "Because of the `RewardSlot` so `-1` ");
		assert_eq!(OracleFinance::get_earliest_reward_period(35), 1 - 1);
		assert_eq!(OracleFinance::get_earliest_reward_period(40), 2 - 1);
		assert_eq!(OracleFinance::get_earliest_reward_period(45), 2 - 1);
		assert_eq!(OracleFinance::get_earliest_reward_period(50), 3 - 1);
		assert_eq!(OracleFinance::get_earliest_reward_period(55), 3 - 1);
		assert_eq!(OracleFinance::get_earliest_reward_period(60), 4 - 1);
		assert_eq!(OracleFinance::get_earliest_reward_period(65), 4 - 1);
	});
}

#[test]
fn test_make_period_num() {
	new_test_ext().execute_with(|| {
		assert_eq!(OracleFinance::make_period_num(0), 0);
		assert_eq!(OracleFinance::make_period_num(5), 0);
		assert_eq!(OracleFinance::make_period_num(10), 1);
		assert_eq!(OracleFinance::make_period_num(15), 1);
		assert_eq!(OracleFinance::make_period_num(20), 2);
		assert_eq!(OracleFinance::make_period_num(25), 2);
		assert_eq!(OracleFinance::make_period_num(30), 3);
	});
}