//! Election adapter between Staking and Election pallets.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_election_provider_support::{ElectionDataProvider, ElectionProvider};
use frame_support::pallet_prelude::PhantomData;
use sp_npos_elections::Supports;

pub trait Config: frame_system::Config {
	type ElectionProvider: ElectionProvider;

	type GenesisElectionProvider: ElectionProvider<
		AccountId = <Self::ElectionProvider as ElectionProvider>::AccountId,
		BlockNumber = <Self::ElectionProvider as ElectionProvider>::BlockNumber,
	>;

	type DataProvider: ElectionDataProvider<
		AccountId = <Self::ElectionProvider as ElectionProvider>::AccountId,
		BlockNumber = <Self::ElectionProvider as ElectionProvider>::BlockNumber,
	>;
}

/// Implement the structure of OnChainSequentialPhragmen.
pub struct OnChainSequentialPhragmen<T: Config>(PhantomData<T>);

impl<T: Config> ElectionProvider for OnChainSequentialPhragmen<T> {
	type AccountId = <T::ElectionProvider as ElectionProvider>::AccountId;
	type BlockNumber = <T::ElectionProvider as ElectionProvider>::BlockNumber;
	type Error = <T::ElectionProvider as ElectionProvider>::Error;
	type DataProvider = T::DataProvider;

	fn elect() -> Result<Supports<Self::AccountId>, Self::Error> {
		T::ElectionProvider::elect()
	}
}

/// Implement the structure of OnChainSequentialPhragmenGenesis.
pub struct OnChainSequentialPhragmenGenesis<T: Config>(PhantomData<T>);

impl<T: Config> ElectionProvider for OnChainSequentialPhragmenGenesis<T> {
	type AccountId = <T::GenesisElectionProvider as ElectionProvider>::AccountId;
	type BlockNumber = <T::GenesisElectionProvider as ElectionProvider>::BlockNumber;
	type Error = <T::GenesisElectionProvider as ElectionProvider>::Error;
	type DataProvider = T::DataProvider;

	fn elect() -> Result<Supports<Self::AccountId>, Self::Error> {
		T::GenesisElectionProvider::elect()
	}
}
