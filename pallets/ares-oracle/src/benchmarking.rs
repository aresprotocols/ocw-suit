#[cfg(any(feature = "runtime-benchmarks", test))]

use super::*;

#[allow(unused)]
use crate::Pallet as AresOracle;
use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use frame_support::traits::Currency;
use oracle_finance::types::{BalanceOf};
use crate::Config;
use sp_std::vec::Vec;
use crate::test_tools::to_test_vec;

// use crate::tests::Test;
// use crate::tests::new_test_ext;

fn init_mock<T: Config> () {
	let price_requests = vec![
		// price , key sign, version, fraction_length, request interval.
		(to_test_vec("btc_price"), to_test_vec("btc"), 2u32, 4u32, 1u8),
		(to_test_vec("eth_price"), to_test_vec("eth"), 2u32, 4u32, 2u8),
		(to_test_vec("dot_price"), to_test_vec("dot"), 2u32, 4u32, 3u8),
		(to_test_vec("xrp_price"), to_test_vec("xrp"), 2u32, 4u32, 4u8),
	];
	let price_request_list = price_requests.into_iter().map(|(price_key, price_token,version, fraction_length, request_interval )| {
		(
			PriceKey::create_on_vec(price_key),
			PriceToken::create_on_vec(price_token),
			version,
			fraction_length,
			request_interval,
		)
	}).collect::<Vec<_>>() ;
	PricesRequests::<T>::put(PricesRequestsVec::create_on_vec(price_request_list) );
}

benchmarks! {
	submit_ask_price {
		init_mock::<T>();
		let balance: BalanceOf<T> = BalanceOf::<T>::max_value();
		let caller: T::AccountId = whitelisted_caller();
		T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
	}: _(RawOrigin::Signed(caller.clone()), balance, "btc_price,eth_price".as_bytes().to_vec())
	verify {
		assert_eq!(<PurchasedRequestPool<T>>::iter().count(), 1);
	}
	impl_benchmark_test_suite!(
		AresOracle, crate::mock::new_test_ext(), crate::mock::Test
	);
}



#[cfg(test)]
mod tests {
	use frame_support::{assert_ok};

	#[test]
	fn start_benchmarking() {
		assert_eq!(1,1);
	}
}