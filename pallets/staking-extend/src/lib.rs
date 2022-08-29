//! Staking-extend is an adapter Pallet used to adapt the Staking and Election modules,
//! by affecting the election `target` list to affecting the final result of the election.
//!
//! This pallet is used for the `pre-check` functions of `ares-oracle`

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::{OpaqueKeys, Zero};
use frame_support::sp_runtime::RuntimeAppPublic;
use frame_support::traits::EstimateNextNewSession;
use frame_support::traits::Get;
use sp_core::sp_std::vec::Vec;
use sp_runtime::traits::MaybeDisplay;
use sp_std::fmt::Debug;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod data;
pub mod elect;



/// Implement the structure of IStakingNpos
pub struct StakingNPOS<T: Config>(PhantomData<T>);

pub trait Config: frame_system::Config {
	/// for Ares authorityid.
	type AuthorityId: Member + Parameter + RuntimeAppPublic + Ord + MaybeSerializeDeserialize;
}

/// Methods to expose Staking and Election
pub trait IStakingNpos<AuthorityId, BlockNumber>
{
	/// Generally corresponds to the validator in the Staking pallet.
	type StashId;

	/// Get the Era of the current Staking pallet.
	fn current_staking_era() -> u32;

	/// Used to determine whether era changes are about to occur.
	///
	/// - period_multiple: session-multi indicates the trigger pre-check session period before the era.
	fn near_era_change(period_multiple: BlockNumber) -> bool;
	fn calculate_near_era_change(
		period_multiple: BlockNumber,
		current_bn: BlockNumber,
		session_length: BlockNumber,
		per_era: BlockNumber,
	) -> bool;

	/// Get the list of validators before the election.
	fn old_npos() -> sp_core::sp_std::vec::Vec<Self::StashId>;

	/// Get the list of new validators after the current election, excluding validators from the previous session.
	fn pending_npos() -> sp_core::sp_std::vec::Vec<(Self::StashId, Option<AuthorityId>)>;
}

impl<T: Config> IStakingNpos<T::AuthorityId, T::BlockNumber> for StakingNPOS<T>
where
	T: pallet_staking::Config,
	T: pallet_session::Config,
	T: crate::Config,
	<T as pallet_session::Config>::ValidatorId: From<<T as frame_system::Config>::AccountId>,
{
	type StashId = <T as frame_system::Config>::AccountId;
	fn current_staking_era() -> u32 {
		pallet_staking::CurrentEra::<T>::get().unwrap_or(0)
	}

	fn near_era_change(period_multiple: T::BlockNumber) -> bool {
		let current_blocknum = <frame_system::Pallet<T>>::block_number();
		let per_era: T::BlockNumber = T::SessionsPerEra::get().into();
		let session_length = <pallet_session::Pallet<T>>::average_session_length();

		Self::calculate_near_era_change(period_multiple, current_blocknum, session_length, per_era)
	}

	fn calculate_near_era_change(
		period_multiple: T::BlockNumber,
		current_bn: T::BlockNumber,
		session_length: T::BlockNumber,
		per_era: T::BlockNumber,
	) -> bool {
		let round_num = session_length * per_era;
		// check period_multiple
		let period_multiple = per_era.min(period_multiple);
		// check session length
		let mut check_session_length: T::BlockNumber = session_length;
		if period_multiple <= 1u32.into() {
			check_session_length = Zero::zero();
		}
		log::debug!("###### current_bn {:?} + ( period_multiple {:?} * session_length {:?}) % round_num {:?} = check_session_length {:?} ## current stakin period = {:?}",
					current_bn,
					period_multiple,
					session_length,
					round_num,
					check_session_length,
					Self::current_staking_era()
		);
		// (n + (2*40)) % 40
		(current_bn + (period_multiple * session_length)) % round_num == check_session_length
	}

	fn old_npos() -> Vec<Self::StashId> {
		// get current era
		let current_era = Self::current_staking_era();
		pallet_staking::ErasStakers::<T>::iter_key_prefix(current_era)
			.into_iter()
			.map(|acc| acc)
			.collect()
	}

	fn pending_npos() -> Vec<(Self::StashId, Option<T::AuthorityId>)> {
		let current_npos_list = Self::old_npos();
		// Make list diff
		let mut target_npos_list = <pallet_staking::Pallet<T>>::get_npos_targets();
		target_npos_list.retain(|target_acc| !current_npos_list.iter().any(|current_acc| &current_acc == &target_acc));

		target_npos_list
			.into_iter()
			.map(|stash_acc| {
				let validator_id: T::ValidatorId = stash_acc.clone().into();
				let session_keys = <pallet_session::NextKeys<T>>::get(&validator_id);
				if session_keys.is_none() {
					return (stash_acc, None);
				}
				let session_keys = session_keys.unwrap();
				let authority_id = session_keys.get::<T::AuthorityId>(T::AuthorityId::ID);
				(stash_acc, authority_id)
			})
			.collect()
	}
}

impl<A, B> IStakingNpos<A, B> for () {
	// type StashId = sp_application_crypto::sr25519::Public;
	type StashId = sp_runtime::AccountId32;
	fn current_staking_era() -> u32 {
		0
	}
	fn near_era_change(_leading_period: B) -> bool {
		false
	}
	fn calculate_near_era_change(_period_multiple: B, _current_bn: B, _session_length: B, _per_era: B) -> bool {
		false
	}
	fn old_npos() -> sp_core::sp_std::vec::Vec<Self::StashId> {
		Vec::new()
	}
	fn pending_npos() -> sp_core::sp_std::vec::Vec<(Self::StashId, Option<A>)> {
		Vec::new()
	}
}

