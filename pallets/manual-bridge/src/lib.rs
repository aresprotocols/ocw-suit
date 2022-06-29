#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;

#[frame_support::pallet]
pub mod pallet {
	use codec::KeyedVec;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Zero;
	use crate::types::BoundVecHelper;
	use crate::types::{BalanceOf, CrossChainInfo, CrossChainInfoList, CrossChainKind, EthereumAddress, Ident, MaximumPendingList};

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {

		/// The balance.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type RequestOrigin: EnsureOrigin<Self::Origin>;

		// type MinimumBalanceThreshold: Get<BalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn waiter_acc)]
	pub type WaiterAccout<T> = StorageValue<_, <T as frame_system::Config>::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn stash_acc)]
	pub type StashAccout<T> = StorageValue<_, <T as frame_system::Config>::AccountId>;


	#[pallet::storage]
	#[pallet::getter(fn pending_list)]
	pub type PendingList<T> = StorageMap<_, Twox64Concat, <T as frame_system::Config>::AccountId, CrossChainInfoList<T>>;

	#[pallet::storage]
	#[pallet::getter(fn minimum_balance_threshold)]
	pub type MinimumBalanceThreshold<T> =  StorageValue<_, BalanceOf<T>>;


	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {

		/// When the waiter account is updated
		WaiterUpdated { acc: T::AccountId },

		/// When the waiter account is updated
		StashUpdated { acc: T::AccountId },

		/// When the MinimumBalanceThreshold is updated
		MinimumBalanceThresholdUpdated { amount: BalanceOf<T> },

		/// Generate cross-connection requests
		CrossChainRequest {
			acc: T::AccountId,
			ident: Ident,
			kind: CrossChainKind,
			amount: BalanceOf<T>
		},

		/// Completed cross-chain requests
		CompletedList(CrossChainInfoList<T>)

	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Waiter does not exist, module has not completed initialization.
		WaiterDoesNotExists,
		/// Stash does not exist, module has not completed initialization.
		StashDoesNotExists,
		/// You need to set the MinimumBalanceThreshold parameter through sudo or committee.
		MinimumBalanceThresholdNotSet,
		/// The transfer amount must be greater than the threshold requirement.
		TransferAmountIsTooSmall,
		/// No permission
		NoPermission,
		/// Pending list is empty
		NoPendingList,
		/// The list data to be completed must all match, otherwise the completion operation cannot be performed.
		CompletedListDataCannotAllMatch,
		///
		StorageOverflow,
		///
		IllegalAddress,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(1_000)]
		pub fn update_waiter(origin: OriginFor<T>, waiter: T::AccountId) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			WaiterAccout::<T>::set(Some(waiter.clone()));
			// Emit an event.
			Self::deposit_event(Event::WaiterUpdated { acc: waiter });
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn update_stash(origin: OriginFor<T>, stash: T::AccountId) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			StashAccout::<T>::set(Some(stash.clone()));
			// Emit an event.
			Self::deposit_event(Event::StashUpdated { acc: stash });
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn update_minimum_balance_threshold(
			origin: OriginFor<T>,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			MinimumBalanceThreshold::<T>::set(Some(amount));
			// Emit an event.
			Self::deposit_event(Event::MinimumBalanceThresholdUpdated { amount });
			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn set_up_completed_list(origin: OriginFor<T>, sender: T::AccountId ,  list: CrossChainInfoList<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Get waiter
			let waiter = WaiterAccout::<T>::get();

			// Check origin is waiter
			ensure!(waiter == Some(who), Error::<T>::NoPermission);

			// Check that all data in the list match
			let pending_list = PendingList::<T>::get(sender.clone());
			ensure!(pending_list.is_some(), Error::<T>::NoPendingList);

			let mut pending_list = pending_list.unwrap();

			//
			let mut search_list = list.clone();
			let mut tmp_list = search_list.clone();

			// Delete pending exists.
			pending_list.retain(|pending_data|{
				let is_exists = tmp_list.iter().enumerate().any(|(idx, completed_data)|{
					// println!("pending_data = {:?}", &pending_data);
					// println!("completed_data = {:?}", &completed_data);
					let is_match = pending_data == completed_data;
					if is_match {
						search_list.remove(idx);
					}
					is_match
				});
				// reset tmp list
				tmp_list = search_list.clone();
				!is_exists
			});

			// The list data to be completed must all match, otherwise the completion operation cannot be performed.
			// ensure!(before_count - pending_list.len() == list.len(), Error::<T>::CompletedListDataCannotAllMatch);
			ensure!(search_list.len() == 0 as usize, Error::<T>::CompletedListDataCannotAllMatch);

			// Rewrite data to storage.
			PendingList::<T>::insert(sender, pending_list);

			// Emit an event.
			Self::deposit_event(Event::CompletedList(list));

			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn transfer_to(
			origin: OriginFor<T>,
			chain_kind: CrossChainKind,
			#[pallet::compact] amount: BalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			// Get waiter
			let stash = StashAccout::<T>::get();
			ensure!(stash.is_some(), Error::<T>::StashDoesNotExists);

			//
			let min_balance_threshold = MinimumBalanceThreshold::<T>::get();
			ensure!(min_balance_threshold.is_some(), Error::<T>::MinimumBalanceThresholdNotSet);
			let min_balance_threshold = min_balance_threshold.unwrap();

			// Check that the fund must be greater than the minimum threshold
			ensure!(amount >= min_balance_threshold, Error::<T>::TransferAmountIsTooSmall );

			// Check address is available
			ensure!(chain_kind.verification_addr(), Error::<T>::IllegalAddress );

			let pending_list = PendingList::<T>::get(who.clone());
			let mut pending_list = pending_list.unwrap_or(CrossChainInfoList::<T>::default());

			// TODO:: Change max count to ::bound()
			let max_count: u32 = MaximumPendingList::get();
			ensure!((pending_list.len() as u32) < max_count, Error::<T>::StorageOverflow );

			T::Currency::transfer(&who, &stash.unwrap(), amount, ExistenceRequirement::KeepAlive)?;

			// Get current blocknumber
			let current_bn: T::BlockNumber = <frame_system::Pallet<T>>::block_number();

			// let current_bn = 1u8;
			let ident: Option<Ident> = Ident::try_create_on_vec(current_bn.encode()).ok();
			ensure!(ident.is_some(), Error::<T>::StorageOverflow);

			let mut ident = ident.unwrap();

			let list_count = pending_list.len() as u32;
			ident.try_push(list_count as u8);

			pending_list.try_push(CrossChainInfo{
				iden: ident.clone(),
				kind: chain_kind.clone(),
				amount: amount,
			});

			PendingList::<T>::insert(who.clone(), pending_list);

			// Emit an event.
			Self::deposit_event(Event::CrossChainRequest {
				acc: who,
				ident: ident,
				kind: chain_kind,
				amount
			});
			Ok(())
		}

	}
}
