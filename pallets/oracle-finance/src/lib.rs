//! A module for controlling the oracle down payment model

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use crate::traits::*;
use crate::types::*;
use frame_support::sp_runtime::traits::AccountIdConversion;
use frame_support::traits::{Currency, ExistenceRequirement, Get, LockableCurrency, LockIdentifier, OnUnbalanced, ReservableCurrency, WithdrawReasons};
use pallet_session::SessionManager;
use sp_runtime::traits::{Saturating, Zero};
use sp_std::vec::Vec;
use ares_oracle_provider_support::{OrderIdEnum};

#[cfg(all(feature = "std", test))]
mod mock;

#[cfg(all(feature = "std", test))]
mod tests;

/// Modules that provide interface definitions.
pub mod traits;
/// Defines the underlying data types required by the Pallet but does not include storage.
pub mod types;

#[cfg(any(feature = "runtime-benchmarks"))]
mod benchmarking;

#[cfg(all(feature = "std", test))]
mod test_tools;

pub mod weights;
pub mod migrations;

#[frame_support::pallet]
pub mod pallet {
	use codec::FullCodec;
	use crate::traits::{IForReward};
	use crate::types::*;
	use frame_support::sp_runtime::traits::Zero;
	use frame_support::traits::{Currency, LockableCurrency, LockIdentifier, OnUnbalanced, ReservableCurrency};
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use pallet_session::SessionManager;
	use sp_runtime::traits::Convert;
	use sp_std::convert::TryFrom;
	use sp_std::convert::TryInto;
	use sp_std::fmt::Debug;
	use ares_oracle_provider_support::{OrderIdEnum, PurchaseId};
	use crate::weights::WeightInfo;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The staking balance.
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId> + LockableCurrency<Self::AccountId>;

		#[pallet::constant]
		type BasicDollars: Get<BalanceOf<Self, I>>;

		#[pallet::constant]
		type AskPerEra: Get<SessionIndex>;

		type ValidatorId: Member
			+ Parameter
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ TryFrom<Self::AccountId>;

		type ValidatorIdOf: Convert<Self::AccountId, Option<Self::AccountId>>;

		/// Handler for managing new session.
		type SessionManager: SessionManager<Self::ValidatorId>;

		#[pallet::constant]
		type HistoryDepth: Get<u32>;

		#[pallet::constant]
		type LockIdentifier: Get<LockIdentifier>;

		type OnSlash: OnUnbalanced<NegativeImbalanceOf<Self, I>>;

		type WeightInfo: WeightInfo;

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	// #[pallet::generate_storage_info]
	pub struct Pallet<T, I = ()>(_);

