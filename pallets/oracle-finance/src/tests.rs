use crate::{mock::*, Error, PaymentTrace, AskEraPayment, RewardTrace, AskEraPoint, RewardEra};
use frame_support::{assert_ok};
use crate::traits::*;
use crate::types::*;

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

		// let current_bn: u64 = 1;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		advance_session();
		advance_session();
		advance_session();
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 20 );
		assert_eq!(Session::current_index(), 4 );
		assert_eq!(OracleFinance::current_era(), Some(1));
		assert_eq!(OracleFinance::eras_start_session_index(1), Some(4));

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
		assert_eq!(payment_trace, PaidValue::<BlockNumber,Balance, EraIndex> {
			amount: calculate_result,
			create_bn: <frame_system::Pallet<Test>>::block_number(),
			is_income: true,
			paid_era: 1,
		});
		// let ask_payment = <AskEraPayment<Test>>::get(0, (AccountId, to_test_vec("Purchased_ID")));
		// assert_eq!(ask_payment, calculate_result);
	});
}

#[test]
fn test_unreserve_ask() {
	let mut t = new_test_ext();
	t.execute_with(|| {

		// let current_bn: u64 = 1;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		advance_session();
		advance_session();
		advance_session();
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 20 );
		assert_eq!(Session::current_index(), 4 );
		assert_eq!(OracleFinance::current_era(), Some(1));
		assert_eq!(OracleFinance::eras_start_session_index(1), Some(4));

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

		// check era income.
		assert_eq!(
			OracleFinance::get_era_income(OracleFinance::current_era_num()),
			0
		);
		assert_eq!(
			OracleFinance::pot(),
			(OracleFinance::account_id(), 0)
		);

		// check storage status
		let payment_trace = <PaymentTrace<Test>>::get(to_test_vec("Purchased_ID"), ACCOUNT_ID);
		assert_eq!(payment_trace, PaidValue::<BlockNumber, Balance, EraIndex> {
			amount: 3000000000000,
			create_bn: <frame_system::Pallet<Test>>::block_number(),
			is_income: true,
			paid_era: 1,
		});

		let ask_payment = <AskEraPayment<Test>>::get(0, (ACCOUNT_ID, to_test_vec("Purchased_ID")));
		assert_eq!(ask_payment, 0, "only reserve so payment still be 0.");

		// get ask paid fee.
		assert_ok!(OracleFinance::unreserve_ask(to_test_vec("Purchased_ID")));
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 3000000000100);

		// check storage status
		let payment_trace = <PaymentTrace<Test>>::try_get(to_test_vec("Purchased_ID"), ACCOUNT_ID);
		assert!(payment_trace.is_err());
		let ask_payment = <AskEraPayment<Test>>::try_get(0, (ACCOUNT_ID, to_test_vec("Purchased_ID")));
		assert!(ask_payment.is_err());

	});


	t.execute_with(|| {

		// let current_bn: u64 = 10;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 20 );
		assert_eq!(Session::current_index(), 4 );
		assert_eq!(OracleFinance::current_era(), Some(1));
		assert_eq!(OracleFinance::eras_start_session_index(1), Some(4));

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

		// let previous_bn: u64 = 10;
		// let current_bn: u64 = 30;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 20 );
		assert_eq!(Session::current_index(), 4 );
		assert_eq!(OracleFinance::current_era(), Some(1));
		assert_eq!(OracleFinance::eras_start_session_index(1), Some(4));

		const ACCOUNT_ID: u64 = 4u64;
		const PRICE_COUNT: u32 = 4;

		// paid
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 4000000000100);
		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID, to_test_vec("Purchased_ID_NEW"), PRICE_COUNT);
		assert_eq!(Balances::free_balance(ACCOUNT_ID), 100);

		assert_eq!(OracleFinance::current_era_num(), 1);
		// check era income.
		assert_eq!(
			OracleFinance::get_era_income(OracleFinance::current_era_num()),
			0
		);

		// check era income.
		assert_eq!(
			OracleFinance::get_era_income(OracleFinance::current_era_num()-1),
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

		advance_session();
		advance_session();
		advance_session();
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 20 );
		assert_eq!(Session::current_index(), 4 );
		assert_eq!(OracleFinance::current_era(), Some(1));
		assert_eq!(OracleFinance::eras_start_session_index(1), Some(4));

		const ACCOUNT_ID_1: u64 = 1;
		const ACCOUNT_ID_2: u64 = 2;

		let purchase_id_1 = to_test_vec("PurchaseId_1");
		// let purchase_id_2 = to_test_vec("PurchaseId_2");

		assert_eq!(OracleFinance::get_era_point(OracleFinance::current_era_num()), 0);
		// who: T::AccountId, p_id: PurchaseId, bn: T::BlockNumber, ask_point: u64sum
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_1, purchase_id_1.clone(), 20, 9));
		assert_eq!(OracleFinance::record_submit_point(ACCOUNT_ID_1, purchase_id_1.clone(), 21, 9), Err(Error::<Test>::PointRecordIsAlreadyExists) );
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_2, purchase_id_1.clone(), 21, 10));
		assert_eq!(OracleFinance::get_era_point(OracleFinance::current_era_num()), 19, "Get all the record points in the first time zone");
		assert_eq!(<RewardEra<Test>>::get(ACCOUNT_ID_1), vec![
			(1, 9, purchase_id_1.clone())
		]);
		assert_eq!(<RewardEra<Test>>::get(ACCOUNT_ID_2), vec![
			(1, 10, purchase_id_1.clone())
		]);

	});
}

