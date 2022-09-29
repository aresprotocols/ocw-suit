use super::*;

#[allow(unused)]
use crate::Pallet as AresChallenge;
// use frame_benchmarking::{benchmarks, whitelisted_caller, account};
use frame_benchmarking::{account, benchmarks_instance_pallet, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use frame_support::traits::Currency;


benchmarks_instance_pallet! {

	// new_challenge {
	// 	let block_hash: T::Hash = T::Hash::default();
	// 	let deposit: BalanceOf<T> = 10000u32.into();
	// 	let caller: T::AccountId = whitelisted_caller();
	// 	let validator: T::AccountId = account("validator", 0, 0);
	// 	let validator_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(validator.clone());
	// 	let delegatee: T::AccountId = pallet_collective::Members::<T, ()>::get()[0].clone();
	// 	let delegatee_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(delegatee.clone());
	//
	// 	T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value() / 100u32.into());
	// 	T::Currency::make_free_balance_be(&validator, BalanceOf::<T>::max_value() / 100u32.into());
	// 	T::Currency::make_free_balance_be(&delegatee, BalanceOf::<T>::max_value()  / 100u32.into());
	//
	// }: _(RawOrigin::Signed(caller.clone()), delegatee_lookup, validator_lookup, block_hash, deposit)
	// verify {
	// 	assert_eq!(<Proposals<T>>::iter().count(), 1);
	// }

	reserve {
		let deposit: BalanceOf<T, I> = 10000u32.into();
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T, I>::max_value() / 100u32.into());
	}: _(RawOrigin::Signed(caller.clone()), deposit)
	verify {
		assert_eq!(T::Currency::reserved_balance_named(&T::PalletId::get().0, &caller), deposit);
	}

	impl_benchmark_test_suite!(AresChallenge, crate::tests::tests::new_test_ext(), crate::tests::tests::Runtime);
}

#[cfg(test)]
mod tests {
	use frame_support::{assert_ok};

	#[test]
	fn start_benchmarking() {
		assert_eq!(1,1);
	}
}