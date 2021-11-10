#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;
use frame_election_provider_support::onchain;

// #[cfg(test)]
// mod mock;
//
// #[cfg(test)]
// mod tests;

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
	// frame-election-provider-support

	// type Aura<T> = pallet_aura::Pallet<T>;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config  {
		// type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type ValidatorId: IsType<<Self as frame_system::Config>::AccountId>  + Encode + Debug + PartialEq;
		type ValidatorSet: ValidatorSet<Self::ValidatorId>;

		type DataProvider: ElectionDataProvider<Self::AccountId, Self::BlockNumber>;

		type DebugError: Debug;

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

	impl<T: Config> frame_election_provider_support::ElectionDataProvider<T::AccountId, T::BlockNumber> for Pallet<T> {
		const MAXIMUM_VOTES_PER_VOTER: u32 = 0;

		fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<T::AccountId>> {
			// submit 3 times. 3% , 2/3 10 block submit.
			let result = T::DataProvider::targets(maybe_max_len);
			log::info!("******* LINDEBUG:: new targets:: == {:?}", result);

			// check current validator
			let current_validators = T::ValidatorSet::validators();
			log::info!("******* LINDEBUG:: current validator:: == {:?}", &current_validators);

			// if result.is_ok() {
			// 	let mut new_target = result.clone().unwrap();
			// 	new_target.retain(|target_acc|{
			// 		current_validators.iter().any(|current_acc|{
			// 			!(current_acc == target_acc)
			// 		})
			// 	});
			// 	log::info!("******* LINDEBUG:: new validator:: == {:?}", &new_target);
			// }

			result
		}

		fn voters(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<(T::AccountId, VoteWeight, Vec<T::AccountId>)>> {
			T::DataProvider::voters(maybe_max_len)
		}

		fn desired_targets() -> data_provider::Result<u32> {
			let result = T::DataProvider::desired_targets();
			log::info!("******* LINDEBUG:: desired_targets:: == {:?}", result);
			result
		}

		fn next_election_prediction(now: T::BlockNumber) -> T::BlockNumber {
			let result = T::DataProvider::next_election_prediction(now);
			log::info!("******* LINDEBUG:: next_election_prediction:: == {:?}", result);
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