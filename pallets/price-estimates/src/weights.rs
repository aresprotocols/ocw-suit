#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_price_estimates.
pub trait WeightInfo {
	fn participate_estimates() -> Weight;
}

/// Weights for pallet_price_estimates using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: Estimates ActivePallet (r:1 w:0)
	// Storage: AresOracle SymbolFraction (r:1 w:0)
	// Storage: Estimates ActiveEstimates (r:1 w:0)
	// Storage: Estimates LockedEstimates (r:1 w:0)
	// Storage: Estimates Participants (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Estimates EstimatesInitDeposit (r:1 w:1)
	fn participate_estimates() -> Weight {
		(72_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: Estimates ActivePallet (r:1 w:0)
	// Storage: AresOracle SymbolFraction (r:1 w:0)
	// Storage: Estimates ActiveEstimates (r:1 w:0)
	// Storage: Estimates LockedEstimates (r:1 w:0)
	// Storage: Estimates Participants (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: Estimates EstimatesInitDeposit (r:1 w:1)
	fn participate_estimates() -> Weight {
		(72_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
	}
}