#[test]
fn test_check_and_slash_expired_rewards() {
	let mut t = new_test_ext();
	t.execute_with(|| {
		// let purchased_submit_bn: u64 = 50;
		// let current_bn: u64 = 50;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 40 );
		assert_eq!(Session::current_index(), 8 );
		assert_eq!(OracleFinance::current_era(), Some(3));
		assert_eq!(OracleFinance::eras_start_session_index(3), Some(8));

		const ACCOUNT_ID_1: u64 = 1;
		const ACCOUNT_ID_2: u64 = 2;
		const ACCOUNT_ID_3: u64 = 3;

		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID_2, to_test_vec("Purchased_ID_BN_55"), 2);
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_55"), <frame_system::Pallet<Test>>::block_number() ,2 ));
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_55"), <frame_system::Pallet<Test>>::block_number() ,2 ));
		assert_ok!(OracleFinance::pay_to_ask(to_test_vec("Purchased_ID_BN_55"), 2));
		// check pot
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));

		assert_eq!(<RewardEra<Test>>::get(ACCOUNT_ID_1), vec![
			(3, 2, to_test_vec("Purchased_ID_BN_55"))
		]);
		assert_eq!(<RewardEra<Test>>::get(ACCOUNT_ID_3), vec![
			(3, 2, to_test_vec("Purchased_ID_BN_55"))
		]);

		println!(" current {}, depth {}", OracleFinance::current_era_num(), HistoryDepth::get());
		// count check_ear
		let check_era = OracleFinance::current_era_num() - HistoryDepth::get() - 1;

		// check none
		assert_eq!(OracleFinance::check_and_slash_expired_rewards(check_era), None);

	});


	t.execute_with(|| {
		// let _purchased_submit_bn: u64 = 50;
		// let current_bn: u64 = 90;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		const ACCOUNT_ID_1: u64 = 1;
		const ACCOUNT_ID_2: u64 = 2;
		const ACCOUNT_ID_3: u64 = 3;


		// check storage struct.
		assert!(<PaymentTrace<Test>>::contains_key(to_test_vec("Purchased_ID_BN_55"), ACCOUNT_ID_2));
		assert!(<AskEraPayment<Test>>::contains_key(3, (ACCOUNT_ID_2, to_test_vec("Purchased_ID_BN_55"))));

		assert_eq!(false, <RewardTrace<Test>>::contains_key(3, ACCOUNT_ID_1));
		assert_eq!(false, <RewardTrace<Test>>::contains_key(3, ACCOUNT_ID_3));
		assert!(<AskEraPoint<Test>>::contains_key(3, (ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_55"))));
		assert!(<AskEraPoint<Test>>::contains_key(3, (ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_55"))));

		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));

		// if reward is expired
		// assert_eq!(
		// 	OracleFinance::check_and_slash_expired_rewards(OracleFinance::current_era_num()),
		// 	Some(2000000000000),
		// );

		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 70 );
		assert_eq!(Session::current_index(), 14 );
		assert_eq!(OracleFinance::current_era(), Some(6));
		assert_eq!(OracleFinance::eras_start_session_index(6), Some(14));

		//
		assert_eq!(
			OracleFinance::take_reward(3, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardEraHasExpired)
		);

		// store clean
		assert_eq!(false, <AskEraPayment<Test>>::contains_key(3, (ACCOUNT_ID_2, to_test_vec("Purchased_ID_BN_55"))));
		assert_eq!(false, <RewardTrace<Test>>::contains_key(3, ACCOUNT_ID_1));
		assert_eq!(false, <RewardTrace<Test>>::contains_key(3, ACCOUNT_ID_3));
		assert_eq!(false, <AskEraPoint<Test>>::contains_key(3, (ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_55"))));
		assert_eq!(false, <AskEraPoint<Test>>::contains_key(3, (ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_55"))));

		// assert_eq!(Balances::usable_balance(OracleFinance::account_id()),0);
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 0));
		assert_eq!(<RewardEra<Test>>::get(ACCOUNT_ID_1), vec![]);
		assert_eq!(<RewardEra<Test>>::get(ACCOUNT_ID_3), vec![]);

	});


}

#[test]
fn test_take_reward() {
	let mut t = new_test_ext();
	t.execute_with(|| {

		// let current_bn: u64 = 50;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 5 );
		assert_eq!(Session::current_index(), 1 );
		assert_eq!(OracleFinance::eras_start_session_index(0), Some(2));
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 10 );
		assert_eq!(Session::current_index(), 2 );
		assert_eq!(OracleFinance::eras_start_session_index(0), Some(2));
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 15 );
		assert_eq!(Session::current_index(), 3 );
		assert_eq!(OracleFinance::eras_start_session_index(1), Some(4));
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 20 );
		assert_eq!(Session::current_index(), 4 );
		assert_eq!(OracleFinance::eras_start_session_index(1), Some(4));
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 25 );
		assert_eq!(Session::current_index(), 5 );
		assert_eq!(OracleFinance::eras_start_session_index(2), Some(6));
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 30 );
		assert_eq!(Session::current_index(), 6 );
		assert_eq!(OracleFinance::eras_start_session_index(2), Some(6));
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 35 );
		assert_eq!(Session::current_index(), 7 );
		assert_eq!(OracleFinance::eras_start_session_index(3), Some(8));
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 40 );
		assert_eq!(Session::current_index(), 8 );
		assert_eq!(OracleFinance::eras_start_session_index(3), Some(8));
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 45 );
		assert_eq!(Session::current_index(), 9 );
		assert_eq!(OracleFinance::eras_start_session_index(4), Some(10));
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 50 );
		assert_eq!(Session::current_index(), 10 );
		assert_eq!(OracleFinance::current_era(), Some(4));
		assert_eq!(OracleFinance::eras_start_session_index(4), Some(10));

		const ACCOUNT_ID_1: u64 = 1;

		assert_eq!(
			OracleFinance::take_reward(10, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);

		assert_eq!(
			OracleFinance::take_reward(1, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardEraHasExpired)
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
			Err(Error::<Test>::RewardSlotNotExpired)
		);
	});


	t.execute_with(|| {

		// let current_bn: u64 = 55;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);


		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 55 );
		assert_eq!(Session::current_index(), 11 );
		assert_eq!(OracleFinance::current_era(), Some(5));
		assert_eq!(OracleFinance::eras_start_session_index(5), Some(12));

		const ACCOUNT_ID_1: u64 = 1;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_1), 1000000000100);

		const ACCOUNT_ID_2: u64 = 2;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_2), 2000000000100);

		const ACCOUNT_ID_3: u64 = 3;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_3), 3000000000100);

		// ask paid.
		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID_2, to_test_vec("Purchased_ID_BN_55"), 2);
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 0));
		assert_ok!(OracleFinance::pay_to_ask(to_test_vec("Purchased_ID_BN_55"), 2));
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));

		//
		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);
	});

	t.execute_with(|| {
		// let purchased_submit_bn: u64 = 55;
		// let current_bn: u64 = 57;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 60 );
		assert_eq!(Session::current_index(), 12 );
		assert_eq!(OracleFinance::current_era(), Some(5));
		assert_eq!(OracleFinance::eras_start_session_index(5), Some(12));

		const ACCOUNT_ID_1: u64 = 1;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_1), 1000000000100);

		const ACCOUNT_ID_2: u64 = 2;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_2), 100);

		const ACCOUNT_ID_3: u64 = 3;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_3), 3000000000100);

		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_55"), <frame_system::Pallet<Test>>::block_number() ,2 ));
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_55"), <frame_system::Pallet<Test>>::block_number() ,2 ));

		//
		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);
	});

	//
	t.execute_with(|| {

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 65 );
		assert_eq!(Session::current_index(), 13 );
		assert_eq!(OracleFinance::current_era(), Some(6));
		assert_eq!(OracleFinance::eras_start_session_index(6), Some(14));

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

		advance_session();
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 75 );
		assert_eq!(Session::current_index(), 15 );
		assert_eq!(OracleFinance::current_era(), Some(7));
		assert_eq!(OracleFinance::eras_start_session_index(7), Some(16));

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
fn test_take_full_ear_reward() {
	let mut t = new_test_ext();

	t.execute_with(|| {

		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 55 );
		assert_eq!(Session::current_index(), 11 );
		assert_eq!(OracleFinance::current_era(), Some(5));
		assert_eq!(OracleFinance::eras_start_session_index(5), Some(12));

		const ACCOUNT_ID_1: u64 = 1;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_1), 1000000000100);

		const ACCOUNT_ID_2: u64 = 2;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_2), 2000000000100);

		const ACCOUNT_ID_3: u64 = 3;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_3), 3000000000100);



		// ask paid.
		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID_2, to_test_vec("Purchased_ID_BN_55"), 2);
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 0));
		assert_ok!(OracleFinance::pay_to_ask(to_test_vec("Purchased_ID_BN_55"), 2));
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));

		//
		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);
	});

	t.execute_with(|| {
		// let purchased_submit_bn: u64 = 55;
		// let current_bn: u64 = 57;
		// System::set_block_number(current_bn);
		// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 60 );
		assert_eq!(Session::current_index(), 12 );
		assert_eq!(OracleFinance::current_era(), Some(5));
		assert_eq!(OracleFinance::eras_start_session_index(5), Some(12));

		const ACCOUNT_ID_1: u64 = 1;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_1), 1000000000100);

		const ACCOUNT_ID_2: u64 = 2;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_2), 100);

		const ACCOUNT_ID_3: u64 = 3;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_3), 3000000000100);

		const ACCOUNT_ID_4: u64 = 4;
		assert_eq!(Balances::free_balance(ACCOUNT_ID_4), 4000000000100);

		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_55"), <frame_system::Pallet<Test>>::block_number() ,2 ));
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_55"), <frame_system::Pallet<Test>>::block_number() ,2 ));

		//
		assert_eq!(
			OracleFinance::take_reward(5, ACCOUNT_ID_1),
			Err(Error::<Test>::RewardSlotNotExpired)
		);

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 65 );
		assert_eq!(Session::current_index(), 13 );
		assert_eq!(OracleFinance::current_era(), Some(6));
		assert_eq!(OracleFinance::eras_start_session_index(6), Some(14));

		OracleFinance::reserve_for_ask_quantity(ACCOUNT_ID_4, to_test_vec("Purchased_ID_BN_66"), 2);
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));
		assert_ok!(OracleFinance::pay_to_ask(to_test_vec("Purchased_ID_BN_66"), 2));
		assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 4000000000000));

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 70 );
		assert_eq!(Session::current_index(), 14 );
		assert_eq!(OracleFinance::current_era(), Some(6));
		assert_eq!(OracleFinance::eras_start_session_index(6), Some(14));

		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_1, to_test_vec("Purchased_ID_BN_66"), <frame_system::Pallet<Test>>::block_number() ,2 ));
		assert_ok!(OracleFinance::record_submit_point(ACCOUNT_ID_3, to_test_vec("Purchased_ID_BN_66"), <frame_system::Pallet<Test>>::block_number() ,2 ));


		let reward_list = RewardEra::<Test>::get(ACCOUNT_ID_1);
		assert_eq!(reward_list.len(), 2);
		let reward_list = RewardEra::<Test>::get(ACCOUNT_ID_3);
		assert_eq!(reward_list.len(), 2);
		// println!("reward_list == {:?}", reward_list);

		assert_ok!(
			OracleFinance::take_all_purchase_reward(Origin::signed(ACCOUNT_ID_1)),
		);

		let reward_list = RewardEra::<Test>::get(ACCOUNT_ID_1);
		assert_eq!(reward_list.len(), 1);
		let reward_list = RewardEra::<Test>::get(ACCOUNT_ID_3);
		assert_eq!(reward_list.len(), 2);
		// println!("reward_list == {:?}", reward_list);

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 75 );
		assert_eq!(Session::current_index(), 15 );
		assert_eq!(OracleFinance::current_era(), Some(7));
		assert_eq!(OracleFinance::eras_start_session_index(7), Some(16));

		assert_ok!(
			OracleFinance::take_all_purchase_reward(Origin::signed(ACCOUNT_ID_1)),
		);

		let reward_list = RewardEra::<Test>::get(ACCOUNT_ID_1);
		assert_eq!(reward_list.len(), 0);
		let reward_list = RewardEra::<Test>::get(ACCOUNT_ID_3);
		assert_eq!(reward_list.len(), 2);
		println!("reward_list == {:?}", reward_list);
	});

	// //
	// t.execute_with(|| {
	// 	// let _purchased_submit_bn: u64 = 55;
	// 	// let current_bn: u64 = 65;
	// 	// System::set_block_number(current_bn);
	// 	// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);
	//
	// 	const ACCOUNT_ID_1: u64 = 1;
	//
	// 	assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 2000000000000));
	// 	// assert_eq!(OracleFinance::take_reward(5, ACCOUNT_ID_1), Ok(2000000000000/2));
	// 	//
	// 	assert_eq!(
	// 		OracleFinance::take_reward(5, ACCOUNT_ID_1),
	// 		Err(Error::<Test>::RewardHasBeenClaimed)
	// 	);
	// 	assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 1000000000000));
	// 	assert_eq!(Balances::free_balance(ACCOUNT_ID_1), 2000000000100);
	// });
	//
	// //
	// t.execute_with(|| {
	// 	// let _purchased_submit_bn: u64 = 55;
	// 	// let current_bn: u64 = 75;
	// 	// System::set_block_number(current_bn);
	// 	// <OracleFinance as OnInitialize<u64>>::on_initialize(current_bn);
	//
	// 	advance_session();
	// 	advance_session();
	// 	assert_eq!(<frame_system::Pallet<Test>>::block_number(), 75 );
	// 	assert_eq!(Session::current_index(), 15 );
	// 	assert_eq!(OracleFinance::current_era(), Some(7));
	// 	assert_eq!(OracleFinance::eras_start_session_index(7), Some(16));
	//
	// 	const ACCOUNT_ID_3: u64 = 3;
	//
	// 	assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 1000000000000));
	// 	assert_eq!(OracleFinance::take_reward(5, ACCOUNT_ID_3), Ok(1000000000000));
	//
	// 	//
	// 	assert_eq!(
	// 		OracleFinance::take_reward(5, ACCOUNT_ID_3),
	// 		Err(Error::<Test>::RewardHasBeenClaimed)
	// 	);
	// 	assert_eq!(OracleFinance::pot(),(OracleFinance::account_id(), 0));
	// 	assert_eq!(Balances::free_balance(ACCOUNT_ID_3), 4000000000100);
	// });
}

