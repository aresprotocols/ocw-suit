#![cfg_attr(not(feature = "std"), no_std)]

use frame_election_provider_support::onchain;
use frame_election_provider_support::{
	data_provider, ElectionDataProvider, ElectionProvider, PerThing128, Supports, VoteWeight,
};
use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::IsMember;
use frame_support::sp_runtime::traits::{OpaqueKeys, Zero};
use frame_support::sp_runtime::RuntimeAppPublic;
use frame_support::sp_std::fmt::Debug;
use frame_support::traits::ValidatorSet;
use frame_support::traits::{EstimateNextSessionRotation, Get};
use sp_core::sp_std::vec::Vec;

use ares_oracle_provider_support::{IAresOraclePreCheck, PreCheckStatus};

#[cfg(test)]
mod mock;

pub mod data;
pub mod elect;
#[cfg(test)]
mod tests;

// impl<T: Config> IsMember<T::ValidatorId> for OnChainSequentialPhragmen<T>
// where
// 	T::ValidatorId: PartialEq<<T::ValidatorSet as ValidatorSet<<T as
// frame_system::Config>::AccountId>>::ValidatorId>, 	T::ValidatorSet: ValidatorSet<<T as
// frame_system::Config>::AccountId>, {
// 	fn is_member(authority_id: &T::ValidatorId) -> bool {
// 		let validator_list = T::ValidatorSet::validators();
//
// 		validator_list.iter().any(|id| {
// 			log::info!("validator_list id = {:?} == author_id = {:?}", &id, &authority_id);
// 			authority_id == id
// 		})
// 	}
// }

pub struct StakingNPOS<T: Config>(PhantomData<T>);

pub trait Config: frame_system::Config {
	// for aura authorityid.
	type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord + MaybeSerializeDeserialize;
}

pub trait IStakingNpos<AuthorityId, BlockNumber> // : frame_system::Config (remove later.)
{
	type StashId;
	fn current_staking_era() -> u32;
	fn near_era_change(period_multiple: BlockNumber) -> bool;
	fn calculate_near_era_change(
		period_multiple: BlockNumber,
		current_bn: BlockNumber,
		session_length: BlockNumber,
		per_era: BlockNumber,
	) -> bool;
	fn old_npos() -> sp_core::sp_std::vec::Vec<Self::StashId>;
	fn pending_npos() -> sp_core::sp_std::vec::Vec<(Self::StashId, Option<AuthorityId>)>;
}

impl<T: Config> IStakingNpos<T::AuthorityId, T::BlockNumber> for StakingNPOS<T>
where
	T: pallet_staking::Config + pallet_session::Config + crate::Config,
	<T as pallet_session::Config>::ValidatorId: From<<T as frame_system::Config>::AccountId>,
{
	type StashId = <T as frame_system::Config>::AccountId;
	fn current_staking_era() -> u32 {
		pallet_staking::CurrentEra::<T>::get().unwrap_or(0)
	}

	fn near_era_change(period_multiple: T::BlockNumber) -> bool {
		let current_blocknum = <frame_system::Pallet<T>>::block_number();
		let per_era: T::BlockNumber = T::SessionsPerEra::get().into();
		let session_length = T::NextSessionRotation::average_session_length();

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
				let session_keys = <pallet_session::Pallet<T>>::load_keys(&stash_acc.clone().into());
				if session_keys.is_none() {
					return (stash_acc, None);
				}
				let session_keys = session_keys.unwrap();
				let authority_id = session_keys.get::<T::AuthorityId>(T::AuthorityId::ID);
				(stash_acc, authority_id)
			})
			.collect()

		// Vec::new()
	}
}

