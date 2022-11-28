use super::*;


pub trait ValidatorCount {
	fn get_validators_count() -> u64;
}

///
pub trait IsAresOracleCall<T: Config, Call>
{
	fn try_get_pallet_call(in_call: &Call) -> Option<&super::pallet::Call<T>>  ;
}

