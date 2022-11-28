#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for manual_bridge.
pub trait WeightInfo {
	fn transfer_to() -> Weight;
}

/// Weights for manual_bridge using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: ManualBridge StashAccout (r:1 w:0)
	// Storage: ManualBridge MinimumBalanceThreshold (r:1 w:0)
	// Storage: ManualBridge PendingList (r:1 w:1)
	fn transfer_to() -> Weight {
		(33_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: ManualBridge StashAccout (r:1 w:0)
	// Storage: ManualBridge MinimumBalanceThreshold (r:1 w:0)
	// Storage: ManualBridge PendingList (r:1 w:1)
	fn transfer_to() -> Weight {
		(33_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
	}
}

