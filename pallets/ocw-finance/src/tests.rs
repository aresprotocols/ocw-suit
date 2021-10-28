use crate::{mock::*, Error, PaymentTrace, AskPeriodPayment};
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
	new_test_ext().execute_with(|| {

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

		assert_eq!(OcwFinance::get_sum_of_record_point(OcwFinance::make_period_num(3)), 0);
		// who: T::AccountId, p_id: PurchaseId, bn: T::BlockNumber, ask_point: u64sum
		assert_ok!(OcwFinance::record_submit_point(AccountId_1, purchase_id_1.clone(), 1, 9));
		assert_eq!(OcwFinance::record_submit_point(AccountId_1, purchase_id_1.clone(), 1, 9), Err(Error::<Test>::PointRecordIsAlreadyExists) );
		assert_ok!(OcwFinance::record_submit_point(AccountId_2, purchase_id_1.clone(), 3, 10));
		assert_eq!(OcwFinance::get_sum_of_record_point(OcwFinance::make_period_num(3)), 19, "Get all the record points in the first time zone");

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