#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;
use frame_election_provider_support::onchain;
use frame_support::traits::{Get, EstimateNextSessionRotation, OneSessionHandler};
use pallet_session::{ShouldEndSession, PeriodicSessions};
use frame_support::sp_std::marker::PhantomData;
use frame_support::sp_runtime::traits::{UniqueSaturatedInto, Zero};
use frame_support::sp_runtime::RuntimeAppPublic;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, ConsensusEngineId};
	use frame_system::pallet_prelude::*;
	// use pallet_ocw::{ValidatorHandler};
	use sp_std::vec::Vec;
	use frame_support::sp_runtime::{RuntimeAppPublic, AccountId32};
	use frame_support::sp_runtime::traits::{IsMember, AccountIdConversion};
	use frame_support::traits::{ValidatorSet, FindAuthor};
	use frame_support::sp_std::fmt::Debug;
	use frame_election_provider_support::{data_provider, VoteWeight, ElectionDataProvider, Supports, ElectionProvider, onchain, PerThing128};
	use crate::IStakingNpos;
	// frame-election-provider-support

	// type Aura<T> = pallet_aura::Pallet<T>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config  {
		// type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type ValidatorId: IsType<<Self as frame_system::Config>::AccountId>  + Encode + Debug + PartialEq;
		type ValidatorSet: ValidatorSet<Self::ValidatorId>;

		type AuthorityId: Member
		+ Parameter
		+ RuntimeAppPublic
		+ Default
		+ Ord
		+ MaybeSerializeDeserialize;

		type DataProvider: ElectionDataProvider<Self::AccountId, Self::BlockNumber>;

		// type DebugError: Debug;
		type IStakingNpos: IStakingNpos<<Self as frame_system::Config>::AccountId, Self::BlockNumber>;

		type OnChainAccuracy: PerThing128;
		type ElectionProvider: frame_election_provider_support::ElectionProvider<
			Self::AccountId,
			Self::BlockNumber,
			// we only accept an election provider that has staking as data provider.
			DataProvider = Pallet<Self>,
		>;

		type GenesisElectionProvider: frame_election_provider_support::ElectionProvider<
			Self::AccountId,
			Self::BlockNumber,
			DataProvider = Pallet<Self>,
		>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	impl<T: Config > IsMember<T::ValidatorId> for Pallet<T>
		// where T::ValidatorId: PartialEq<<T::ValidatorSet as ValidatorSet<<T as frame_system::Config>::AccountId>>::ValidatorId>
		where T::ValidatorId: PartialEq<<T::ValidatorSet as ValidatorSet<<T as frame_system::Config>::AccountId>>::ValidatorId>,
			  T::ValidatorSet: ValidatorSet<<T as frame_system::Config>::AccountId>
	{
		fn is_member(authority_id: &T::ValidatorId) -> bool
		{
			let validator_list = T::ValidatorSet::validators();

			validator_list.iter().any(|id| {
				log::info!("validator_list id = {:?} == author_id = {:?}", & id, &authority_id);
				authority_id == id
			})
		}
	}

	impl<T: Config> frame_election_provider_support::ElectionDataProvider<T::AccountId, T::BlockNumber> for Pallet<T>
		where <<T as Config>::ValidatorSet as ValidatorSet<<T as Config>::ValidatorId>>::ValidatorId: PartialEq<<T as frame_system::Config>::AccountId>
	{
		const MAXIMUM_VOTES_PER_VOTER: u32 = 0;

		fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<T::AccountId>>
		{
			T::DataProvider::targets(maybe_max_len)
		}

		// TODO:: kami:: develop for new feature, don't remove blew.
		// fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<T::AccountId>>
		// {
		// 	// submit 3 times. 3% , 2/3 10 block submit.
		// 	let result = T::DataProvider::targets(maybe_max_len);
		// 	log::debug!(target: "staking_extend", "******* LINDEBUG:: new targets:: == {:?}", result);
		//
		// 	// check current validator
		// 	let current_validators = T::ValidatorSet::validators();
		// 	log::debug!(target: "staking_extend", "******* LINDEBUG:: current validator:: == {:?}", &current_validators);
		//
		// 	//
		// 	let mut old_target_list = Vec::new();
		//
		// 	if result.is_ok() {
		// 		let mut new_target = result.clone().unwrap();
		// 		new_target.retain(|target_acc|{
		// 			!current_validators.iter().any(|current_acc|{
		// 				let is_exists = &current_acc == &target_acc;
		// 				log::debug!(target: "staking_extend", "current_acc {:?} == target_acc {:?} ", &current_acc, &target_acc);
		// 				log::debug!(target: "staking_extend", "Result = {:?} ", &is_exists);
		// 				if is_exists {
		// 					old_target_list.push(target_acc.clone());
		// 				}
		// 				is_exists
		// 			})
		// 		});
		// 		log::debug!(target: "staking_extend", "******* LINDEBUG:: new validator:: == {:?}", &new_target);
		// 	}
		//
		// 	Ok(old_target_list)
		// 	// result
		// }

		fn voters(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)>> {
			T::DataProvider::voters(maybe_max_len)
		}

		fn desired_targets() -> data_provider::Result<u32> {
			let result = T::DataProvider::desired_targets();
			log::debug!(target: "staking_extend", "******* LINDEBUG:: desired_targets:: == {:?}", result);
			result
		}

		fn next_election_prediction(now: T::BlockNumber) -> T::BlockNumber {
			let result = T::DataProvider::next_election_prediction(now);
			log::debug!(target: "staking_extend", "******* LINDEBUG:: next_election_prediction:: == {:?}", result);
			result
		}
	}

	impl<T: Config> frame_election_provider_support::ElectionProvider<T::AccountId, T::BlockNumber> for Pallet<T> {
		// type Error = T::DebugError;
		type Error = <T::ElectionProvider as ElectionProvider<<T as frame_system::Config>::AccountId, <T as frame_system::Config>::BlockNumber>>::Error;
		type DataProvider = T::DataProvider ;

		fn elect() -> Result<Supports<T::AccountId>, Self::Error> {
			T::ElectionProvider::elect()
		}
	}

}

