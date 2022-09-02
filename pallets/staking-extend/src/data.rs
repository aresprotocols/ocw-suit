//! The module that implements the `ElectionDataProvider` trait is the adapter between election and Staking data.

#![cfg_attr(not(feature = "std"), no_std)]

// use core::num::FpCategory::Zero;
use ares_oracle_provider_support::{IAresOraclePreCheck, PreCheckStatus};
use codec::Encode;
use frame_election_provider_support::{data_provider, ElectionDataProvider, VoterOf};
use frame_support::pallet_prelude::PhantomData;
use frame_support::sp_std::fmt::Debug;
use frame_support::traits::{IsType, ValidatorSet};
use frame_support::Parameter;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::sp_std::vec::Vec;
use sp_runtime::traits::{MaybeSerializeDeserialize, Member};

pub trait Config: frame_system::Config {
	/// Something that provides the data for election.
	type DataProvider: ElectionDataProvider;

	type ValidatorId: IsType<<Self as frame_system::Config>::AccountId> + Encode + Debug + PartialEq;

	type ValidatorSet: ValidatorSet<Self::ValidatorId>;

	// for aura authorityid.
	type AuthorityId: Member + Parameter + RuntimeAppPublic + Ord + MaybeSerializeDeserialize;

	type AresOraclePreCheck: IAresOraclePreCheck<Self::AccountId, Self::AuthorityId, Self::BlockNumber>;
}

/// Implement the structure of ElectionDataProvider.
pub struct DataProvider<T: Config>(PhantomData<T>);
impl<T: Config> ElectionDataProvider for DataProvider<T>
where
	<<T as Config>::ValidatorSet as ValidatorSet<<T as Config>::ValidatorId>>::ValidatorId:
		PartialEq<<<T as Config>::DataProvider as ElectionDataProvider>::AccountId>,
	<<T as Config>::DataProvider as ElectionDataProvider>::AccountId: Clone + PartialEq,
	<T as frame_system::Config>::AccountId: From<<<T as Config>::DataProvider as ElectionDataProvider>::AccountId>,
{
	type AccountId = <<T as Config>::DataProvider as ElectionDataProvider>::AccountId; // T::AccountId;
	type BlockNumber = <<T as Config>::DataProvider as ElectionDataProvider>::BlockNumber; // T::BlockNumber;
	type MaxVotesPerVoter = <<T as Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter; // T::MaxNominations;
																								 // const MAXIMUM_VOTES_PER_VOTER: u32 = 0;

	fn electable_targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<Self::AccountId>> {
		//
		let result = T::DataProvider::electable_targets(maybe_max_len);
		// log::debug!(target: "staking_extend", "******* LINDEBUG:: new targets:: == {:?}", result);

		if result.is_ok() {
			// check current validator
			let current_validators = T::ValidatorSet::validators();
			// log::debug!(target: "staking_extend", "******* LINDEBUG:: current validator:: == {:?}",
			// &current_validators);
			let mut old_target_list = Vec::new();
			let mut new_target = result.unwrap();
			// let mut new_target = new_target.clone();
			new_target.retain(|target_acc| {
				let is_new_target = !current_validators.iter().any(|current_acc| {
					let is_exists = &current_acc == &target_acc;
					// log::debug!(target: "staking_extend", "current_acc {:?} == target_acc {:?} ", &current_acc,
					// &target_acc); log::debug!(target: "staking_extend", "Result = {:?} ", &is_exists);
					if is_exists {
						old_target_list.push(target_acc.clone());
					}
					is_exists
				});

				if is_new_target {
					// check pre-price has success.
					if let Some((_, new_target_status)) =
						T::AresOraclePreCheck::get_pre_check_status(target_acc.clone().into())
					{
						match new_target_status {
							PreCheckStatus::Review => {}
							PreCheckStatus::Prohibit => {}
							PreCheckStatus::Pass => {
								old_target_list.push(target_acc.clone());
							}
						}
					}
				}
				is_new_target
			});
			// log::debug!(target: "staking_extend", "******* LINDEBUG:: new validator:: == {:?}", &new_target);
			return Ok(old_target_list);
		}
		return result;
	}

	fn electing_voters(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<VoterOf<Self>>> {
		let voters = T::DataProvider::electing_voters(maybe_max_len);
		voters
	}

	fn desired_targets() -> data_provider::Result<u32> {

		let staking_desired_target = T::DataProvider::desired_targets();
		if let Ok(staking_desired_target) = staking_desired_target {
			let desired_targets = Self::electable_targets(None).ok().map_or(0usize, |v| { v.len() }) as u32;
			if desired_targets > staking_desired_target {
				return data_provider::Result::<u32>::Ok(staking_desired_target)
			}
			return data_provider::Result::<u32>::Ok(desired_targets);
		}
		staking_desired_target

		// T::DataProvider::desired_targets()
	}

	fn next_election_prediction(now: Self::BlockNumber) -> Self::BlockNumber {
		T::DataProvider::next_election_prediction(now)
	}
}
