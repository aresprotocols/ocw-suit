use crate::{mock::*, Config};
use crate::IStakingNpos;

use frame_support::{assert_noop, assert_ok};
// use crate::traits::*;
// use crate::types::*;
use frame_support::traits::OnInitialize;

use sp_runtime::{
	impl_opaque_keys,
	testing::{Header, UintAuthorityId},
	traits::{BlakeTwo256, ConvertInto, IdentityLookup},
	Perbill
};

#[test]
fn test_get_pending_npos_listd() {
	new_test_ext().execute_with(|| {

		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert_eq!(<Test as crate::Config>::IStakingNpos::old_npos(), vec![CONST_VALIDATOR_ID_2, CONST_VALIDATOR_ID_1]) ;
		assert_eq!(<Test as crate::Config>::IStakingNpos::pending_npos().len(), 0) ;

		// bound new validator
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert_ok!(Staking::bond(
			Origin::signed(CONST_VALIDATOR_ID_3),
			CONST_VALIDATOR_ID_3,
			1500,
			pallet_staking::RewardDestination::Controller));
		assert_ok!(Staking::validate(Origin::signed(CONST_VALIDATOR_ID_3), pallet_staking::ValidatorPrefs::default()));

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

		// Set session key
		assert_ok!(Session::set_keys(Origin::signed(CONST_VALIDATOR_ID_3), UintAuthorityId(3).into(), vec![]));

		//
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 1);
		assert_eq!(<Test as crate::Config>::IStakingNpos::pending_npos().len(), 1) ;
		assert_eq!(<Test as crate::Config>::IStakingNpos::pending_npos(), vec![CONST_VALIDATOR_ID_3]) ;

	});
}


#[test]
fn test_near_era_change() {
	new_test_ext().execute_with(|| {

		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();

		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(<Test as crate::Config>::IStakingNpos::near_era_change(2), "Check npos.");

		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 1);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 1);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));
		advance_session();
		assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 1);
		assert!(!<Test as crate::Config>::IStakingNpos::near_era_change(2));

	});
}
