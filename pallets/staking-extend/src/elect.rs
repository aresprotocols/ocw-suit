#![cfg_attr(not(feature = "std"), no_std)]

use frame_election_provider_support::{ElectionDataProvider, ElectionProvider};
use sp_npos_elections::Supports;
use frame_support::pallet_prelude::PhantomData;

pub struct OnChainSequentialPhragmen<T: Config>(PhantomData<T>);

pub trait Config: frame_system::Config {
	type ElectionProvider: ElectionProvider<Self::AccountId, Self::BlockNumber>;
	type DataProvider: ElectionDataProvider<Self::AccountId, Self::BlockNumber>;
}

impl<T: Config> ElectionProvider<T::AccountId, T::BlockNumber> for OnChainSequentialPhragmen<T> {
	// type Error = T::DebugError;
	type Error = <T::ElectionProvider as ElectionProvider<
		<T as frame_system::Config>::AccountId,
		<T as frame_system::Config>::BlockNumber,
	>>::Error;
	type DataProvider = T::DataProvider;

	fn elect() -> Result<Supports<T::AccountId>, Self::Error> {
		T::ElectionProvider::elect()
	}
}
