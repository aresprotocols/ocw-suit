#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{Currency, Get, ReservableCurrency, ExistenceRequirement};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;
use crate::traits::{IForPrice, OcwPaymentResult};
use crate::types::{PurchaseId, BalanceOf};
use sp_runtime::traits::Saturating;
use frame_support::sp_runtime::traits::AccountIdConversion;



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

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
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
		OcwPaymentResult::Success(p_id, reserve_balance)
	}

	fn refund_ask_paid(p_id: PurchaseId) -> bool {
		todo!()
	}
}