impl<A, B> IStakingNpos<A, B> for () {
	type StashId = sp_application_crypto::sr25519::Public;
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

// impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
// 	type Public = T::AuthorityId;
// }
//
// impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T>
// {
// 	type Public = <T::WithSessionHandler as OneSessionHandler<<T as
// frame_system::Config>::AccountId>>::Key; 	// type Public = T::AuthorityId ;
// }

// impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
// 	// type Key = T::AuthorityId ;
// 	type Key = <T::WithSessionHandler as OneSessionHandler<<T as
// frame_system::Config>::AccountId>>::Key ;
//
// 	fn on_genesis_session<'a, I: 'a>(validators: I)
// 		where
// 			I: Iterator<Item = (&'a T::AccountId, Self::Key)> ,
// 			// <T::WithSessionHandler as OneSessionHandler<<T as frame_system::Config>::AccountId>>::Key:
// From<T::AuthorityId>, 	{
//
//
// 		let new_validators: Box<dyn Iterator<Item=_>> = Box::new(validators
// 			.map(|k| k ));
// 		// println!("xxx= {}", validators.count());
//
//
// 		// let new_validator = validators.map(|(_, k)| {
// 		// 	k as <T::WithSessionHandler as OneSessionHandler<<T as frame_system::Config>::AccountId>>::Key
// 		// }).collect::<Vec<<T::WithSessionHandler as OneSessionHandler<<T as
// frame_system::Config>::AccountId>>::Key>>();
//
// 		T::WithSessionHandler::on_genesis_session(new_validators)
// 	}
//
// 	fn on_new_session<'a, I: 'a>(changed: bool, validators: I, queued_validators: I)
// 		where
// 			I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
// 	{
//
// 		let new_validators: Box<dyn Iterator<Item=_>> = Box::new(validators
// 			.map(|k| {
// 				log::debug!("++++++++++++ on_new_session = validators {:?}", k.0.clone());
// 				k
// 			} ));
//
//
// 		log::debug!("----------- on_new_session = BEGIN is change = {}", changed);
// 		// println!("----------- on_new_session =   BEGIN is change = {}", changed);
//
// 		let new_queued_validators: Box<dyn Iterator<Item=_>> = Box::new(queued_validators
// 			.map(|k| {
// 				log::debug!("----------- on_new_session = new_queued_validators {:?}", k.0.clone());
// 				// println!("----------- on_new_session = new_queued_validators {:?}", k.0.clone());
// 				k
// 			} ));
// 		log::debug!("----------- on_new_session = new_queued_validators END");
//
// 		// let new_queued_validators = queued_validators.map(|(_, k)| {
// 		// 	log::info!(" ***** LINDEBUG new_queued_validators ");
// 		// 	k
// 		// }).collect::<Vec<_>>();
//
// 		// let new_queued_validators: Box<dyn Iterator<Item=_>> = Box::new(validators
// 		// 	.map(|k| k ));
//
// 		// log::info!("****** 2debug .. on_new_session == validators-count= {:?} queued_validators-count
// = {:?} ", &validators.count().clone(), &queued_validators.count().clone());
//
// 		// let next_authorities = validators.map(|(_, k)| {
// 		// 	log::info!(" ***** LINDEBUG validator = {:?}", k);
// 		// 	println!(" ***** LINDEBUG validator = {:?}", k);
// 		// 	k
// 		// }).collect::<Vec<T::AuthorityId>>();
// 		//
// 		// let next_authorities = queued_validators.map(|(_, k)| {
// 		// 	log::info!(" ***** LINDEBUG queued = {:?}", k);
// 		// 	println!(" ***** LINDEBUG queued = {:?}", k);
// 		// 	k
// 		// }).collect::<Vec<T::AuthorityId>>();
// 		// println!("@@@@@@@@@");
// 		// println!("*** LINDEBUG:: current_validators == {:?}", current_validators);
// 		// println!("*** 2 LINDEBUG:: next_authorities == {:?}", &next_authorities);
// 		T::WithSessionHandler::on_new_session(changed, new_validators, new_queued_validators)
// 	}
//
// 	fn on_disabled(i: usize) {
// 		T::WithSessionHandler::on_disabled(i)
// 	}
// }
