#![cfg_attr(not(feature = "std"), no_std)]

use ares_oracle_provider_support::{IAresOraclePreCheck, PreCheckStatus};
use codec::Encode;
use frame_election_provider_support::{data_provider, ElectionDataProvider, VoterOf};
use frame_support::pallet_prelude::PhantomData;
use frame_support::sp_std::fmt::Debug;
use frame_support::traits::{Get, IsType, ValidatorSet};
use frame_support::Parameter;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::sp_std::vec::Vec;
use sp_npos_elections::{PerThing128, VoteWeight};
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

pub struct DataProvider<T: Config>(PhantomData<T>);
impl<T: Config> ElectionDataProvider for DataProvider<T>
where
	<<T as Config>::ValidatorSet as ValidatorSet<<T as Config>::ValidatorId>>::ValidatorId:
		PartialEq<<<T as Config>::DataProvider as ElectionDataProvider>::AccountId>,
	<<T as Config>::DataProvider as ElectionDataProvider>::AccountId: Clone,
	<T as frame_system::Config>::AccountId: From<<<T as Config>::DataProvider as ElectionDataProvider>::AccountId>,
{
	type AccountId = <<T as Config>::DataProvider as ElectionDataProvider>::AccountId; // T::AccountId;
	type BlockNumber = <<T as Config>::DataProvider as ElectionDataProvider>::BlockNumber; // T::BlockNumber;
	type MaxVotesPerVoter = <<T as Config>::DataProvider as ElectionDataProvider>::MaxVotesPerVoter; // T::MaxNominations;
																								 // const MAXIMUM_VOTES_PER_VOTER: u32 = 0;

	fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<Self::AccountId>> {
		//
		let result = T::DataProvider::targets(maybe_max_len);
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

	fn voters(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<VoterOf<Self>>> {
		T::DataProvider::voters(maybe_max_len)
	}

	// fn voters(
	// 	maybe_max_len: Option<usize>,
	// ) -> data_provider::Result<Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)>> {
	// 	T::DataProvider::voters(maybe_max_len)
	// }

	fn desired_targets() -> data_provider::Result<u32> {
		T::DataProvider::desired_targets()
	}

	fn next_election_prediction(now: Self::BlockNumber) -> Self::BlockNumber {
		T::DataProvider::next_election_prediction(now)
	}
}