#[test]
fn test_get_earliest_reward_era() {
	new_test_ext().execute_with(|| {

		advance_session();
		assert_eq!(OracleFinance::get_earliest_reward_era(), None);
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 0);
		assert_eq!(OracleFinance::get_earliest_reward_era(), None);
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 1);
		assert_eq!(OracleFinance::get_earliest_reward_era(), None);
		advance_session();
		assert_eq!(OracleFinance::get_earliest_reward_era(), None);
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 2);
		assert_eq!(OracleFinance::get_earliest_reward_era(), Some(0));
		advance_session();
		assert_eq!(OracleFinance::get_earliest_reward_era(), Some(0));
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 3);
		assert_eq!(OracleFinance::get_earliest_reward_era(), Some(1), " 3 - 2 ");
		advance_session();
		assert_eq!(OracleFinance::get_earliest_reward_era(), Some(1));
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 4);
		assert_eq!(OracleFinance::get_earliest_reward_era(), Some(2));
		advance_session();
		assert_eq!(OracleFinance::get_earliest_reward_era(), Some(2));
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 5);
		assert_eq!(OracleFinance::get_earliest_reward_era(), Some(3));
		advance_session();
		assert_eq!(OracleFinance::get_earliest_reward_era(), Some(3));
	});
}

#[test]
fn test_current_era_num() {
	new_test_ext().execute_with(|| {
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 5 );
		assert_eq!(Session::current_index(), 1 );
		assert_eq!(OracleFinance::current_era_num(), 0);
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 0);
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 1);
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 1);
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 2);
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 2);
		advance_session();
		assert_eq!(OracleFinance::current_era_num(), 3);
	});
}