	#[pallet::storage]
	pub type StorageVersion<T: Config<I>, I: 'static = ()> = StorageValue<_, Releases, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn order_locked_deposit)]
	pub type OrderLockedDeposit<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		OrderIdEnum,
		(<T as frame_system::Config>::AccountId, BalanceOf<T, I>),
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn account_locked_deposit)]
	pub type AccountLockedDeposit<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BalanceOf<T, I>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn payment_trace)]
	pub type PaymentTrace<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		OrderIdEnum, // purchased_id,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId, // pay or pay to account. ,,
		PaidValue<<T as frame_system::Config>::BlockNumber, BalanceOf<T, I>, EraIndex>,
		ValueQuery,
	>;

	// pub type Proposals<T: Config<I>, I: 'static = ()>

	#[pallet::storage]
	#[pallet::getter(fn ask_era_payment)]
	pub type AskEraPayment<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		EraIndex, // ,
		Blake2_128Concat,
		(<T as frame_system::Config>::AccountId, OrderIdEnum), // pay or pay to account. ,,
		BalanceOf<T, I>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ask_era_point)]
	pub type AskEraPoint<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		EraIndex, //,
		Blake2_128Concat,
		(<T as frame_system::Config>::AccountId, OrderIdEnum), // pay or pay to account. ,,
		AskPointNum,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn current_era)]
	pub type CurrentEra<T: Config<I>, I: 'static = ()> = StorageValue<_, EraIndex>;

	#[pallet::storage]
	#[pallet::getter(fn eras_start_session_index)]
	pub type ErasStartSessionIndex<T: Config<I>, I: 'static = ()> = StorageMap<_, Twox64Concat, EraIndex, SessionIndex>;

	#[pallet::storage]
	#[pallet::getter(fn reward_era)]
	pub type RewardEra<T: Config<I>, I: 'static = ()> = StorageMap<
		_,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId,                                    //,
		BoundedVec<(EraIndex, AskPointNum, OrderIdEnum), MaximumRewardEras>, // pay or pay to account. ,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn reward_trace)]
	pub type RewardTrace<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		EraIndex, //,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId,
		(<T as frame_system::Config>::BlockNumber, BalanceOf<T, I>),
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
		pub _pt: PhantomData<(T, I)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config<I>, I: 'static> Default for GenesisConfig<T,I> {
		fn default() -> Self {
			Self {
				_pt: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
		fn build(&self) {
			let finance_account = <Pallet<T, I>>::account_id().unwrap();
			if T::Currency::total_balance(&finance_account).is_zero() {
				log::info!("oracle-finance deposit creating, {:?}", T::Currency::minimum_balance());
				T::Currency::deposit_creating(&finance_account, T::Currency::minimum_balance());
			}
			StorageVersion::<T,I>::put(Releases::V1);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		PayForPurchase {
			agg_count: u32,
			dest: T::AccountId,
			fee: BalanceOf<T, I>,
			purchase_id: OrderIdEnum,
			unreserve_balance: BalanceOf<T, I>,
			who: T::AccountId,
		},
		PurchaseRewardToken {
			created_at: T::BlockNumber,
			era: EraIndex,
			who: T::AccountId,
			reward: BalanceOf<T, I>,
		},
		PurchaseRewardSlashedAfterExpiration {
			created_at: T::BlockNumber,
			era: EraIndex,
			slash: BalanceOf<T, I>,
		},
		// CreatePurchaseOrder {
		// 	era: EraIndex,
		// 	pid: OrderIdEnum,
		// 	reserve_balance: BalanceOf<T, I>,
		// 	who: T::AccountId,
		// },
		EndOfAskEra {
			era: EraIndex,
			era_income: BalanceOf<T, I>,
			era_points: AskPointNum,
			session_index: SessionIndex,
		},
		ReserveFee {
			order_id: OrderIdEnum,
			who: T::AccountId,
			amount: BalanceOf<T, I>,
		},
		UnreserveFee {
			order_id: OrderIdEnum,
			who: T::AccountId,
			amount: BalanceOf<T, I>,
		},
		LockDeposit {
			order_id: OrderIdEnum,
			who: T::AccountId,
			amount: BalanceOf<T, I>,
		},
		UnlockDeposit {
			order_id: OrderIdEnum,
			who: T::AccountId,
			amount: BalanceOf<T, I>,
		},
		TraceLog {
			order_id: OrderIdEnum,
			who: T::AccountId,
			paid_value: PaidValue<<T as frame_system::Config>::BlockNumber, BalanceOf<T, I>, EraIndex>,
		},
		TraceCount {
			count: u64,
		},
	}

	// Errors inform users that something went wrong.
	#[derive(PartialEq, Eq)]
	#[pallet::error]
	pub enum Error<T, I = ()> {
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
		//
		RewardSlotNotExpired,
		//
		RewardEraHasExpired,
		//
		NoRewardPoints,
		//
		RewardHasBeenClaimed,
		//
		UnReserveBalanceError,
		//
		TransferBalanceError,
		//
		VectorIsFull,
		//
		NoStashAccount,
		//
		NoPotAccount,
		//
		CalculateFeeError,
		//
		NotFoundLockedDeposit,
		//
		LiquidityInsufficient,
	}

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			0
		}
	}

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {

		#[pallet::weight(0)]
		pub fn trace_log(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			PaymentTrace::<T, I>::iter().for_each(|(k1,k2,v)|{
				Self::deposit_event(Event::TraceLog {
					order_id: k1,
					who: k2,
					paid_value: v
				});
			});

			Self::deposit_event(Event::TraceCount {
				count: PaymentTrace::<T, I>::iter().count() as u64
			});

			Ok(())
		}

		/// Validators get rewards corresponding to eras.
		///
		/// _Note_: An ear cannot be the current unfinished era, and rewards are not permanently stored.
		/// If the reward exceeds the depth defined by T::HistoryDepth, you will not be able to claim it.
		///
		/// The dispatch origin for this call must be _Signed_
		///
		/// Earliest reward Era = Current-Era - T::HistoryDepth
		///
		/// - ask_era: Era number is a u32
		#[pallet::weight(<T as Config<I>>::WeightInfo::take_purchase_reward())]
		pub fn take_purchase_reward(origin: OriginFor<T>, ask_era: EraIndex) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let who = T::ValidatorIdOf::convert(controller.clone());
			log::debug!(
				"Call take_purchase_reward, Controller = {:?}, Stash = {:?}",
				&controller.clone(), &who.clone(),
			);
			ensure!(who.is_some(), Error::<T, I>::NoStashAccount);
			let who: T::AccountId = who.unwrap();

			let take_balance: BalanceOf<T, I> = Self::take_reward(ask_era, who.clone())?;

			let current_block_number = <frame_system::Pallet<T>>::block_number();
			// Emit an event.
			Self::deposit_event(Event::PurchaseRewardToken {
				created_at: current_block_number,
				era: ask_era,
				who,
				reward: take_balance,
			});
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		/// Validators get rewards, it will help validators get all available rewards
		///
		/// _Note_: An ear cannot be the current unfinished era, and rewards are not permanently stored.
		/// If the reward exceeds the depth defined by T::HistoryDepth, you will not be able to claim it.
		///
		/// The dispatch origin for this call must be _Signed_
		///
		/// Earliest reward Era = Current-Era - T::HistoryDepth
		#[pallet::weight(<T as Config<I>>::WeightInfo::take_all_purchase_reward())]
		pub fn take_all_purchase_reward(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let who = T::ValidatorIdOf::convert(controller.clone());
			log::debug!(
				"Call take_all_purchase_reward, Controller = {:?}, Stash = {:?}",
				&controller.clone(), &who.clone(),
			);
			ensure!(who.is_some(), Error::<T, I>::NoStashAccount);
			let who: T::AccountId = who.unwrap();

			let reward_era_list:BoundedVec<(EraIndex, AskPointNum, OrderIdEnum), MaximumRewardEras>
				= RewardEra::<T, I>::get(who.clone());

			ensure!(!reward_era_list.is_empty(), Error::<T, I>::NoRewardPoints);

			let current_era_opt = CurrentEra::<T, I>::get();
			let mut era_list = Vec::new();
			for (ask_era, _ask_point_num, _pid) in reward_era_list {

				if !era_list.iter().any(|exists_era|{
					&ask_era == exists_era
				}) {
					if let Some(current_era) = current_era_opt {
						if ask_era <  current_era {
							era_list.push(ask_era);
						}
					}
				}
			}

			let current_block_number = <frame_system::Pallet<T>>::block_number();
			for ask_era in era_list {
				let take_balance: BalanceOf<T, I> = Self::take_reward(ask_era, who.clone())?;
				// Emit an event.
				Self::deposit_event(Event::PurchaseRewardToken {
					created_at: current_block_number,
					era: ask_era,
					who: who.clone(),
					reward: take_balance,
				});
			}

			Ok(())
		}
	}
}

