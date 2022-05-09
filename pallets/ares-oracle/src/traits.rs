use sp_runtime::generic::UncheckedExtrinsic;
use super::*;
// use frame_support::weights::Weight;
// use frame_support::sp_runtime::Percent;

pub trait ValidatorCount {
	fn get_validators_count() -> u64;
}

pub trait SymbolInfo {
	fn price(symbol: &Vec<u8>) -> Result<(u64, FractionLength), ()>;

	fn fraction(symbol: &Vec<u8>) -> Option<FractionLength>;
}

///
pub trait IsAresOracleCall<T: Config, Call> {
	fn try_get_pallet_call(in_call: &Call) -> Option<&super::pallet::Call<T>>;
}