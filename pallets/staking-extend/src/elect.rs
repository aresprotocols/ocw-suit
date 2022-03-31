#![cfg_attr(not(feature = "std"), no_std)]

use frame_election_provider_support::{ElectionDataProvider, ElectionProvider};
use sp_npos_elections::Supports;
use frame_support::pallet_prelude::PhantomData;

pub struct OnChainSequentialPhragmen<T: Config>(PhantomData<T>);

pub trait Config: frame_system::Config {
	type ElectionProvider: ElectionProvider;
	// type DataProvider: ElectionDataProvider<AccountId=Self::AccountId, BlockNumber=Self::BlockNumber>;
}

impl<T: Config> ElectionProvider for OnChainSequentialPhragmen<T> {
	type AccountId = <T::ElectionProvider as ElectionProvider>::AccountId;
	type BlockNumber = <T::ElectionProvider as ElectionProvider>::BlockNumber ;
	type Error = <T::ElectionProvider as ElectionProvider>::Error ;
	type DataProvider = <T::ElectionProvider as ElectionProvider>::DataProvider; // T::DataProvider ;

	fn elect() -> Result<Supports<Self::AccountId>, Self::Error> {
		T::ElectionProvider::elect()
	}
	// fn elect() -> Result<Supports<T::AccountId>, Self::Error> {
	// 	T::ElectionProvider::elect()
	// }
}
