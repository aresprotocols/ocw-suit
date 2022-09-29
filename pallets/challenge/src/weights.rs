#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_ares_challenge<Instance1>.
pub trait WeightInfo {
	fn reserve() -> Weight;
}

/// Weights for pallet_ares_challenge<Instance1> using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T, I: 'static = ()>(PhantomData<(T, I)>);
impl<T: frame_system::Config, I> WeightInfo for SubstrateWeight<T, I> {
	// Storage: Balances Reserves (r:1 w:1)
	fn reserve() -> Weight {
		(32_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Balances Reserves (r:1 w:1)
	fn reserve() -> Weight {
		(32_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
}

