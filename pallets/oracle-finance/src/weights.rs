#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for oracle_finance.
pub trait WeightInfo {
	fn take_purchase_reward() -> Weight;
	fn take_all_purchase_reward() -> Weight;
}

/// Weights for oracle_finance using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	// Storage: OracleFinance RewardTrace (r:1 w:1)
	// Storage: OracleFinance CurrentEra (r:1 w:0)
	// Storage: OracleFinance AskEraPoint (r:3 w:0)
	// Storage: OracleFinance AskEraPayment (r:1 w:0)
	// Storage: OracleFinance RewardEra (r:1 w:1)
	fn take_purchase_reward() -> Weight {
		(82_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: OracleFinance RewardEra (r:1 w:1)
	// Storage: OracleFinance CurrentEra (r:1 w:0)
	// Storage: OracleFinance RewardTrace (r:1 w:1)
	// Storage: OracleFinance AskEraPoint (r:3 w:0)
	// Storage: OracleFinance AskEraPayment (r:1 w:0)
	fn take_all_purchase_reward() -> Weight {
		(80_000_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(7 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	// Storage: OracleFinance RewardTrace (r:1 w:1)
	// Storage: OracleFinance CurrentEra (r:1 w:0)
	// Storage: OracleFinance AskEraPoint (r:3 w:0)
	// Storage: OracleFinance AskEraPayment (r:1 w:0)
	// Storage: OracleFinance RewardEra (r:1 w:1)
	fn take_purchase_reward() -> Weight {
		(82_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
	// Storage: OracleFinance RewardEra (r:1 w:1)
	// Storage: OracleFinance CurrentEra (r:1 w:0)
	// Storage: OracleFinance RewardTrace (r:1 w:1)
	// Storage: OracleFinance AskEraPoint (r:3 w:0)
	// Storage: OracleFinance AskEraPayment (r:1 w:0)
	fn take_all_purchase_reward() -> Weight {
		(80_000_000 as Weight)
			.saturating_add(RocksDbWeight::get().reads(7 as Weight))
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
	}
}

