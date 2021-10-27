use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};
use crate::traits::*;

#[test]
fn it_works_for_default_value() {
	new_test_ext().execute_with(|| {

		let calculate_result = OcwFinance::calculate_fee_of_ask_quantity(3);
		assert_eq!(calculate_result, 3u64.saturating_mul(DOLLARS));
	});
}
