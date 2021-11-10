use crate::{mock::*, Error, };
use frame_support::{assert_noop, assert_ok};
use crate::traits::*;
use crate::types::*;
use frame_support::traits::OnInitialize;

#[test]
fn test_it_works_for_default_value() {
	new_test_ext().execute_with(|| {

		assert(false);
	});
}
