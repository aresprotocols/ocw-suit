#![cfg_attr(not(feature = "std"), no_std)]

use ares_oracle_provider_support::{IAresOraclePreCheck, PreCheckStatus};
use codec::Encode;
use frame_election_provider_support::{data_provider, ElectionDataProvider};
use frame_support::pallet_prelude::PhantomData;
use frame_support::sp_std::fmt::Debug;
use frame_support::traits::{IsType, ValidatorSet};
use frame_support::Parameter;
use sp_application_crypto::RuntimeAppPublic;
use sp_npos_elections::{PerThing128, VoteWeight};
use sp_runtime::traits::{MaybeSerializeDeserialize, Member};
use sp_core::sp_std::vec::Vec;

pub struct DataProvider<T: Config>(PhantomData<T>);

pub trait Config: frame_system::Config {
	/// Something that provides the data for election.
	type DataProvider: ElectionDataProvider<Self::AccountId, Self::BlockNumber>;

	type ValidatorId: IsType<<Self as frame_system::Config>::AccountId> + Encode + Debug + PartialEq;

	type ValidatorSet: ValidatorSet<Self::ValidatorId>;

	// for aura authorityid.
	type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord + MaybeSerializeDeserialize;

	type AresOraclePreCheck: IAresOraclePreCheck<Self::AccountId, Self::AuthorityId, Self::BlockNumber>;
}

impl<T: Config> ElectionDataProvider<T::AccountId, T::BlockNumber> for DataProvider<T>
where
	<<T as Config>::ValidatorSet as ValidatorSet<<T as Config>::ValidatorId>>::ValidatorId:
		PartialEq<<T as frame_system::Config>::AccountId>,
{
	const MAXIMUM_VOTES_PER_VOTER: u32 = 0;

	fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<T::AccountId>> {
		//
		let result = T::DataProvider::targets(maybe_max_len);
		// log::debug!(target: "staking_extend", "******* LINDEBUG:: new targets:: == {:?}", result);

		if result.is_ok() {
			// check current validator
			let current_validators = T::ValidatorSet::validators();
			// log::debug!(target: "staking_extend", "******* LINDEBUG:: current validator:: == {:?}",
			// &current_validators);
			let mut old_target_list = Vec::new();
			let new_target = result.unwrap();
			let mut new_target = new_target.clone();
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
						T::AresOraclePreCheck::get_pre_check_status(target_acc.clone())
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
			log::debug!(target: "staking_extend", "******* LINDEBUG:: new validator:: == {:?}", &new_target);
			return Ok(old_target_list);
		}
		return result;
		// result
	}

	fn voters(
		maybe_max_len: Option<usize>,
	) -> data_provider::Result<Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)>> {
		T::DataProvider::voters(maybe_max_len)
	}

	fn desired_targets() -> data_provider::Result<u32> {
		T::DataProvider::desired_targets()
		// let result = T::DataProvider::desired_targets();
		// log::debug!(target: "staking_extend", "******* LINDEBUG:: desired_targets:: == {:?}",
		// result); result
	}

	fn next_election_prediction(now: T::BlockNumber) -> T::BlockNumber {
		// let result = T::DataProvider::next_election_prediction(now);
		// log::debug!(target: "staking_extend", "******* LINDEBUG:: next_election_prediction:: == {:?}",
		// result); result
		T::DataProvider::next_election_prediction(now)
	}
}