#[test]
fn test_ask_era_num() {
	new_test_ext().execute_with(|| {
		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 5 );
		assert_eq!(Session::current_index(), 1 );
		assert_eq!(OracleFinance::current_era(), Some(0));
		assert_eq!(OracleFinance::eras_start_session_index(0), Some(2));

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 10 );
		assert_eq!(Session::current_index(), 2 );
		assert_eq!(OracleFinance::current_era(), Some(0));
		assert_eq!(OracleFinance::eras_start_session_index(0), Some(2));

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 15 );
		assert_eq!(Session::current_index(), 3 );
		assert_eq!(OracleFinance::current_era(), Some(1));
		assert_eq!(OracleFinance::eras_start_session_index(1), Some(4));

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 20 );
		assert_eq!(Session::current_index(), 4 );
		assert_eq!(OracleFinance::current_era(), Some(1));
		assert_eq!(OracleFinance::eras_start_session_index(1), Some(4));

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 25 );
		assert_eq!(Session::current_index(), 5 );
		assert_eq!(OracleFinance::current_era(), Some(2));
		assert_eq!(OracleFinance::eras_start_session_index(2), Some(6));

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 30 );
		assert_eq!(Session::current_index(), 6 );
		assert_eq!(OracleFinance::current_era(), Some(2));
		assert_eq!(OracleFinance::eras_start_session_index(2), Some(6));

		advance_session();
		assert_eq!(<frame_system::Pallet<Test>>::block_number(), 35 );
		assert_eq!(Session::current_index(), 7 );
		assert_eq!(OracleFinance::current_era(), Some(3));
		assert_eq!(OracleFinance::eras_start_session_index(3), Some(8));
	});
}

