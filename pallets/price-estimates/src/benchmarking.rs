use super::*;

// #[allow(unused)]
use crate::Pallet as PriceEstimates;
use frame_benchmarking::{benchmarks, whitelisted_caller, account};
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use frame_support::traits::Currency;
use crate::{BalanceOf};
// use oracle_finance::types::{EraIndex, PurchaseId};
// use oracle_finance::Pallet as OracleFinance;
use core::convert::TryInto;
use sp_std::marker::PhantomData;
use ares_oracle::traits::SymbolInfo;
use bound_vec_helper::BoundVecHelper;
use crate::Config;
use crate::types::BoundedVecOfMultiplierOption;
use sp_std::vec;

pub struct BenchmarkingSymbolInfo<T>(PhantomData<T>) ;

impl <T :Config>SymbolInfo<T::BlockNumber> for BenchmarkingSymbolInfo<T> {
	fn price(symbol: &Vec<u8>) -> Result<(u64, FractionLength, T::BlockNumber), ()> {
		Ok(
			(23164822300, BenchmarkingSymbolInfo::<T>::fraction(symbol).unwrap(), 50u32.into())
		)
	}
	fn fraction(symbol: &Vec<u8>) -> Option<FractionLength> {
		Some(6)
	}
}

fn init_mock<T: Config> (caller: &T::AccountId) {

	T::Currency::make_free_balance_be(caller, 10000u32.into());
	frame_system::Pallet::<T>::set_block_number(0u32.into());
	let symbol = "btc-usdt".as_bytes().to_vec();

	let lock_number: T::BlockNumber = 1u32.into();
	LockedEstimates::<T>::put(lock_number);

	// System::set_block_number(0);
	let start: T::BlockNumber = 5u32.into(); //     start: T::BlockNumber,
	let end: T::BlockNumber = 10u32.into(); //     end: T::BlockNumber,
	let distribute: T::BlockNumber = 15u32.into(); //     distribute: T::BlockNumber,
	let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
	let deviation = Some(Permill::from_percent(10)); // Some(Permill::from_percent(10)); //     deviation: Option<Permill>,
	let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
	let multiplier: Vec<MultiplierOption> = vec![
		MultiplierOption::Base(1),
		MultiplierOption::Base(3),
		MultiplierOption::Base(5),
	];
	let ticket_price: BalanceOf<T> = 1000u32.into();

	// Get and check subaccount.
	let source_acc = PriceEstimates::<T>::account_id((BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type.clone()));
	let source_acc = source_acc.unwrap();

	let _res = T::Currency::transfer(
		caller,
		&source_acc,
		ticket_price,
		ExistenceRequirement::KeepAlive,
	);

	let config: SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>> = SymbolEstimatesConfig{
		symbol: BoundedVecOfSymbol::create_on_vec(symbol.clone()),
		estimates_type: estimates_type.clone(),
		id: 0,
		ticket_price: 1000u32.into(),
		symbol_completed_price: 0,
		symbol_fraction: BenchmarkingSymbolInfo::<T>::fraction(&symbol.clone()).unwrap(),
		start,
		end,
		distribute,
		multiplier: BoundedVecOfMultiplierOption::create_on_vec(multiplier),
		deviation,
		range: None, //BoundedVecOfConfigRange::create_on_vec(range),
		total_reward: 0u32.into(),
		state: EstimatesState::Active,
	};
	let storage_key = (BoundedVecOfSymbol::create_on_vec(symbol.clone()), estimates_type);
	ActiveEstimates::<T>::insert(&storage_key, config);
	EstimatesInitDeposit::<T>::insert(storage_key.clone(), 0, ticket_price);

	frame_system::Pallet::<T>::set_block_number(start);
}

benchmarks! {
	participate_estimates {

		let caller: T::AccountId = whitelisted_caller();
		init_mock::<T>(&caller);
		let symbol = "btc-usdt".as_bytes().to_vec();
		let estimated_type = EstimatesType::DEVIATION;
		let estimated_price = Some(100000u64);
		let estimated_fraction_length = Some(4u32);
		let range_index = None;
		let multiplier: MultiplierOption = MultiplierOption::Base(3);
		let bsc_address = None;

	}: _(
		RawOrigin::Signed(caller.clone()),
		symbol,
		estimated_type,
		estimated_price,
		estimated_fraction_length,
		range_index,
		multiplier, bsc_address
	)
	verify {
		assert_eq!(
			Participants::<T>::iter().count(),
			1u8 as usize,
		);
	}

	impl_benchmark_test_suite!(PriceEstimates, crate::mock::new_test_ext(), crate::mock::Test);
}

#[cfg(test)]
mod tests {
	use frame_support::{assert_ok};

	#[test]
	fn start_benchmarking() {
		assert_eq!(1,1);
	}
}