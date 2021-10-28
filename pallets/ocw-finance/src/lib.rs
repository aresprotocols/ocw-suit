#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{Currency, Get, ReservableCurrency, ExistenceRequirement};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;
use crate::traits::*;
use crate::types::*;
use sp_runtime::traits::Saturating;
use frame_support::sp_runtime::traits::{AccountIdConversion, Clear};
use frame_support::sp_std::convert::TryInto;


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod traits;
mod types;


#[frame_support::pallet]
pub mod pallet {
	use frame_support::traits::{Currency, ReservableCurrency};
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;
	use crate::types::*;
	use frame_support::sp_runtime::traits::Zero;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The staking balance.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		#[pallet::constant]
		type BasicDollars: Get<BalanceOf<Self>>;

		///
		#[pallet::constant]
		type AskPeriod: Get<Self::BlockNumber>;

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn something)]
	pub type Something<T> = StorageValue<_, u32>;

	#[pallet::storage]
	#[pallet::getter(fn payment_trace)]
	pub type PaymentTrace<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PurchaseId, // purchased_id,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId, // pay or pay to account. ,,
		PaidValue<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ask_period_payment)]
	pub type AskPeriodPayment<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AskPeriodNum, // ,
		Blake2_128Concat,
		(<T as frame_system::Config>::AccountId, PurchaseId), // pay or pay to account. ,,
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ask_period_point)]
	pub type AskPeriodPoint<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AskPeriodNum, //,
		Blake2_128Concat,
		(<T as frame_system::Config>::AccountId, PurchaseId), // pay or pay to account. ,,
		AskPointNum,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub(crate) _pt: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				_pt: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			let finance_account = <Pallet<T>>::account_id();
			if T::Currency::total_balance(&finance_account).is_zero() {
				T::Currency::deposit_creating(&finance_account, T::Currency::minimum_balance());
				// Self::deposit_event(Event::<T>::OcwFinanceDepositCreating(T::Currency::minimum_balance()));
			}
		}
	}



	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		SomethingStored(u32, T::AccountId),
		OcwFinanceDepositCreating(BalanceOf<T>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	#[derive(PartialEq, Eq)]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		//
		NotFoundPaymentRecord,
		//
		RefundFailed,
		//
		PointRecordIsAlreadyExists,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			// let finance_account = Self::account_id();
			// if T::Currency::total_balance(&finance_account).is_zero() {
			// 	T::Currency::deposit_creating(&finance_account, T::Currency::minimum_balance());
			// 	Self::deposit_event(Event::<T>::OcwFinanceDepositCreating(T::Currency::minimum_balance()));
			// }
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://substrate.dev/docs/en/knowledgebase/runtime/origin
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored(something, who));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResult {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue)?,
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(())
				}
			}
		}
	}
}

impl<T: Config> Pallet<T> {

	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	fn pot() -> (T::AccountId, BalanceOf<T>) {
		let account_id = Self::account_id();
		let balance =
			T::Currency::free_balance(&account_id).saturating_sub(T::Currency::minimum_balance());
		(account_id, balance)
	}
}

impl <T: Config> IForPrice<T> for Pallet<T> {

	fn calculate_fee_of_ask_quantity(price_count: u32) -> BalanceOf<T> {
		T::BasicDollars::get().saturating_mul(price_count.into())
	}

	fn payment_for_ask_quantity(who: <T as frame_system::Config>::AccountId, p_id: PurchaseId, price_count: u32) -> OcwPaymentResult<T> {
		let reserve_balance = Self::calculate_fee_of_ask_quantity(price_count);
		// T::Currency::reserve(&account_id, reserve_balance.clone());
		let res = T::Currency::transfer(
			&who,
			&Self::account_id(),
			reserve_balance,
			ExistenceRequirement::KeepAlive,
		);

		if res.is_err() {
			return OcwPaymentResult::InsufficientBalance(p_id.clone(), reserve_balance.clone());
		}

		let current_block_number = <frame_system::Pallet<T>>::block_number();

		<PaymentTrace<T>>::insert(
			p_id.clone(),
			who.clone(),
			PaidValue {
				create_bn: current_block_number,
				amount: reserve_balance,
				is_income: true,
			},
		);
		let ask_period = Self::make_period_num(current_block_number);
		<AskPeriodPayment<T>>::insert(ask_period, (who.clone(), p_id.clone()), reserve_balance);

		OcwPaymentResult::Success(p_id, reserve_balance)
	}

	fn refund_ask_paid(p_id: PurchaseId) -> Result<(), Error<T>> {

		let payment_list = <PaymentTrace<T>>::iter_prefix(p_id.clone());
		// if payment_list.into_iter()..count() == 0 as usize {
		// 	return Err(Error::NotFoundPaymentRecord);
		// }
		let mut find_pid = false;
		let is_success = payment_list.into_iter().into_iter().any(|(who, paid_value)| {
			if paid_value.is_income {
				let res = T::Currency::transfer(
					&Self::account_id(),
					&who,
					paid_value.amount,
					ExistenceRequirement::KeepAlive,
				);
				if res.is_ok() {
					<PaymentTrace<T>>::remove(p_id.clone(), who.clone());
					let ask_period = Self::make_period_num(paid_value.create_bn);
					<AskPeriodPayment<T>>::remove(ask_period, (who.clone(), p_id.clone()));
					return true;
				}
				find_pid = true;
			}
			false
		});

		if is_success {
			return Ok(());
		}

		if false == find_pid {
			return Err(Error::NotFoundPaymentRecord);
		}
		Err(Error::RefundFailed)
	}
}

impl <T: Config> IForReporter<T> for Pallet<T> {

	//
	fn record_submit_point(who: T::AccountId, p_id: PurchaseId, bn: T::BlockNumber, ask_point: u64) -> Result<(), Error<T>> {
		// (period, who, (purchase_id, price_count) )
		let ask_period = Self::make_period_num(bn);
		// println!("(who.clone(), p_id.clone()) = {:?}", (who.clone(), p_id.clone()));
		if <AskPeriodPoint<T>>::contains_key(ask_period, (who.clone(), p_id.clone())) {
			return Err(Error::<T>::PointRecordIsAlreadyExists);
		}
		<AskPeriodPoint<T>>::insert(ask_period, (who.clone(), p_id), ask_point);
		Ok(())
	}
}


impl <T: Config> IForBase<T> for Pallet<T> {
	//
	fn make_period_num(bn: T::BlockNumber) -> AskPeriodNum {
		let param_bn: u64 = bn.try_into().unwrap_or(0);
		let ask_perio: u64 = T::AskPeriod::get().try_into().unwrap_or(0);
		if 0 == param_bn || 0 == ask_perio {
			return 0;
		}
		param_bn / ask_perio
	}
}

impl <T: Config> IForReward<T> for Pallet<T> {
	//
	fn take_reward(ask_period: AskPeriodNum, who: T::AccountId) -> Result<(), Error<T>> {
		// get his point.

		todo!()
	}

	fn get_earliest_reward_period(bn: T::BlockNumber) -> AskPeriodNum {
		todo!()
	}

	//
	fn get_sum_of_record_point(ask_period: AskPeriodNum) -> AskPointNum {
		<AskPeriodPoint<T>>::iter_prefix_values(ask_period).into_iter().sum()
	}

	//
	fn check_and_slash_expired_rewards(ask_period: AskPeriodNum) -> Option<BalanceOf<T>> {
		todo!()
	}
}
