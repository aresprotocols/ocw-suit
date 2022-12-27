#[cfg(test)]

use crate::mock::*;
use crate::IStakingNpos;

use frame_support::assert_ok;
use pallet_staking::ConvertCurve;

use sp_runtime::{
	impl_opaque_keys,
	testing::{Header, UintAuthorityId},
	traits::{BlakeTwo256, ConvertInto, IdentityLookup},
	Perbill,
};

#[test]
fn test_get_pending_npos_listd() {
	new_test_ext().execute_with(|| {
		advance_session();
		// assert_eq!(<Test as crate::Config>::IStakingNpos::current_staking_era(), 0);
		// assert_eq!(<Test as crate::Config>::IStakingNpos::old_npos(), vec![CONST_VALIDATOR_ID_2,
		// CONST_VALIDATOR_ID_1]) ; assert_eq!(<Test as crate::Config>::IStakingNpos::pending_npos().len(),
		// 0) ;

		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert_eq!(crate::StakingNPOS::<Test>::old_npos(), vec![CONST_VALIDATOR_ID_2, CONST_VALIDATOR_ID_1]);
		assert_eq!(crate::StakingNPOS::<Test>::pending_npos().len(), 0);

		// bound new validator
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert_ok!(Staking::bond(
			Origin::signed(CONST_VALIDATOR_ID_3),
			CONST_VALIDATOR_ID_3,
			1500,
			pallet_staking::RewardDestination::Controller
		));
		assert_ok!(Staking::validate(
			Origin::signed(CONST_VALIDATOR_ID_3),
			pallet_staking::ValidatorPrefs::default()
		));

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
		assert_ok!(Session::set_keys(
			Origin::signed(CONST_VALIDATOR_ID_3),
			UintAuthorityId(8).into(),
			vec![]
		));

		advance_session();
		advance_session();

		//
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 1);
		assert_eq!(crate::StakingNPOS::<Test>::pending_npos().len(), 1);
		assert_eq!(
			crate::StakingNPOS::<Test>::pending_npos(),
			vec![(CONST_VALIDATOR_ID_3, Some(UintAuthorityId(8))),]
		);

		advance_session();
		advance_session();
	});
}

#[test]
fn test_lines() {
	use pallet_staking::EraPayout;
	new_test_ext().execute_with(|| {
		// 28994100000000
		// let payout = ConvertCurve::<RewardCurve>::era_payout(28183518u64, 1040000000u64, 60*60*24*1000);
		let payout = ConvertCurve::<RewardCurve>::era_payout(28183518u64, 1000000000u64, 60*60*24*1000);
		println!("payout = {:?}", payout)
		// <Staking as pallet_staking::Config>::EraPayout::era_payout(5000, 10000, 6000);
	});
}

#[test]
fn test_calculate_near_era_change_period_eq_ear() {
	new_test_ext().execute_with(|| {
		advance_session(); // 1 bn = 5
		assert!(crate::StakingNPOS::<Test>::calculate_near_era_change(2, 5, 5, 1));

		advance_session(); // 1
		assert!(crate::StakingNPOS::<Test>::calculate_near_era_change(2, 10, 5, 1));

		advance_session(); // 1
		assert!(crate::StakingNPOS::<Test>::calculate_near_era_change(2, 15, 5, 1));

		advance_session(); // 1
		assert!(crate::StakingNPOS::<Test>::calculate_near_era_change(2, 20, 5, 1));
	});
}

#[test]
fn test_calculate_near_era_change_period_eq_muti() {
	new_test_ext().execute_with(|| {
		advance_session(); // 1
		assert!(crate::StakingNPOS::<Test>::calculate_near_era_change(2, 5, 5, 2));

		advance_session(); // 1
		assert!(!crate::StakingNPOS::<Test>::calculate_near_era_change(2, 10, 5, 2));

		advance_session(); // 1
		assert!(crate::StakingNPOS::<Test>::calculate_near_era_change(2, 15, 5, 2));

		advance_session(); // 1
		assert!(!crate::StakingNPOS::<Test>::calculate_near_era_change(2, 20, 5, 2));
	});
}

#[test]
fn test_calculate_near_era_change_period_eq_ear_triple() {
	new_test_ext().execute_with(|| {
		advance_session(); // 1 bn = 5 (period_multiple,current_bn,session_len,per_era)
		assert!(!crate::StakingNPOS::<Test>::calculate_near_era_change(2, 5, 5, 3));

		advance_session(); // 1 bn = 10
		assert!(crate::StakingNPOS::<Test>::calculate_near_era_change(2, 10, 5, 3));

		advance_session(); // 1 bn = 15
		assert!(!crate::StakingNPOS::<Test>::calculate_near_era_change(2, 15, 5, 3));

		advance_session(); // 1 bn = 20
		assert!(!crate::StakingNPOS::<Test>::calculate_near_era_change(2, 20, 5, 3));
	});
}

#[test]
fn test_near_era_change() {
	new_test_ext().execute_with(|| {
		advance_session(); // 1
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 2
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 3
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 4
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 5
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 6
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 7
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 8
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 9
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(crate::StakingNPOS::<Test>::near_era_change(2), "Check npos request by offchain worker.");

		advance_session(); // 10
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 0);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 11
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 1);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 12
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 1);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));

		advance_session(); // 13
		assert_eq!(crate::StakingNPOS::<Test>::current_staking_era(), 1);
		assert!(!crate::StakingNPOS::<Test>::near_era_change(2));
	});
}
