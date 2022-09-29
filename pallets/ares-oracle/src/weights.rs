#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for ares_oracle.
pub trait WeightInfo {
	fn submit_ask_price() -> Weight;
}

/// Weights for ares_oracle using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: AresOracle PurchasedDefaultSetting (r:1 w:0)
	// Storage: AresOracle PurchasedRequestPool (r:1 w:1)
	// Storage: AresOracle PricesRequests (r:1 w:0)
	// Storage: OracleFinance CurrentEra (r:1 w:0)
	// Storage: OracleFinance PaymentTrace (r:0 w:1)
	fn submit_ask_price() -> Weight {
		(51_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: AresOracle PurchasedDefaultSetting (r:1 w:0)
	// Storage: AresOracle PurchasedRequestPool (r:1 w:1)
	// Storage: AresOracle PricesRequests (r:1 w:0)
	// Storage: OracleFinance CurrentEra (r:1 w:0)
	// Storage: OracleFinance PaymentTrace (r:0 w:1)
	fn submit_ask_price() -> Weight {
		(51_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
}

