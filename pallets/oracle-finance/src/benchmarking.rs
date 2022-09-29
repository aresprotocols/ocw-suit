use super::*;

#[allow(unused)]
use crate::Pallet as OracleFinance;
use frame_benchmarking::{benchmarks, whitelisted_caller, account};
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use frame_support::traits::Currency;
use crate::types::{BalanceOf, PurchaseId};
use core::convert::TryInto;
use sp_runtime::traits::Convert;
use sp_std::marker::PhantomData;


pub struct Test<T>(PhantomData<T>);

fn init_mock<T: Config> (caller: &T::AccountId, ask_era: &EraIndex) {
	T::Currency::make_free_balance_be(&caller, 1000u32.into());
	T::Currency::make_free_balance_be(&OracleFinance::<T>::account_id().unwrap(), 10000u32.into());

	// set current era.
	CurrentEra::<T>::put(ask_era);

	let pid_a: PurchaseId = "PID_A".as_bytes().to_vec().try_into().unwrap();
	let pid_b: PurchaseId = "PID_B".as_bytes().to_vec().try_into().unwrap();

	// OracleFinance::<T>::pay_to_ask(pid_a, 1u8 as usize);
	OracleFinance::<T>::record_submit_point(caller.clone(), pid_a, 1u32.into(), 100);
	// OracleFinance::<T>::pay_to_ask(pid_b, 1u8 as usize);
	OracleFinance::<T>::record_submit_point(caller.clone(), pid_b, 2u32.into(), 100);

	CurrentEra::<T>::put(ask_era + 1);
}

benchmarks! {

	take_purchase_reward {
		let caller: T::AccountId = whitelisted_caller();
		let ask_era: EraIndex =1;
		init_mock::<T>(&caller, &ask_era);
	}: _(RawOrigin::Signed(caller.clone()), ask_era)
	verify {
		assert_eq!(
			RewardTrace::<T>::iter().count(),
			1 as usize,
		);
	}

	take_all_purchase_reward {
		let caller: T::AccountId = whitelisted_caller();
		let ask_era: EraIndex =2;
		init_mock::<T>(&caller, &ask_era);
	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		assert_eq!(
			RewardTrace::<T>::iter().count(),
			1 as usize,
		);
	}

	impl_benchmark_test_suite!(OracleFinance, crate::mock::new_test_ext(), crate::mock::Test);
}

#[cfg(test)]
mod tests {
	use frame_support::{assert_ok};

	#[test]
	fn start_benchmarking() {
		assert_eq!(1,1);
	}
}