// use sp_core::Encode;
// use sp_core::Decode;
use sp_core::hexdisplay::{HexDisplay};
use codec::{Encode};


#[test]
fn test_aa () {
	// let mut a = 0xdea0b564;
	// let a = "are-ocw::local_host_key";
	// let ss: u32 = 385329431u32;
	let ss: u32 = 1689624798;
	let aa: u32 = 0xdea0b564;
	// let using_u8 = ss.using_encoded(|v| {
	// 	// println!("--- a = {:?}", HexDisplay::from(&ss.encode().encode()));
	// 	println!("--- a = {:?}", v);
	// 	// HexDisplay::from(v);
	// 	// println!("--- b = {:?}", HexDisplay::from(v));
	// });
	let using_u8 = ss.encode();
	println!("--- a = {:?}", using_u8);
	println!("--- b = {:?}", aa);
	println!("--- c = {:?}", HexDisplay::from(&using_u8));
	println!("--- d = {:?}", hex::decode("dea0b564".as_bytes()));

	// let number_to_str = sp_std::str::from_utf8(HexDisplay::from(&using_u8));
	// println!("--- c = {:?}", number_to_str);
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

	let target_json = 385329431u32;
	let target_json_v8 = target_json.encode();
	println!("Try body : Vec<u8> encode {:?} ", HexDisplay::from(&target_json_v8));

	// let target_json = "are-ocw::make_price_request_pool";
	// println!("Old title : Vec<u8> encode {:?} ", HexDisplay::from(target_json));
	assert!(true);
}

#[test]
fn test_end_session_event() {
	new_test_ext().execute_with(|| {
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
		advance_session();
	});
}