/// Wrapper type that implements the configurations needed for the on-chain backup.
pub struct OnChainConfig<T: Config>(sp_std::marker::PhantomData<T>);
impl<T: Config> onchain::Config for OnChainConfig<T> {
	type AccountId = T::AccountId;
	type BlockNumber = T::BlockNumber;
	type Accuracy = T::OnChainAccuracy;
	type DataProvider = T::DataProvider;
}


impl<T: Config> Pallet<T> {
	// pub fn hello() {
	// 	println!("HELLO");
	// 	let nops_list = T::IStakingNpos::new_npos();
	// 	println!("nops_list = {:?}" ,nops_list);
	// }
}

pub trait IStakingNpos<ValidatorId, BlockNumber>: frame_system::Config {
	fn current_staking_era() -> u32;
	fn near_era_change(leading_period: BlockNumber) -> bool;
	fn old_npos() -> Vec<ValidatorId> ;
	fn pending_npos() -> Vec<ValidatorId> ;
}


impl<T: Config> IStakingNpos<<T as frame_system::Config>::AccountId, T::BlockNumber> for T
	where T: pallet_staking::Config + pallet_authority_discovery::Config,
{

	fn near_era_change(leading_period: T::BlockNumber) -> bool {
		let current_blocknum = <frame_system::Pallet<T>>::block_number();
		let per_era: T::BlockNumber = T::SessionsPerEra::get().into();
		let session_length = T::NextSessionRotation::average_session_length();
		let round_num = session_length * per_era ;
		// println!("current_blocknum = {:?} , leading_period = {:?}, per_era = {:?}, session_length = {:?}, current_staking = {:?}, xx = {:?}", current_blocknum, leading_period, per_era, session_length, current_staking, xx);
		(current_blocknum + (leading_period * session_length )) % round_num == session_length
	}

	fn current_staking_era() -> u32 {
		pallet_staking::CurrentEra::<T>::get().unwrap_or(0)
	}

	fn old_npos() -> Vec<<T as frame_system::Config>::AccountId> {
		// get current era
		let current_era = Self::current_staking_era();
		pallet_staking::ErasStakers::<T>::iter_key_prefix(current_era)
			.into_iter()
			.map(|acc| acc)
			.collect()
	}

	fn pending_npos() -> Vec<<T as frame_system::Config>::AccountId> {
		let current_npos_list = Self::old_npos();
		// Make list diff
		let mut target_npos_list = <pallet_staking::Pallet<T>>::get_npos_targets();
		target_npos_list.retain(|target_acc|{
			!current_npos_list.iter().any(|current_acc|{
				&current_acc == &target_acc
				// let is_exists = &current_acc == &target_acc;
				// println!("staking_extend current_acc {:?} == target_acc {:?} ", &current_acc, &target_acc);
				// println!("staking_extend Result = {:?} ", &is_exists);
				// is_exists
			})
		});

		if target_npos_list.len() > 0 {
			// let keys_public_id = <pallet_session::Pallet<T>>::NextKeys::<T>::get(target_npos_list[0]);
			// <pallet_session::Pallet<T>>::
			// let keys_public_id = pallet_session::NextKeys::<T>::get(target_npos_list[0]);
			// let keys_public_id = <pallet_session::Pallet<T>>::nextKeys(target_npos_list[0]);
			// let keys_public_id = <pallet_session::Pallet<T>>::next_keys(target_npos_list[0]);

			let keys_public_id = <pallet_authority_discovery::Pallet<T>>::next_authorities();
			// println!("keys_public_id = {:}", keys_public_id);
		}

		target_npos_list
	}
}



impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T>
{
	type Public = T::AuthorityId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T>
{
	type Key = T::AuthorityId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
		where
			I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		let authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		Self::initialize_authorities(&authorities);
	}

	fn on_new_session<'a, I: 'a>(changed: bool, validators: I, queued_validators: I)
		where
			I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		// instant changes
		// if changed {
		//     let next_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();
		//     let last_authorities = Self::authorities();
		//     // if next_authorities != last_authorities {
		//     //     Self::change_authorities(next_authorities);
		//     // }
		//     log::info!("*** LINDEBUG:: last_authorities == {:?}", last_authorities);
		//     log::info!("*** LINDEBUG:: validators == {:?}", validators);
		//     log::info!("*** LINDEBUG:: queued_validators == {:?}", queued_validators);
		// }

		let last_authorities = Self::authorities();
		log::info!("*** LINDEBUG:: last_authorities == {:?}", last_authorities);
		log::info!("*** LINDEBUG:: validators == {:?}", validators);
		log::info!("*** LINDEBUG:: queued_validators == {:?}", queued_validators);
	}

	fn on_disabled(i: usize) {
		let log: DigestItem<T::Hash> = DigestItem::Consensus(
			AURA_ENGINE_ID,
			ConsensusLog::<T::AuthorityId>::OnDisabled(i as AuthorityIndex).encode(),
		);

		<frame_system::Pallet<T>>::deposit_log(log.into());
	}
}
