#![cfg_attr(not(feature = "std"), no_std)]

use crate::traits::*;
use crate::types::*;
use frame_support::sp_runtime::traits::AccountIdConversion;
use frame_support::sp_std::convert::TryInto;
use frame_support::traits::{Currency, ExistenceRequirement, Get, OnUnbalanced, ReservableCurrency};
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;
use sp_runtime::traits::{Saturating, Zero};
use sp_std::vec::Vec;
use pallet_session::SessionManager;

// #[cfg(feature = "std")]
// use frame_support::traits::GenesisBuild;

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
	use crate::traits::{IForBase, IForReward};
	use crate::types::*;
	use frame_support::sp_runtime::traits::Zero;
	use frame_support::traits::{Currency, OnUnbalanced, ReservableCurrency};
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, PalletId};
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use pallet_session::SessionManager;
	use sp_runtime::traits::Convert;
	use sp_std::convert::TryFrom;

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

		// #[pallet::constant]
		// type AskPeriod: Get<Self::BlockNumber>;

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

		// #[pallet::constant]
		// type RewardEraCycle: Get<EraIndex>;
		//
		// #[pallet::constant]
		// type RewardSlot: Get<EraIndex>;

		type OnSlash: OnUnbalanced<NegativeImbalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	// #[pallet::generate_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn payment_trace)]
	pub type PaymentTrace<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PurchaseId, // purchased_id,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId, // pay or pay to account. ,,
		PaidValue<<T as frame_system::Config>::BlockNumber, BalanceOf<T>, EraIndex>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ask_era_payment)]
	pub type AskEraPayment<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		EraIndex, // ,
		Blake2_128Concat,
		(<T as frame_system::Config>::AccountId, PurchaseId), // pay or pay to account. ,,
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ask_era_point)]
	pub type AskEraPoint<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		EraIndex, //,
		Blake2_128Concat,
		(<T as frame_system::Config>::AccountId, PurchaseId), // pay or pay to account. ,,
		AskPointNum,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn current_era)]
	pub type CurrentEra<T> = StorageValue<_, EraIndex>;

	#[pallet::storage]
	#[pallet::getter(fn eras_start_session_index)]
	pub type ErasStartSessionIndex<T> = StorageMap<_, Twox64Concat, EraIndex, SessionIndex>;

	#[pallet::storage]
	#[pallet::getter(fn reward_era)]
	pub type RewardEra<T> = StorageMap<
		_,
		Blake2_128Concat,
		<T as frame_system::Config>::AccountId,                                    //,
		BoundedVec<(EraIndex, AskPointNum, PurchaseId), MaximumRewardEras>, // pay or pay to account. ,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn reward_trace)]
	pub type RewardTrace<T> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		EraIndex, //,
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
				log::info!("oracle-finance deposit creating, {:?}", T::Currency::minimum_balance());
				T::Currency::deposit_creating(&finance_account, T::Currency::minimum_balance());
				// Self::deposit_event(Event::<T>::OracleFinanceDepositCreating(T::Currency::
				// minimum_balance()));
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PayForPurchase {
			agg_count: u32,
			dest: T::AccountId,
			fee: BalanceOf<T>,
			purchase_id: PurchaseId,
			unreserve_balance: BalanceOf<T>,
			who: T::AccountId,
		},
		PurchaseRewardToken {
			created_at: T::BlockNumber,
			era: EraIndex,
			who: T::AccountId,
			reward: BalanceOf<T>,
		},
		PurchaseRewardSlashedAfterExpiration {
			created_at: T::BlockNumber,
			era: EraIndex,
			slash: BalanceOf<T>,
		},
		// CreatePurchaseOrder {
		// 	era: EraIndex,
		// 	pid: PurchaseId,
		// 	reserve_balance: BalanceOf<T>,
		// 	who: T::AccountId,
		// },
		EndOfAskEra {
			era: EraIndex,
			era_income: BalanceOf<T>,
			era_points: AskPointNum,
			session_index: SessionIndex,
		},

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
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: T::BlockNumber) -> Weight {
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000)]
		pub fn take_purchase_reward(origin: OriginFor<T>, ask_era: EraIndex) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let who = T::ValidatorIdOf::convert(controller.clone());
			log::debug!(
				"Call take_purchase_reward, Controller = {:?}, Stash = {:?}",
				&controller.clone(), &who.clone(),
			);
			ensure!(who.is_some(), Error::<T>::NoStashAccount);
			let who: T::AccountId = who.unwrap();

			let take_balance: BalanceOf<T> = Self::take_reward(ask_era, who.clone())?;
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

		#[pallet::weight(1000)]
		pub fn take_all_purchase_reward(origin: OriginFor<T>) -> DispatchResult {
			let controller = ensure_signed(origin)?;
			let who = T::ValidatorIdOf::convert(controller.clone());
			log::debug!(
				"Call take_all_purchase_reward, Controller = {:?}, Stash = {:?}",
				&controller.clone(), &who.clone(),
			);
			ensure!(who.is_some(), Error::<T>::NoStashAccount);
			let who: T::AccountId = who.unwrap();

			let reward_era_list:BoundedVec<(EraIndex, AskPointNum, PurchaseId), MaximumRewardEras>
				= RewardEra::<T>::get(who.clone());

			ensure!(!reward_era_list.is_empty(), Error::<T>::NoRewardPoints);


			let current_era_opt = CurrentEra::<T>::get();
			let mut era_list = Vec::new();
			for (ask_era, _ask_point_num, pid) in reward_era_list {

				if !era_list.iter().any(|exists_era|{
					&ask_era == exists_era
				}) {
					if let (Some(current_era)) = current_era_opt {
						if ask_era <  current_era {
							era_list.push(ask_era);
						}
					}
				}
			}

			let current_block_number = <frame_system::Pallet<T>>::block_number();
			for ask_era in era_list {
				let take_balance: BalanceOf<T> = Self::take_reward(ask_era, who.clone())?;
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

impl<T: Config> IForPrice<T> for Pallet<T> {
	fn calculate_fee_of_ask_quantity(price_count: u32) -> BalanceOf<T> {
		T::BasicDollars::get().saturating_mul(price_count.into())
	}

	fn reserve_for_ask_quantity(
		who: <T as frame_system::Config>::AccountId,
		p_id: PurchaseId,
		price_count: u32,
	) -> OcwPaymentResult<BalanceOf<T>, PurchaseId> {
		let reserve_balance = Self::calculate_fee_of_ask_quantity(price_count);
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
				paid_era: Self::current_era_num(),
				amount: reserve_balance,
				is_income: true,
			},
		);
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
					// let ask_era = Self::current_era_num(paid_value.create_bn);
					// <AskEraPayment<T>>::remove(ask_era, (who.clone(), p_id.clone()));
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

	fn pay_to_ask(p_id: PurchaseId, agg_count: usize) -> Result<(), Error<T>> {
		let payment_list = <PaymentTrace<T>>::iter_prefix(p_id.clone());

		let mut opt_paid_value: Option<(T::AccountId, PaidValue<T::BlockNumber, BalanceOf<T>, EraIndex>)> = None;
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
		// unreserve
		let unreserve_value = T::Currency::unreserve(&who, paid_value.amount);
		if !unreserve_value.is_zero() {
			return Err(Error::<T>::UnReserveBalanceError);
		}

		// Calculate the actual cost.
		let actual_amount = Self::calculate_fee_of_ask_quantity(agg_count as u32);
		assert!(actual_amount <= paid_value.amount);

		let res = T::Currency::transfer(
			&who,
			&Self::account_id(),
			actual_amount,
			ExistenceRequirement::KeepAlive,
		);

		if !res.is_ok() {
			return Err(Error::<T>::TransferBalanceError);
		}

		let ask_era = Self::current_era_num();
		<AskEraPayment<T>>::insert(ask_era, (who.clone(), p_id.clone()), paid_value.amount);
		// Emit an event.
		Self::deposit_event(Event::PayForPurchase {
			agg_count: agg_count as u32,
			dest: Self::account_id(),
			fee: actual_amount,
			purchase_id: p_id,
			unreserve_balance: paid_value.amount,
			who: who.clone(),
		});

		Ok(())
	}
}

impl<T: Config> IForReporter<T> for Pallet<T> {
	// Note that `bn` is not the current block, but `p_id` corresponds to the submitted block
	fn record_submit_point(
		who: T::AccountId,
		p_id: PurchaseId,
		_bn: T::BlockNumber,
		ask_point: AskPointNum,
	) -> Result<(), Error<T>> {
		// (era, who, (purchase_id, price_count) )
		let ask_era = Self::current_era_num();
		// println!("(who.clone(), p_id.clone()) = {:?}", (who.clone(), p_id.clone()));
		if <AskEraPoint<T>>::contains_key(ask_era, (who.clone(), p_id.clone())) {
			return Err(Error::<T>::PointRecordIsAlreadyExists);
		}
		<AskEraPoint<T>>::insert(ask_era, (who.clone(), p_id.clone()), ask_point);
		// Get reward era vec.
		// let mut reward_era = <RewardEra<T>>::get(who.clone());
		// reward_era.push((ask_era, ask_point, p_id));
		// <RewardEra<T>>::insert(who.clone(), reward_era);

		//TODO new method should be test
		RewardEra::<T>::try_mutate(who.clone(),|reward_era|{
			reward_era.try_push((ask_era, ask_point, p_id)).map_err(|e|Error::<T>::PointRecordIsAlreadyExists)
		})
		// Ok(())
	}

	fn get_record_point(ask_era: EraIndex, who: T::AccountId, p_id: PurchaseId) -> Option<AskPointNum> {
		let point = <AskEraPoint<T>>::try_get(ask_era, (who.clone(), p_id.clone()));
		if point.is_err() {
			return None;
		}
		return Some(point.unwrap());
	}
}

impl<T: Config> IForBase<T> for Pallet<T> {
	// TODO::
	fn current_era_num() -> EraIndex {
		// let param_bn: u64 = bn.try_into().unwrap_or(0);
		// let ask_perio: u64 = T::AskPeriod::get().try_into().unwrap_or(0);
		// if 0 == param_bn || 0 == ask_perio {
		// 	return 0;
		// }
		// param_bn / ask_perio
		Self::current_era().unwrap_or(0)
	}

	//
	fn get_earliest_reward_era() -> Option<EraIndex> {
		Self::current_era_num().checked_sub(T::HistoryDepth::get())
	}
}

impl<T: Config> IForReward<T> for Pallet<T> {
	//
	fn take_reward(ask_era: EraIndex, who: T::AccountId) -> Result<BalanceOf<T>, Error<T>> {
		//
		if <RewardTrace<T>>::contains_key(ask_era.clone(), who.clone()) {
			return Err(Error::<T>::RewardHasBeenClaimed);
		}

		// calculate current era number
		// let current_block_number = <frame_system::Pallet<T>>::block_number();
		let current_era = Self::current_era_num();

		// Check unreached.
		if ask_era >= current_era {
			return Err(Error::<T>::RewardSlotNotExpired);
		}

		// Check expired.
		if let Some(earliest_era) = Self::get_earliest_reward_era() {
			// println!(" ask_era {:?} < earliest_era {:?}", ask_era, earliest_era);
			if ask_era < earliest_era {
				return Err(Error::<T>::RewardEraHasExpired);
			}
		}

		// get his point.
		let mut reward_point: AskPointNum = 0;
		let _reward_list: Vec<(T::AccountId, PurchaseId)> = <AskEraPoint<T>>::iter_prefix(ask_era)
			.map(|((acc, p_id), point)| {
				if &acc == &who {
					reward_point += point;
				}
				(who.clone(), p_id.clone())
			})
			.collect();

		if 0 == reward_point {
			return Err(Error::<T>::NoRewardPoints);
		}

		//convert point to balance.
		// get era income
		let era_income: BalanceOf<T> = Self::get_era_income(ask_era);
		let era_point = Self::get_era_point(ask_era);
		let single_reward_balance = era_income / era_point.into();

		// get reward of `who`
		let reward_balance = single_reward_balance.saturating_mul(reward_point.into());

		let res = T::Currency::transfer(
			&Self::account_id(),
			&who,
			reward_balance,
			ExistenceRequirement::KeepAlive,
		);
		if res.is_err() {
			return Err(Error::<T>::TransferBalanceError);
		}

		let current_block_number = <frame_system::Pallet<T>>::block_number();
		<RewardTrace<T>>::insert(ask_era.clone(), who.clone(), (current_block_number, reward_balance));

		RewardEra::<T>::mutate(who.clone(),|reward_era_vec|{
			reward_era_vec.retain(|(era_num, _, _)| {
				if era_num == &ask_era {
					return false;
				}
				true
			});
		});

		Ok(reward_balance)
	}

	fn get_era_income(ask_era: EraIndex) -> BalanceOf<T> {
		// <BalanceOf<T>>::from(1000u32)
		let mut result = <BalanceOf<T>>::from(0u32);
		<AskEraPayment<T>>::iter_prefix_values(ask_era)
			.into_iter()
			.any(|x| {
				result += x;
				false
			});
		result
	}

	//
	fn get_era_point(ask_era: EraIndex) -> AskPointNum {
		<AskEraPoint<T>>::iter_prefix_values(ask_era).into_iter().sum()
	}

	//
	fn check_and_slash_expired_rewards(check_era: EraIndex) -> Option<BalanceOf<T>> {
		// println!(" RUN check_and_slash_expired_rewards  ");
		// let check_era = ask_era.checked_sub(T::HistoryDepth::get() + 1);
		// if check_era.is_none() {
		// 	return None;
		// }
		// let check_era = check_era.unwrap();
		// println!("check_era={:?}", check_era);

		// get checkout era fund
		let era_income: BalanceOf<T> = Self::get_era_income(check_era);

		// get sum of paid reward
		let mut paid_reward = <BalanceOf<T>>::from(0u32);
		<RewardTrace<T>>::iter_prefix_values(check_era).any(|(_bn, amount)| {
			paid_reward += amount;
			false
		});

		let diff: BalanceOf<T> = era_income.saturating_sub(paid_reward);
		if diff == 0u32.into() {
			return None;
		}

		let (negative_imbalance, _remaining_balance) = T::Currency::slash(&Self::account_id(), diff);
		T::OnSlash::on_unbalanced(negative_imbalance);

		<RewardTrace<T>>::remove_prefix(check_era, None);
		// <AskEraPoint<T>>::remove_prefix(check_era, None);
		// RewardEra
		<AskEraPoint<T>>::iter_prefix(check_era).any(|((acc, _), _)| {
			let mut reward_era = <RewardEra<T>>::get(acc.clone());
			reward_era.retain(|(del_era, _, _)| {
				if &check_era == del_era {
					return false;
				}
				true
			});
			<RewardEra<T>>::insert(acc.clone(), reward_era);
			false
		});
		<AskEraPoint<T>>::remove_prefix(check_era, None);
		<AskEraPayment<T>>::remove_prefix(check_era, None);

		let current_block_number = <frame_system::Pallet<T>>::block_number();
		Self::deposit_event(Event::PurchaseRewardSlashedAfterExpiration {
			created_at: current_block_number,
			era: check_era,
			slash: diff,
		});

		Some(diff)
	}
}

impl<T: Config> Pallet<T> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}
	fn pot() -> (T::AccountId, BalanceOf<T>) {
		let account_id = Self::account_id();
		let balance = T::Currency::free_balance(&account_id).saturating_sub(T::Currency::minimum_balance());
		(account_id, balance)
	}
	/// Clear all era information for given era.
	pub(crate) fn clear_era_information(era_index: EraIndex) {
		ErasStartSessionIndex::<T>::remove(era_index);
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
			// println!("current_era_start_session_index = {:?} era_length {:?} >= T::AskPerEra::get() {:?}",
			// 	current_era_start_session_index,
			// 		 era_length,
			// 		 T::AskPerEra::get()
			// );
			if era_length >= T::AskPerEra::get() {
				return true;
			}
		}
		return false;
	}

	fn new_session(session_index: SessionIndex, is_genesis: bool) -> Option<Vec<T::ValidatorId>> {
		if let Some(_current_era) = Self::current_era() {
			if Self::check_start_new_era(session_index) {
				let new_planned_era = CurrentEra::<T>::mutate(|s| {
					*s = Some(s.map(|s| s + 1).unwrap_or(0));
					s.unwrap()
				});
				ErasStartSessionIndex::<T>::insert(&new_planned_era, &session_index);
				// Clean old era information.
				if let Some(old_era) = new_planned_era.checked_sub(T::HistoryDepth::get() + 1) {
					Self::clear_era_information(old_era);
				}
			}
		}else{
			CurrentEra::<T>::put(0);
			ErasStartSessionIndex::<T>::insert(0, session_index);
		}

		if is_genesis {
			return T::SessionManager::new_session_genesis(session_index);
		}
		// println!("new_session =  {:?} current_index = {:?}", session_index, Self::current_era());
		T::SessionManager::new_session(session_index)
	}
	fn start_session(session_index: SessionIndex) {
		// println!("start_session =  {:?}", session_index);
		T::SessionManager::start_session(session_index)
	}
	fn end_session(session_index: SessionIndex) {
		// println!("end_session =  {:?} # current_era = {:?}  # new_session_index = {:?}",
		// 		 session_index,
		// 		 Self::current_era_num(),
		// 		 Self::eras_start_session_index(Self::current_era_num()),
		// );
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

impl<T: Config> pallet_session::SessionManager<T::ValidatorId> for Pallet<T> {
	fn new_session(new_index: SessionIndex) -> Option<Vec<T::ValidatorId>> {
		log::info!("🚅 planning new oracle-finance session {}", new_index);
		// CurrentPlannedSession::<T>::put(new_index);
		Self::new_session(new_index, false)
	}
	fn new_session_genesis(new_index: SessionIndex) -> Option<Vec<T::ValidatorId>> {
		log::info!("🚅 planning new oracle-finance session {} at genesis", new_index);
		// CurrentPlannedSession::<T>::put(new_index);
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
