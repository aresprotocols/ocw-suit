#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::traits::{Currency, Get, ReservableCurrency, ExistenceRequirement, OnUnbalanced};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;
use crate::traits::*;
use crate::types::*;
use sp_runtime::traits::{Saturating, Zero};
use frame_support::sp_runtime::traits::{AccountIdConversion, Clear};
use frame_support::sp_std::convert::TryInto;
use sp_std::vec::Vec;

#[cfg(feature = "std")]
use frame_support::traits::GenesisBuild;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod traits;
pub mod types;


#[frame_support::pallet]
pub mod pallet {
	use frame_support::traits::{Currency, ReservableCurrency, OnUnbalanced};
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;
	use crate::types::*;
	use frame_support::sp_runtime::traits::Zero;
	use crate::traits::{IForReward, IForBase};

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

		#[pallet::constant]
		type AskPeriod: Get<Self::BlockNumber>;

		#[pallet::constant]
		type RewardPeriodCycle: Get<AskPeriodNum>;

		#[pallet::constant]
		type RewardSlot: Get<AskPeriodNum>;

		type OnSlash: OnUnbalanced<NegativeImbalanceOf<Self>>;

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


	#[pallet::storage]
	#[pallet::getter(fn reward_trace)]
	pub type RewardTrace<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AskPeriodNum, //,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId,
		(<T as frame_system::Config>::BlockNumber, BalanceOf<T>),
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub _pt: PhantomData<T>,
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
		PurchaseRewardTaken(T::AccountId, BalanceOf<T>),
		// OcwFinanceDepositCreating(BalanceOf<T>),
		PurchaseRewardSlashedAfterExpiration(BalanceOf<T>),
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
		//
		RewardSlotNotExpired,
		//
		RewardPeriodHasExpired,
		//
		NoRewardPoints,
		//
		RewardHasBeenClaimed,
		//
		UnReserveBalanceError,
		//
		TransferBalanceError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let current_block_number = n;
			if T::BlockNumber::zero() == current_block_number % T::AskPeriod::get() {
				Self::check_and_slash_expired_rewards(Self::make_period_num(current_block_number));
			}
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(10000)]
		pub fn take_purchase_reward(origin: OriginFor<T>, ask_period: AskPeriodNum) -> DispatchResult {

			let who = ensure_signed(origin)?;

			let take_balance: BalanceOf<T> = Self::take_reward(ask_period, who.clone(), )?;

			// Emit an event.
			Self::deposit_event(Event::PurchaseRewardTaken(who, take_balance));
			// Return a successful DispatchResultWithPostInfo
			Ok(())
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

	fn reserve_for_ask_quantity(who: <T as frame_system::Config>::AccountId, p_id: PurchaseId, price_count: u32) -> OcwPaymentResult<T> {
		let reserve_balance = Self::calculate_fee_of_ask_quantity(price_count);
		// T::Currency::reserve(&account_id, reserve_balance.clone());
		// let res = T::Currency::transfer(
		// 	&who,
		// 	&Self::account_id(),
		// 	reserve_balance,
		// 	ExistenceRequirement::KeepAlive,
		// );

		let res = T::Currency::reserve(&who, reserve_balance);

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
		// <AskPeriodPayment<T>>::insert(ask_period, (who.clone(), p_id.clone()), reserve_balance);

		OcwPaymentResult::Success(p_id, reserve_balance)
	}

	fn unreserve_ask(p_id: PurchaseId) -> Result<(), Error<T>> {

		let payment_list = <PaymentTrace<T>>::iter_prefix(p_id.clone());
		// if payment_list.into_iter()..count() == 0 as usize {
		// 	return Err(Error::NotFoundPaymentRecord);
		// }
		let mut find_pid = false;
		let is_success = payment_list.into_iter().into_iter().any(|(who, paid_value)| {
			if paid_value.is_income {
				find_pid = true;

				let res = T::Currency::unreserve(&who, paid_value.amount);

				if res.is_zero() {
					<PaymentTrace<T>>::remove(p_id.clone(), who.clone());
					// let ask_period = Self::make_period_num(paid_value.create_bn);
					// <AskPeriodPayment<T>>::remove(ask_period, (who.clone(), p_id.clone()));
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

	fn pay_to_ask(p_id: PurchaseId) -> Result<(), Error<T>> {

		let payment_list = <PaymentTrace<T>>::iter_prefix(p_id.clone());

		let mut opt_paid_value: Option<(T::AccountId, PaidValue<T>)> = None;
		payment_list.into_iter().into_iter().any(|(who, paid_value)| {
			if paid_value.is_income {
				opt_paid_value = Some((who,paid_value));
			}
			false
		});

		if opt_paid_value.is_none() {
			return Err(Error::NotFoundPaymentRecord);
		}

		let (who, paid_value ) = opt_paid_value.unwrap();
		// unreserve
		let unreserve_value = T::Currency::unreserve(&who, paid_value.amount);
		if !unreserve_value.is_zero() {
			return Err(Error::<T>::UnReserveBalanceError);
		}

		let res = T::Currency::transfer(
			&who,
			&Self::account_id(),
			paid_value.amount,
			ExistenceRequirement::KeepAlive,
		);

		if !res.is_ok() {
			return Err(Error::<T>::TransferBalanceError);
		}

		let ask_period = Self::make_period_num(paid_value.create_bn);
		<AskPeriodPayment<T>>::insert(ask_period, (who.clone(), p_id.clone()), paid_value.amount);

		// Self::deposit_event(Event::OcwFinanceDepositCreating());

		Ok(())
	}
}

impl <T: Config> IForReporter<T> for Pallet<T> {

	// Note that `bn` is not the current block, but `p_id` corresponds to the submitted block
	fn record_submit_point(who: T::AccountId, p_id: PurchaseId, bn: T::BlockNumber, ask_point: AskPointNum) -> Result<(), Error<T>> {
		// (period, who, (purchase_id, price_count) )
		let ask_period = Self::make_period_num(bn);
		// println!("(who.clone(), p_id.clone()) = {:?}", (who.clone(), p_id.clone()));
		if <AskPeriodPoint<T>>::contains_key(ask_period, (who.clone(), p_id.clone())) {
			return Err(Error::<T>::PointRecordIsAlreadyExists);
		}
		<AskPeriodPoint<T>>::insert(ask_period, (who.clone(), p_id), ask_point);
		Ok(())
	}

	fn get_record_point(ask_period: u64, who: T::AccountId, p_id: PurchaseId) -> Option<AskPointNum> {
		let point = <AskPeriodPoint<T>>::try_get(ask_period, (who.clone(), p_id.clone()));
		if point.is_err() {
			return None;
		}
		return Some(point.unwrap());
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

// impl <BlockNumber: > IForBase<BlockNumber> for Pallet<BlockNumber> {
// 	//
// 	fn make_period_num(bn: BlockNumber) -> AskPeriodNum {
// 		let param_bn: u64 = bn.try_into().unwrap_or(0);
// 		let ask_perio: u64 = T::AskPeriod::get().try_into().unwrap_or(0);
// 		if 0 == param_bn || 0 == ask_perio {
// 			return 0;
// 		}
// 		param_bn / ask_perio
// 	}
// }

impl <T: Config> IForReward<T> for Pallet<T> {
	//
	fn take_reward(ask_period: AskPeriodNum, who: T::AccountId) -> Result<BalanceOf<T>, Error<T>> {

		//
		if <RewardTrace<T>>::contains_key(ask_period.clone(), who.clone()) {
			return Err(Error::<T>::RewardHasBeenClaimed);
		}

		// calculate current period number
		let current_block_number = <frame_system::Pallet<T>>::block_number();
		let current_period = Self::make_period_num(current_block_number);

		// Check unreached.
		if ask_period > (current_period.saturating_sub(T::RewardSlot::get())) {
			return Err(Error::<T>::RewardSlotNotExpired);
		}

		// Check expired.
		if ask_period < Self::get_earliest_reward_period(current_block_number) {
			return Err(Error::<T>::RewardPeriodHasExpired);
		}

		// get his point.
		let mut reward_point: AskPointNum = 0;
		let reward_list: Vec<(T::AccountId, PurchaseId)> = <AskPeriodPoint<T>>::iter_prefix(ask_period).map(|((acc, p_id), point)| {
			if &acc == &who {
				reward_point += point;
			}
			(who.clone(), p_id.clone())
		}).collect();

		if 0 == reward_point {
			return Err(Error::<T>::NoRewardPoints);
		}

		//convert point to balance.
		// get period income
		let period_income: BalanceOf<T> = Self::get_period_income(ask_period);
		let period_point = Self::get_period_point(ask_period);
		let single_reward_balance = period_income / period_point.into();

		// get reward of `who`
		let reward_balance = single_reward_balance.saturating_mul(reward_point.into());

		let res = T::Currency::transfer(
			&Self::account_id(),
			&who,
			reward_balance,
			ExistenceRequirement::KeepAlive,
		);

		<RewardTrace<T>>::insert(ask_period.clone(), who.clone(), (current_block_number, reward_balance));

		Ok(reward_balance)
	}

	fn get_period_income(ask_period: AskPeriodNum) -> BalanceOf<T> {
		// <BalanceOf<T>>::from(1000u32)
		let mut result = <BalanceOf<T>>::from(0u32);
		<AskPeriodPayment<T>>::iter_prefix_values(ask_period).into_iter().any(|x| {
			result += x;
			false
		});
		result
	}


	fn get_earliest_reward_period(bn: T::BlockNumber) -> AskPeriodNum {
		// calculate current period number
		let param_period = Self::make_period_num(bn);
		param_period.saturating_sub(T::RewardPeriodCycle::get()).saturating_sub(T::RewardSlot::get())
	}

	//
	fn get_period_point(ask_period: AskPeriodNum) -> AskPointNum {
		<AskPeriodPoint<T>>::iter_prefix_values(ask_period).into_iter().sum()
	}

	//
	fn check_and_slash_expired_rewards(ask_period: AskPeriodNum) -> Option<BalanceOf<T>> {
		let check_period = ask_period
			.saturating_sub(1)
			.saturating_sub(T::RewardPeriodCycle::get())
			.saturating_sub(T::RewardSlot::get());

		if 0 == check_period {
			return None;
		}

		// get checkout period fund
		let period_income: BalanceOf<T> = Self::get_period_income(check_period);

		// get sum of paid reward
		let mut paid_reward = <BalanceOf<T>>::from(0u32);
		<RewardTrace<T>>::iter_prefix_values(check_period).any(|(bn, amount)| {
			paid_reward += amount;
			false
		});

		let diff: BalanceOf<T> = period_income.saturating_sub(paid_reward);
		if diff == 0u32.into() {
			return None;
		}

		// println!("check_period = {:?}, paid_reward = {:?}, diff = {:?}  ", check_period, paid_reward, diff);

		let (negative_imbalance, _remaining_balance) = T::Currency::slash(&Self::account_id(), diff);
		T::OnSlash::on_unbalanced(negative_imbalance);

		<RewardTrace<T>>::remove_prefix(check_period, None);
		<AskPeriodPoint<T>>::remove_prefix(check_period, None);
		<AskPeriodPayment<T>>::remove_prefix(check_period, None);

		Self::deposit_event(Event::PurchaseRewardSlashedAfterExpiration(diff));

		Some(diff)
	}
}