// Balance, Error
impl<T: Config<I>, I: 'static> IForPrice<T, I> for Pallet<T, I> {

	fn calculate_fee(price_count: u32) -> BalanceOf<T, I> {
		T::BasicDollars::get().saturating_mul(price_count.into())
	}

	fn reserve_fee(
		who: &<T as frame_system::Config>::AccountId,
		p_id: &OrderIdEnum,
		price_count: u32,
	) -> OcwPaymentResult<BalanceOf<T, I>, OrderIdEnum> {
		let reserve_balance = Self::calculate_fee(price_count);
		let res = T::Currency::reserve(who, reserve_balance);
		if res.is_err() {
			return OcwPaymentResult::InsufficientBalance(p_id.clone(), reserve_balance.clone());
		}
		let current_block_number = <frame_system::Pallet<T>>::block_number();
		<PaymentTrace<T, I>>::insert(
			p_id.clone(),
			who.clone(),
			PaidValue {
				create_bn: current_block_number,
				paid_era: Self::current_era_num(),
				amount: reserve_balance.clone(),
				is_income: true,
			},
		);

		Self::deposit_event(Event::ReserveFee {
			order_id: p_id.clone(),
			who: who.clone(),
			amount: reserve_balance,
		});

		OcwPaymentResult::Success(p_id.clone(), reserve_balance)
	}

	fn unreserve_fee(p_id: &OrderIdEnum) -> Result<(), Error<T, I>> {
		let payment_list = <PaymentTrace<T, I>>::iter_prefix(p_id);
		// if payment_list.into_iter()..count() == 0 as usize {
		// 	return Err(Error::NotFoundPaymentRecord);
		// }
		let mut find_pid = false;
		let is_success = payment_list.into_iter().into_iter().any(|(who, paid_value)| {
			if paid_value.is_income {
				find_pid = true;

				let res = T::Currency::unreserve(&who, paid_value.amount);

				Self::deposit_event(Event::UnreserveFee {
					order_id: p_id.clone(),
					who: who.clone(),
					amount: paid_value.amount,
				});

				if res.is_zero() {
					<PaymentTrace<T, I>>::remove(p_id, &who);
					// let ask_era = Self::current_era_num(paid_value.create_bn);
					// <AskEraPayment<T, I>>::remove(ask_era, (who.clone(), p_id.clone()));
					return true;
				}
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

	fn get_reserve_fee(p_id: &OrderIdEnum) -> BalanceOf<T, I> {
		let payment_list = <PaymentTrace<T, I>>::iter_prefix(p_id);

		let mut opt_paid_value: Option<(T::AccountId, PaidValue<T::BlockNumber, BalanceOf<T, I>, EraIndex>)> = None;
		payment_list.into_iter().into_iter().any(|(who, paid_value)| {
			if paid_value.is_income {
				opt_paid_value = Some((who, paid_value));
			}
			false
		});

		if let Some((_who, paid_value)) = opt_paid_value {
			paid_value.amount
		}else{
			0u32.into()
		}
	}

	/// Lock up deposits, for guarantees, etc, which will not be paid.
	///
	/// - p_id: Purchase order id.
	fn lock_deposit(who: &T::AccountId, pid: &OrderIdEnum, order_deposit: BalanceOf<T, I>) -> Result<(), Error<T, I>> {

		// Check for old data
		let locked_infos = <OrderLockedDeposit<T, I>>::get(pid);

		// Get old locked deposit.
		let old_locked_deposit: BalanceOf<T, I> = Self::get_all_locked_deposit_with_acc(who);
		let mut all_deposit = old_locked_deposit;

		// Check empty deposit.
		if locked_infos.is_some() && order_deposit == Zero::zero() {
			return Self::unlock_deposit(pid);
		} else if locked_infos.is_some() {
			let old_order_deposit = if let Some((_, deposit)) = locked_infos {
				deposit
			}else{
				0u32.into()
			};

			if order_deposit > old_order_deposit {
				all_deposit = all_deposit.saturating_add(order_deposit.saturating_sub(old_order_deposit));
			}else if order_deposit < old_order_deposit {
				all_deposit = all_deposit.saturating_sub(old_order_deposit.saturating_sub(order_deposit));
			}else{
				return Ok(());
			}
		}else{
			all_deposit = all_deposit.saturating_add(order_deposit);
		}

		// Check free balance
		let freee_balance = T::Currency::free_balance(who);
		if freee_balance < all_deposit {
			return Err(Error::<T, I>::LiquidityInsufficient);
		}

		// Get lock identifier infos
		let lock_id: LockIdentifier = T::LockIdentifier::get();
		// Lock balance
		T::Currency::extend_lock(lock_id, who, all_deposit, WithdrawReasons::all());
		// Update storage
		<AccountLockedDeposit<T, I>>::insert(who, all_deposit);
		<OrderLockedDeposit<T, I>>::insert(pid, (who, order_deposit));

		Self::deposit_event(Event::LockDeposit {
			order_id: pid.clone(),
			who: who.clone(),
			amount: order_deposit,
		});

		Ok(())
	}

	/// Unlock deposits.
	///
	/// - p_id: Purchase order id.
	fn unlock_deposit(pid: &OrderIdEnum) -> Result<(), Error<T, I>>{
		let locked_infos = <OrderLockedDeposit<T, I>>::get(&pid);
		if let Some(locked_data) = locked_infos {
			let all_deposit = Self::get_all_locked_deposit_with_acc(&locked_data.0);
			let new_all_deposit = all_deposit.saturating_sub(locked_data.1.clone());

			if new_all_deposit == Zero::zero() {
				<AccountLockedDeposit<T, I>>::remove(&locked_data.0);
				T::Currency::remove_lock(T::LockIdentifier::get(), &locked_data.0);
			}else{
				<AccountLockedDeposit<T, I>>::insert(&locked_data.0, new_all_deposit);
				T::Currency::remove_lock(T::LockIdentifier::get(), &locked_data.0);
				T::Currency::extend_lock(T::LockIdentifier::get(), &locked_data.0, new_all_deposit, WithdrawReasons::all());
			}
			<OrderLockedDeposit<T, I>>::remove(&pid);

			Self::deposit_event(Event::UnlockDeposit {
				order_id: pid.clone(),
				who: locked_data.0.clone(),
				amount: locked_data.1.clone(),
			});

			return Ok(());
		}
		return Err(Error::<T, I>::NotFoundLockedDeposit)
	}

	/// Unlock deposits.
	///
	/// - p_id: Purchase order id.
	fn get_locked_deposit(pid: &OrderIdEnum) -> BalanceOf<T, I> {
		let locked_infos = <OrderLockedDeposit<T, I>>::get(pid);
		if let Some(locked_infos) = locked_infos {
			return locked_infos.1;
		}
		0u32.into()
	}

	fn get_all_locked_deposit_with_acc(who: &T::AccountId) -> BalanceOf<T, I> {
		return AccountLockedDeposit::<T, I>::get(who).unwrap_or(0u32.into());
	}

	fn pay_to(p_id: &OrderIdEnum, agg_count: usize) -> Result<(), Error<T, I>> {
		// Get pot account
		let pot_account = Self::account_id();
		if pot_account.is_none() {
			return Err(Error::<T, I>::NoPotAccount);
		}
		let pot_account = pot_account.unwrap();

		let payment_list = <PaymentTrace<T, I>>::iter_prefix(p_id);

		let mut opt_paid_value: Option<(T::AccountId, PaidValue<T::BlockNumber, BalanceOf<T, I>, EraIndex>)> = None;
		payment_list.into_iter().into_iter().any(|(who, paid_value)| {
			if paid_value.is_income {
				opt_paid_value = Some((who, paid_value));
			}
			false
		});

		if opt_paid_value.is_none() {
			return Err(Error::NotFoundPaymentRecord);
		}

		let (who, paid_value) = opt_paid_value.unwrap();

		// Calculate the actual cost.
		let mut actual_amount = Self::calculate_fee(agg_count as u32);
		// assert!(actual_amount <= paid_value.amount);
		if actual_amount > paid_value.amount {
			// return Err(Error::<T, I>::CalculateFeeError);
			actual_amount = paid_value.amount
		}

		// unreserve
		let unreserve_value = T::Currency::unreserve(&who, actual_amount);
		if !unreserve_value.is_zero() {
			return Err(Error::<T, I>::UnReserveBalanceError);
		}

		if actual_amount < paid_value.amount {
			<PaymentTrace<T, I>>::insert(
				p_id.clone(),
				who.clone(),
				PaidValue {
					create_bn: paid_value.create_bn,
					paid_era: paid_value.paid_era,
					amount: paid_value.amount.saturating_sub(actual_amount),
					is_income: true,
				},
			);
		}else{
			<PaymentTrace<T, I>>::remove(
				p_id.clone(),
				who.clone(),
			);
		}

		let res = T::Currency::transfer(
			&who,
			&pot_account,
			actual_amount,
			ExistenceRequirement::KeepAlive,
		);

		if !res.is_ok() {
			return Err(Error::<T, I>::TransferBalanceError);
		}

		let ask_era = Self::current_era_num();
		<AskEraPayment<T, I>>::insert(ask_era, (who.clone(), p_id.clone()), paid_value.amount);
		// Emit an event.
		Self::deposit_event(Event::PayForPurchase {
			agg_count: agg_count as u32,
			dest: pot_account,
			fee: actual_amount,
			purchase_id: p_id.clone(),
			unreserve_balance: paid_value.amount,
			who: who.clone(),
		});

		Ok(())
	}
}

impl<T: Config<I>, I: 'static> IForReporter<T, I> for Pallet<T, I> {
	// Note that `bn` is not the current block, but `p_id` corresponds to the submitted block
	fn record_submit_point(
		who: &T::AccountId,
		p_id: &OrderIdEnum,
		_bn: T::BlockNumber,
		ask_point: AskPointNum,
	) -> Result<(), Error<T, I>> {
		// (era, who, (purchase_id, price_count) )
		let ask_era = Self::current_era_num();
		// println!("(who.clone(), p_id.clone()) = {:?}", (who.clone(), p_id.clone()));
		if <AskEraPoint<T, I>>::contains_key(ask_era, (who.clone(), p_id.clone())) {
			return Err(Error::<T, I>::PointRecordIsAlreadyExists);
		}
		<AskEraPoint<T, I>>::insert(ask_era, (who.clone(), p_id.clone()), ask_point);

		//TODO new method should be test
		RewardEra::<T, I>::try_mutate(who.clone(),|reward_era|{
			reward_era.try_push((ask_era, ask_point, p_id.clone())).map_err(|_e|Error::<T, I>::PointRecordIsAlreadyExists)
		})
	}

	fn get_record_point(ask_era: EraIndex, who: &T::AccountId, p_id: &OrderIdEnum) -> Option<AskPointNum> {
		let point = <AskEraPoint<T, I>>::try_get(ask_era, (who.clone(), p_id.clone()));
		if point.is_err() {
			return None;
		}
		return Some(point.unwrap());
	}
}

impl<T: Config<I>, I: 'static> IForBase<T, I> for Pallet<T, I> {

	fn current_era_num() -> EraIndex {
		Self::current_era().unwrap_or(0)
	}

	fn get_earliest_reward_era() -> Option<EraIndex> {
		Self::current_era_num().checked_sub(T::HistoryDepth::get())
	}
}

impl<T: Config<I>, I: 'static> IForReward<T, I> for Pallet<T, I> {
	//
	fn take_reward(ask_era: EraIndex, who: T::AccountId) -> Result<BalanceOf<T, I>, Error<T, I>> {

		let pot_account = Self::account_id();
		if pot_account.is_none() {
			return Err(Error::<T, I>::NoPotAccount);
		}
		let pot_account = pot_account.unwrap();

		//
		if <RewardTrace<T, I>>::contains_key(ask_era.clone(), who.clone()) {
			return Err(Error::<T, I>::RewardHasBeenClaimed);
		}

		// calculate current era number
		// let current_block_number = <frame_system::Pallet<T>>::block_number();
		let current_era = Self::current_era_num();

		// Check unreached.
		if ask_era >= current_era {
			return Err(Error::<T, I>::RewardSlotNotExpired);
		}

		// Check expired.
		if let Some(earliest_era) = Self::get_earliest_reward_era() {
			// println!(" ask_era {:?} < earliest_era {:?}", ask_era, earliest_era);
			if ask_era < earliest_era {
				return Err(Error::<T, I>::RewardEraHasExpired);
			}
		}

		// get his point.
		let mut reward_point: AskPointNum = 0;
		let _reward_list: Vec<(T::AccountId, OrderIdEnum)> = <AskEraPoint<T, I>>::iter_prefix(ask_era)
			.map(|((acc, p_id), point)| {
				if &acc == &who {
					reward_point += point;
				}
				(who.clone(), p_id.clone())
			})
			.collect();

		if 0 == reward_point {
			return Err(Error::<T, I>::NoRewardPoints);
		}

		//convert point to balance.
		// get era income
		let era_income: BalanceOf<T, I> = Self::get_era_income(ask_era);
		let era_point = Self::get_era_point(ask_era);
		let single_reward_balance = era_income / era_point.into();

		// get reward of `who`
		let reward_balance = single_reward_balance.saturating_mul(reward_point.into());

		let res = T::Currency::transfer(
			&pot_account,
			&who,
			reward_balance,
			ExistenceRequirement::KeepAlive,
		);
		if res.is_err() {
			return Err(Error::<T, I>::TransferBalanceError);
		}

		let current_block_number = <frame_system::Pallet<T>>::block_number();
		<RewardTrace<T, I>>::insert(ask_era.clone(), who.clone(), (current_block_number, reward_balance));

		RewardEra::<T, I>::mutate(who.clone(),|reward_era_vec|{
			reward_era_vec.retain(|(era_num, _, _)| {
				if era_num == &ask_era {
					return false;
				}
				true
			});
		});

		Ok(reward_balance)
	}

	fn get_era_income(ask_era: EraIndex) -> BalanceOf<T, I> {
		// <BalanceOf<T, I>>::from(1000u32)
		let mut result = <BalanceOf<T, I>>::from(0u32);
		<AskEraPayment<T, I>>::iter_prefix_values(ask_era)
			.into_iter()
			.any(|x| {
				result += x;
				false
			});
		result
	}

	//
	fn get_era_point(ask_era: EraIndex) -> AskPointNum {
		<AskEraPoint<T, I>>::iter_prefix_values(ask_era).into_iter().sum()
	}

	//
	fn check_and_slash_expired_rewards(check_era: EraIndex) -> Option<BalanceOf<T, I>> {
		let pot_account = Self::account_id();
		if pot_account.is_none() {
			return None;
		}
		let pot_account = pot_account.unwrap();

		// get checkout era fund
		let era_income: BalanceOf<T, I> = Self::get_era_income(check_era);

		// get sum of paid reward
		let mut paid_reward = <BalanceOf<T, I>>::from(0u32);
		<RewardTrace<T, I>>::iter_prefix_values(check_era).any(|(_bn, amount)| {
			paid_reward += amount;
			false
		});

		let diff: BalanceOf<T, I> = era_income.saturating_sub(paid_reward);
		if diff == 0u32.into() {
			return None;
		}

		let (negative_imbalance, _remaining_balance) = T::Currency::slash(&pot_account, diff);
		T::OnSlash::on_unbalanced(negative_imbalance);

		let _res = <RewardTrace<T, I>>::clear_prefix(check_era, u32::MAX, None);
		// <AskEraPoint<T, I>>::remove_prefix(check_era, None);
		// RewardEra
		<AskEraPoint<T, I>>::iter_prefix(check_era).any(|((acc, _), _)| {
			let mut reward_era = <RewardEra<T, I>>::get(acc.clone());
			reward_era.retain(|(del_era, _, _)| {
				if &check_era == del_era {
					return false;
				}
				true
			});
			<RewardEra<T, I>>::insert(acc.clone(), reward_era);
			false
		});
		let _res = <AskEraPoint<T, I>>::clear_prefix(check_era, u32::MAX, None);
		let _res = <AskEraPayment<T, I>>::clear_prefix(check_era, u32::MAX, None);

		let current_block_number = <frame_system::Pallet<T>>::block_number();
		Self::deposit_event(Event::PurchaseRewardSlashedAfterExpiration {
			created_at: current_block_number,
			era: check_era,
			slash: diff,
		});

		Some(diff)
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {

	pub fn account_id() -> Option<T::AccountId> {
		// T::PalletId::get().try_into_account()
		Some(T::PalletId::get().into_account_truncating())
	}
	fn pot() -> Option<(T::AccountId, BalanceOf<T, I>)> {
		let account_id = Self::account_id();
		if account_id.is_none() {
			return None
		}
		let account_id = account_id.unwrap();
		let balance = T::Currency::free_balance(&account_id).saturating_sub(T::Currency::minimum_balance());
		Some((account_id, balance))
	}
	/// Clear all era information for given era.
	pub(crate) fn clear_era_information(era_index: EraIndex) {
		ErasStartSessionIndex::<T, I>::remove(era_index);
		Self::check_and_slash_expired_rewards(era_index);
	}

	fn check_start_new_era(session_index: SessionIndex) ->bool {
		if let Some(current_era) = Self::current_era() {
			// Initial era has been set.
			let current_era_start_session_index = Self::eras_start_session_index(current_era)
				.unwrap_or_else(|| {
					frame_support::print("Error: oracle-finance start_session_index must be set for current_era");
					0
				});
			let era_length =
				session_index.checked_sub(current_era_start_session_index).unwrap_or(0);
			if era_length >= T::AskPerEra::get() {
				return true;
			}
		}
		return false;
	}

	fn new_session(session_index: SessionIndex, is_genesis: bool) -> Option<Vec<T::ValidatorId>> {
		if let Some(_current_era) = Self::current_era() {
			if Self::check_start_new_era(session_index) {
				let new_planned_era = CurrentEra::<T, I>::mutate(|s| {
					*s = Some(s.map(|s| s + 1).unwrap_or(0));
					s.unwrap()
				});
				ErasStartSessionIndex::<T, I>::insert(&new_planned_era, &session_index);
				// Clean old era information.
				if let Some(old_era) = new_planned_era.checked_sub(T::HistoryDepth::get() + 1) {
					Self::clear_era_information(old_era);
				}
			}
		}else{
			CurrentEra::<T, I>::put(0);
			ErasStartSessionIndex::<T, I>::insert(0, session_index);
		}

		if is_genesis {
			return T::SessionManager::new_session_genesis(session_index);
		}
		T::SessionManager::new_session(session_index)
	}
	fn start_session(session_index: SessionIndex) {
		T::SessionManager::start_session(session_index)
	}
	fn end_session(session_index: SessionIndex) {
		if Self::check_start_new_era(session_index + 2) {
			Self::deposit_event(Event::EndOfAskEra {
				era: Self::current_era_num(),
				era_income: Self::get_era_income(Self::current_era_num()),
				era_points: Self::get_era_point(Self::current_era_num()),
				session_index
			});
		}
		T::SessionManager::end_session(session_index)
	}
}

impl<T: Config<I>, I: 'static> pallet_session::SessionManager<T::ValidatorId> for Pallet<T, I> {
	fn new_session(new_index: SessionIndex) -> Option<Vec<T::ValidatorId>> {
		log::info!("🚅 planning new oracle-finance session {}", new_index);
		Self::new_session(new_index, false)
	}
	fn new_session_genesis(new_index: SessionIndex) -> Option<Vec<T::ValidatorId>> {
		log::info!("🚅 planning new oracle-finance session {} at genesis", new_index);
		Self::new_session(new_index, true)
	}
	fn start_session(start_index: SessionIndex) {
		log::info!("🚅 starting oracle-finance session {}", start_index);
		Self::start_session(start_index)
	}
	fn end_session(end_index: SessionIndex) {
		log::info!("🚅 ending oracle-finance session {}", end_index);
		Self::end_session(end_index)
	}
}
