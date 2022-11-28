use super::*;

#[allow(unused)]
use crate::Pallet as ManualBridge;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use crate::types::EthereumAddress;
use crate::types::CrossChainKind;
use crate::types::BalanceOf;
use sp_runtime::traits::Bounded;
use frame_support::traits::Currency;
use crate::Config;

fn init_mock<T: Config> () {
	let stash_acc: T::AccountId = whitelisted_caller();
	ManualBridge::<T>::update_stash(RawOrigin::Root.into(), stash_acc);
	ManualBridge::<T>::update_minimum_balance_threshold(RawOrigin::Root.into(), 10u32.into());
}

benchmarks! {
	transfer_to {
		init_mock::<T>();
		let caller: T::AccountId = whitelisted_caller();
		let balance: BalanceOf<T> = 1000u32.into();
		let eth_add: EthereumAddress = EthereumAddress::new([3u8; 20]);
		// let _ = <Balances<T, I> as Currency<_>>::make_free_balance_be(&caller, T::Balance::max_value());
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
	}: _(RawOrigin::Signed(caller.clone()), CrossChainKind::ETH(eth_add), balance)
	verify {
		assert!(PendingList::<T>::get(caller.clone()).is_some());
		assert_eq!(PendingList::<T>::get(caller.clone()).unwrap().len(), 1);
	}
	impl_benchmark_test_suite!(ManualBridge, crate::mock::new_test_ext(), crate::mock::Test);
}

#[cfg(test)]
mod tests {
	use frame_support::{assert_ok};

	#[test]
	fn start_benchmarking() {
		assert_eq!(1,1);
	}
}