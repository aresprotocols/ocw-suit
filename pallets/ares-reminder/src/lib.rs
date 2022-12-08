#![cfg_attr(not(feature = "std"), no_std)]


use ares_oracle_provider_support::{ChainPrice, PriceKey};
pub use pallet::*;
use sp_std::vec::Vec;
use crate::types::{PriceTrigger, ReminderCondition, ReminderIden, ReminderIdenList, ReminderReceiver, RepeatCount};

#[cfg(all(feature = "std", test))]
mod mock;

#[cfg(all(feature = "std", test))]
mod tests;

// #[cfg(any(feature = "runtime-benchmarks", test))]
// mod benchmarking;

pub mod types;

pub mod weights;
mod offchain_payload_helper;

#[frame_support::pallet]
pub mod pallet {
	use core::num::FpCategory::Zero;
	use frame_support::{log, transactional};
	use bound_vec_helper::BoundVecHelper;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement};
	use frame_system::offchain::{AppCrypto, CreateSignedTransaction, SignedPayload, SigningTypes};
	use frame_system::pallet_prelude::*;
	use sp_runtime::app_crypto::UncheckedFrom;
	use sp_runtime::RuntimeAppPublic;
	use sp_runtime::traits::Saturating;
	use ares_oracle_provider_support::{ChainPrice, ConvertChainPrice, FractionLength, IOracleAvgPriceEvents, IStashAndAuthority, OrderIdEnum, PriceKey, SymbolInfo};
	use ares_oracle_provider_support::crypto::sr25519::AuthorityId;
	use ares_oracle_provider_support::IAresOraclePreCheck;
	use oracle_finance::traits::{IForPrice, IForReporter, IForReward};
	use oracle_finance::types::{BalanceOf, OcwPaymentResult};
	use crate::types::{CompletePayload, DispatchPayload, OffchainSignature, PriceTrigger, ReminderCondition, ReminderIden, ReminderIdenList, ReminderReceiver, ReminderSendList, ReminderTriggerTip, RepeatCount};
	use crate::weights::WeightInfo;
	use sp_std::vec::Vec;

	pub const DEBUG_TARGET: &str = "ares-reminder";

	impl<T: SigningTypes + Config> SignedPayload<T>
	for CompletePayload<ReminderIden, Vec<u8>, u32, T::AuthorityAres, T::Public, T::BlockNumber>
	{
		fn public(&self) -> T::Public {
			self.public.clone()
		}
	}

	impl<T: SigningTypes + Config> SignedPayload<T>
	for DispatchPayload<T::AuthorityAres, T::Public, T::BlockNumber>
	{
		fn public(&self) -> T::Public {
			self.public.clone()
		}
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config + oracle_finance::Config<Self::FinanceInstance>{

		/// The identifier type for an offchain worker.
		type OffchainAppCrypto: AppCrypto<Self::Public, Self::Signature>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type UnsignedPriority: Get<TransactionPriority>;

		type FinanceInstance: Clone + Copy + PartialEq + Eq ;

		type OracleFinanceHandler: IForPrice<Self, Self::FinanceInstance> +
			IForReporter<Self, Self::FinanceInstance> +
			IForReward<Self, Self::FinanceInstance>;

		type PriceProvider: SymbolInfo<Self::BlockNumber>;

		/// RequestOrigin
		type RequestOrigin: EnsureOrigin<Self::Origin>;

		/// ocw store key pair.
		type AuthorityAres: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ UncheckedFrom<[u8; 32]>
			+ MaxEncodedLen
			+ From<AuthorityId>;

		type StashAndAuthorityPort: IStashAndAuthority<Self::AccountId, Self::AuthorityAres>;

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn reminder_count)]
	pub type ReminderCount<T: Config> = StorageValue<_, ReminderIden>;

	#[pallet::storage]
	#[pallet::getter(fn reminder_list)]
	pub type ReminderList<T: Config> = StorageMap<_, Twox64Concat,
		ReminderIden ,
		PriceTrigger<T::AccountId, ChainPrice, T::BlockNumber, RepeatCount, ReminderCondition<PriceKey, ChainPrice>, ReminderReceiver>
	>;

	#[pallet::storage]
	#[pallet::getter(fn symbol_list)]
	pub type SymbolList<T: Config> = StorageMap<_, Twox64Concat, PriceKey , ReminderIdenList >;

	#[pallet::storage]
	#[pallet::getter(fn owner_list)]
	pub type OwnerList<T: Config> = StorageMap<_, Twox64Concat, T::AccountId , ReminderIdenList >;

	#[pallet::storage]
	#[pallet::getter(fn security_deposit)]
	pub type SecurityDeposit<T: Config> = StorageValue<_, BalanceOf<T, T::FinanceInstance>>;

	#[pallet::storage]
	#[pallet::getter(fn max_pending_keep_bn)]
	pub type MaxPendingKeepBn<T: Config> = StorageValue<_, T::BlockNumber>;

	#[pallet::storage]
	#[pallet::getter(fn max_waiting_keep_bn)]
	pub type MaxWaitingKeepBn<T: Config> = StorageValue<_, T::BlockNumber>;

	#[pallet::storage]
	#[pallet::getter(fn waiting_send_list)]
	pub type WaitingSendList<T: Config> = StorageValue<_, ReminderSendList<ReminderIden, T::BlockNumber>>;

	#[pallet::storage]
	#[pallet::getter(fn offence_sender)]
	pub type OffenceSender<T: Config> = StorageMap<_, Twox64Concat, T::AccountId , ReminderSendList<ReminderIden, T::BlockNumber> >;

	#[pallet::storage]
	#[pallet::getter(fn pending_send_list)]
	pub(super) type PendingSendList<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId, // purchased_id,
		Blake2_128Concat,
		(ReminderIden, T::BlockNumber), // price_key,
		T::BlockNumber,
		OptionQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub security_deposit: BalanceOf<T, T::FinanceInstance>,
		pub max_pending_keep_bn: T::BlockNumber,
		pub max_waiting_keep_bn: T::BlockNumber,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				security_deposit: 0u32.into(),
				max_pending_keep_bn: 60u32.into(),
				max_waiting_keep_bn: 180u32.into(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			SecurityDeposit::<T>::put(self.security_deposit);
			MaxPendingKeepBn::<T>::put(self.max_pending_keep_bn);
			MaxWaitingKeepBn::<T>::put(self.max_waiting_keep_bn);
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit_dispatch_action {
				dispatch_payload: ref payload,
				ref signature,
			} = call
			{
				log::info!( target: DEBUG_TARGET, "Validate unsigned: submit_dispatch_action",);
				let stash_infos =  T::StashAndAuthorityPort::get_stash_id(&payload.auth.clone());
				if stash_infos.is_none() {
					log::error!( target: DEBUG_TARGET, "Error: Submitter must be a validator of submit_dispatch_action!",);
					return InvalidTransaction::BadProof.into();
				}

				let current_bn = <frame_system::Pallet<T>>::block_number();

				let priority_num: u64 = T::UnsignedPriority::get();

				ValidTransaction::with_tag_prefix("ares-reminder::validate::submit_dispatch_action")
					.priority(priority_num.saturating_add(1))
					.and_provides(current_bn)
					.longevity(5)
					.propagate(true)
					.build()

			} else if let Call::submit_complete_reminder {
				complete_payload: ref payload,
				ref signature,
			} = call
			{
				log::info!( target: DEBUG_TARGET, "Validate unsigned: submit_complete_reminder",);

				let stash_id =  T::StashAndAuthorityPort::get_stash_id(&payload.auth.clone());
				if stash_id.is_none() {
					log::error!( target: DEBUG_TARGET, "Error: Submitter must be a validator of submit_complete_reminder!",);
					return InvalidTransaction::BadProof.into();
				}
				let stash_id = stash_id.unwrap();

				// Get reminder
				let reminder_create_bn = PendingSendList::<T>::get(&stash_id, &payload.reminder);
				if reminder_create_bn.is_none() {
					log::error!( target: DEBUG_TARGET, "Error: Submitter must be a validator of submit_complete_reminder!",);
					return InvalidTransaction::BadProof.into();
				}

				let current_bn = <frame_system::Pallet<T>>::block_number();
				let priority_num: u64 = T::UnsignedPriority::get();

				log::info!( target: DEBUG_TARGET, "Will return ValidTransaction::build() {:?}", (payload.public.clone(), current_bn, payload.reminder.clone()));
				ValidTransaction::with_tag_prefix("ares-reminder::validate::submit_complete_reminder")
					.priority(priority_num.saturating_add(1))
					.and_provides((payload.public.clone(), current_bn, payload.reminder.clone()))
					// .and_provides(current_bn)
					.longevity(5)
					.propagate(true)
					.build()

			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		CreateReminder {
			rid: ReminderIden,
			symbol: PriceKey,
			trigger: PriceTrigger<
				T::AccountId,
				ChainPrice,
				T::BlockNumber,
				RepeatCount,
				ReminderCondition<PriceKey, ChainPrice>,
				ReminderReceiver,
			>,
		},
		ReminderMsg {
			rid: ReminderIden,
			remaining_count: u32,
			update_bn: T::BlockNumber,
			submitter: T::AccountId,
			response_mark: Option<Vec<u8>>,
			status: Option<u32>,
		},
		ReminderReleased {
			rid: ReminderIden,
			release_bn: T::BlockNumber,
		}
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		///
		SymbolNotSupported,
		///
		InsufficientBalance,
		///
		EstimatedCostExceedsMaximumFee,
		///
		ReminderListExceedsMaximumLimit,
		///
		ReminderIdNotExists,
		///
		NotTheReminderOwner,
		///
		PendingReminderNotExists,
		///
		CanNotFindStashAccount,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
		where <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		/// Offchain Worker entry point.
		fn offchain_worker(_block_number: T::BlockNumber) {
			Self::call_reminder(true);
			let waiting_list = WaitingSendList::<T>::get().unwrap_or(ReminderSendList::default());
			if waiting_list.len() > 0 {
				Self::save_dispatch_action();
			}
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		#[pallet::weight(0)]
		#[transactional]
		pub fn add_reminder(
			origin: OriginFor<T>,
			condition: ReminderCondition<PriceKey, ChainPrice>,
			receiver: ReminderReceiver, // call_back_url
			interval: T::BlockNumber,
			repeat_count: u32,
			tip: Option<ReminderTriggerTip>,
			max_fee: BalanceOf<T, T::FinanceInstance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let current_bn = <frame_system::Pallet<T>>::block_number();
			//
			let estimate_fee = T::OracleFinanceHandler::calculate_fee(repeat_count.clone());
			ensure!(estimate_fee <= max_fee, Error::<T>::EstimatedCostExceedsMaximumFee);
			ensure!(estimate_fee <= T::Currency::free_balance(&who), Error::<T>::InsufficientBalance);

			let symbol = match condition.clone() {
				ReminderCondition::TargetPriceModel {
					price_key, anchor_price
				} => {price_key}
			};

			let oracle_result= T::PriceProvider::price(&symbol);
			ensure!(oracle_result.is_ok(), Error::<T>::SymbolNotSupported);

			// Get current reminder id
			let current_reminder_id: ReminderIden = ReminderCount::<T>::get().unwrap_or(0);
			let order_id = OrderIdEnum::Integer(current_reminder_id);

			// Get security deposit fee because must be do the deposit fee first.
			let deposit_fee = SecurityDeposit::<T>::get().unwrap_or(0u32.into());

			// Try to deposit some balance for security.
			// params: Who, pid, and deposit.
			T::OracleFinanceHandler::lock_deposit(&who, &order_id, deposit_fee)?;

			// Try to reserve some fee to pay the order.
			let payment_res = T::OracleFinanceHandler::reserve_fee(&who, &order_id, repeat_count.clone());
			// println!("payment_res = {:?}", payment_res);

			match payment_res {
				OcwPaymentResult::InsufficientBalance(_, _) => {
					// unlock deposit fee
					T::OracleFinanceHandler::unlock_deposit(&order_id)?;
					return Err(Error::<T>::InsufficientBalance.into());
				},
				OcwPaymentResult::Success(_, _) => {},
			}


			let (number, fraction_length, _) = oracle_result.unwrap();

			// Get current chain price
			let current_chain_price = ChainPrice::new((number, fraction_length));
			// println!("current_chain_price == {:?}", &current_chain_price);
			let insert_data = PriceTrigger {
				owner: who.clone(),
				interval_bn: interval,
				repeat_count: repeat_count,
				create_bn: current_bn,
				update_bn: 0u32.into(),
				price_snapshot: current_chain_price,
				trigger_condition: condition,
				trigger_receiver: receiver,
				last_check_infos: None,
				tip,
			};

			// Insert
			Self::add_price_trigger(&who, &symbol, insert_data)?;
			Ok(())
		}

		#[pallet::weight(0)]
		#[transactional]
		pub fn update_reminder(
			origin: OriginFor<T>,
			rid: ReminderIden,
			condition: ReminderCondition<PriceKey, ChainPrice>,
			receiver: ReminderReceiver, // call_back_url
			interval: T::BlockNumber,
			repeat_count: u32,
			tip: Option<ReminderTriggerTip>,
			max_fee: BalanceOf<T, T::FinanceInstance>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let trigger = ReminderList::<T>::get(&rid);
			ensure!(trigger.is_some(), Error::<T>::ReminderIdNotExists);
			let trigger = trigger.unwrap();
			Self::check_trigger_owner(&who, &trigger)?;

			let order_id = OrderIdEnum::Integer(rid.clone());

			// Get exists reserve fee
			let old_reserve_fee = T::OracleFinanceHandler::get_reserve_fee(&order_id);
			// New estimate fee.
			let estimate_fee = T::OracleFinanceHandler::calculate_fee(repeat_count.clone());
			ensure!(estimate_fee.saturating_sub(old_reserve_fee) <= max_fee, Error::<T>::EstimatedCostExceedsMaximumFee);

			// Remove old locked deposit
			T::OracleFinanceHandler::unlock_deposit(&order_id)?;
			// Get security deposit fee because must be do the deposit fee first.
			let deposit_fee = SecurityDeposit::<T>::get().unwrap_or(0u32.into());
			// Try to deposit some balance for security.
			T::OracleFinanceHandler::lock_deposit(&who, &order_id, deposit_fee)?;
			if estimate_fee > T::Currency::free_balance(&who).saturating_add(old_reserve_fee) {
				return Err(Error::<T>::InsufficientBalance.into());
			}
			// Remove old reserve fee
			T::OracleFinanceHandler::unreserve_fee(&order_id)?;
			// Try to reserve some fee to pay the order.
			let payment_res = T::OracleFinanceHandler::reserve_fee(&who, &order_id, repeat_count.clone());
			match payment_res {
				OcwPaymentResult::InsufficientBalance(_, _) => {
					// unlock deposit fee
					T::OracleFinanceHandler::unlock_deposit(&order_id)?;
					return Err(Error::<T>::InsufficientBalance.into());
				},
				OcwPaymentResult::Success(_, _) => {},
			}
			// ------
			let symbol = match condition.clone() {
				ReminderCondition::TargetPriceModel {
					price_key, anchor_price
				} => {price_key}
			};

			let oracle_result= T::PriceProvider::price(&symbol);
			ensure!(oracle_result.is_ok(), Error::<T>::SymbolNotSupported);

			let (number, fraction_length, _) = oracle_result.unwrap();

			// Get current chain price
			let current_chain_price = ChainPrice::new((number, fraction_length));

			let insert_data = PriceTrigger {
				owner: who.clone(),
				interval_bn: interval,
				repeat_count: repeat_count,
				create_bn: trigger.create_bn.clone(),
				update_bn: trigger.update_bn.clone(),
				price_snapshot: current_chain_price,
				trigger_condition: condition,
				trigger_receiver: receiver,
				last_check_infos: None,
				tip,
			};

			// Insert
			Self::update_price_trigger(rid, &who, &symbol, insert_data)?;

			Ok(())

		}

		#[pallet::weight(0)]
		pub fn remove_reminder(origin: OriginFor<T>, rid: ReminderIden) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Check delete permissions.

			let trigger = ReminderList::<T>::get(&rid);
			ensure!(trigger.is_some(), Error::<T>::ReminderIdNotExists);
			let trigger = trigger.unwrap();
			Self::check_trigger_owner(&who, &trigger)?;

			Self::do_remove_reminder(&rid);

			// Unlock deposit
			T::OracleFinanceHandler::unlock_deposit(&OrderIdEnum::Integer(rid.clone()))?;
			T::OracleFinanceHandler::unreserve_fee(&OrderIdEnum::Integer(rid.clone()))?;

			Ok(())
		}


		#[pallet::weight(0)]
		#[transactional]
		pub fn submit_dispatch_action(
			origin: OriginFor<T>,
			dispatch_payload: DispatchPayload<T::AuthorityAres, T::Public, T::BlockNumber>,
			_signature: OffchainSignature<T>,
		) -> DispatchResult {
			ensure_none(origin)?;
			Self::dispatch_waiting_list();
			Ok(())
		}

		#[pallet::weight(0)]
		#[transactional]
		pub fn submit_complete_reminder (
			origin: OriginFor<T>,
			complete_payload: CompletePayload<ReminderIden, Vec<u8>, u32, T::AuthorityAres, T::Public, T::BlockNumber>,
			_signature: OffchainSignature<T>,
		) -> DispatchResult {
			ensure_none(origin)?;

			//TODO 先要合适验证人身份，validator 必须是一个 Ares 验证人
			let stash_id = T::StashAndAuthorityPort::get_stash_id(&complete_payload.auth);
			ensure!(stash_id.is_some(), Error::<T>::CanNotFindStashAccount);
			let stash_id = stash_id.unwrap();

			// Get reminder
			let reminder_create_bn = PendingSendList::<T>::get(&stash_id, &complete_payload.reminder);
			ensure!(reminder_create_bn.is_some(), Error::<T>::PendingReminderNotExists);

			//TODO 检查 rid 是否存在，并且提取对应的信息，这里会扣除费用，并启减少通知次数。
			// Get trigger
			let trigger = ReminderList::<T>::get(&complete_payload.reminder.0);
			ensure!(trigger.is_some(), Error::<T>::ReminderIdNotExists);
			// Reduce repeat count.
			let mut trigger = trigger.unwrap();
			let current_bn = <frame_system::Pallet<T>>::block_number();
			trigger.repeat_count = trigger.repeat_count - 1;

			let rid = complete_payload.reminder.0.clone();

			if trigger.repeat_count >= 1 {
				// Update trigger
				ReminderList::<T>::insert(&rid, trigger.clone());
			}else{
				// Remove trigger
				Self::do_remove_reminder(&rid);
			}

			// take fee
			T::OracleFinanceHandler::pay_to(&OrderIdEnum::Integer(rid.clone()), 1)?;

			if trigger.repeat_count < 1 {
				// Unlock deposit
				T::OracleFinanceHandler::unlock_deposit(&OrderIdEnum::Integer(rid.clone()));
				T::OracleFinanceHandler::unreserve_fee(&OrderIdEnum::Integer(rid.clone()));
			}

			PendingSendList::<T>::remove(&stash_id, &complete_payload.reminder);

			Self::deposit_event(Event::ReminderMsg {
				rid,
				remaining_count: trigger.repeat_count,
				update_bn: complete_payload.reminder.1,
				submitter: stash_id ,
				response_mark: complete_payload.response_mark,
				status: complete_payload.status,
			});

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn update_security_deposit(origin: OriginFor<T>, deposit: BalanceOf<T, T::FinanceInstance>) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			SecurityDeposit::<T>::put(deposit);
			Ok(())
		}

	}

	impl<T: Config> Pallet<T> {
		fn add_price_trigger(who: &T::AccountId, symbol: &PriceKey, trigger: PriceTrigger<T::AccountId, ChainPrice, T::BlockNumber, RepeatCount, ReminderCondition<PriceKey, ChainPrice>, ReminderReceiver>) -> DispatchResult {
			let current_reminder_id: ReminderIden = ReminderCount::<T>::get().unwrap_or(0);

			let mut old_symbol_list = SymbolList::<T>::get(symbol).unwrap_or(ReminderIdenList::default());
			ensure!(old_symbol_list.try_push(current_reminder_id.clone()).is_ok(), Error::<T>::ReminderListExceedsMaximumLimit);
			SymbolList::<T>::insert(symbol, old_symbol_list);

			let mut old_owner_list = OwnerList::<T>::get(who).unwrap_or(ReminderIdenList::default());
			ensure!(old_owner_list.try_push(current_reminder_id.clone()).is_ok(), Error::<T>::ReminderListExceedsMaximumLimit);
			OwnerList::<T>::insert(who, old_owner_list);

			ReminderList::<T>::insert(current_reminder_id.clone(), trigger.clone());

			// upgrade reminder id.
			ReminderCount::<T>::put(current_reminder_id.saturating_add(1));

			Self::deposit_event(Event::CreateReminder {
				rid: current_reminder_id,
				symbol: symbol.clone(),
				trigger,
			});

			Ok(())
		}

		fn update_price_trigger(rid: ReminderIden, who: &T::AccountId, new_symbol: &PriceKey, trigger: PriceTrigger<T::AccountId, ChainPrice, T::BlockNumber, RepeatCount, ReminderCondition<PriceKey, ChainPrice>, ReminderReceiver>) -> DispatchResult {

			let old_reminder_tigger = ReminderList::<T>::get(&rid);
			ensure!(old_reminder_tigger.is_some(), Error::<T>::ReminderIdNotExists);
			let old_trigger_condition = old_reminder_tigger.unwrap().trigger_condition;

			let old_symbol = match old_trigger_condition {
				ReminderCondition::TargetPriceModel {
					price_key, anchor_price
				} => {price_key}
			};

			// Check and update old_symbol_list
			let mut old_symbol_list = SymbolList::<T>::get(&old_symbol).unwrap_or(ReminderIdenList::default());
			let old_symbol_list_count = old_symbol_list.len();
			old_symbol_list.retain(|old_iden|{
				old_iden != &rid
			});
			ensure!(old_symbol_list_count.saturating_sub(1) == old_symbol_list.len(), Error::<T>::ReminderIdNotExists);

			// Check and update owner list, this list only check completeness no need to update.
			// let mut old_owner_list = OwnerList::<T>::get(who).unwrap_or(ReminderIdenList::default());
			// let old_owner_list_count = old_owner_list.len();
			// old_owner_list.retain(|old_iden|{
			// 	old_iden != &rid
			// });
			// ensure!(old_owner_list_count.saturating_sub(1) == old_owner_list.len(), Error::<T>::ReminderIdNotExists);

			// Update symbol_list
			if &old_symbol != new_symbol {
				SymbolList::<T>::insert(&old_symbol, old_symbol_list);
				let mut new_symbol_list = SymbolList::<T>::get(&new_symbol).unwrap_or(ReminderIdenList::default());
				ensure!(new_symbol_list.try_push(rid.clone()).is_ok(), Error::<T>::ReminderListExceedsMaximumLimit);
				SymbolList::<T>::insert(new_symbol, new_symbol_list);
			}

			ReminderList::<T>::insert(rid.clone(), trigger);

			Ok(())
		}

		fn check_trigger_owner(who: &T::AccountId, trigger: &PriceTrigger<T::AccountId, ChainPrice, T::BlockNumber, RepeatCount, ReminderCondition<PriceKey, ChainPrice>, ReminderReceiver> ) -> DispatchResult {
			// Check delete permissions.
			// let trigger = ReminderList::<T>::get(rid);
			// ensure!(trigger.is_some(), Error::<T>::ReminderIdNotExists);
			// let trigger = trigger.unwrap();
			ensure!(&trigger.owner == who, Error::<T>::NotTheReminderOwner);
			Ok(())
		}

		pub fn get_round(waiting_list_size:usize, validator_size:usize) -> usize {
			if waiting_list_size % validator_size > 0 {
				waiting_list_size / validator_size + 1
			}else{
				waiting_list_size / validator_size
			}
		}

		pub fn do_remove_reminder(rid: &ReminderIden) {
			let trigger = ReminderList::<T>::get(rid);
			if trigger.is_none() {
				return
			}
			let trigger = trigger.unwrap();

			let symbol = match trigger.trigger_condition.clone() {
				ReminderCondition::TargetPriceModel {
					price_key, anchor_price
				} => {price_key}
			};

			let mut old_symbol_list = SymbolList::<T>::get(&symbol).unwrap_or(ReminderIdenList::default());
			old_symbol_list.retain(|reminder_id| {
				reminder_id != rid
			});

			let mut old_owner_list = OwnerList::<T>::get(&trigger.owner).unwrap_or(ReminderIdenList::default());
			old_owner_list.retain(|reminder_id| {
				reminder_id != rid
			});

			SymbolList::<T>::insert(&symbol, old_symbol_list);
			OwnerList::<T>::insert(&trigger.owner, old_owner_list);
			ReminderList::<T>::remove(rid);

			let current_bn = <frame_system::Pallet<T>>::block_number();

			Self::deposit_event(Event::ReminderReleased {
				rid: rid.clone(),
				release_bn: current_bn
			});

		}

		pub fn dispatch_waiting_list() {

			let validator_list = T::StashAndAuthorityPort::get_list_of_storage();
			let waiting_list = WaitingSendList::<T>::get().unwrap_or(ReminderSendList::default());
			let current_bn = <frame_system::Pallet<T>>::block_number();

			log::info!( target: DEBUG_TARGET, "Pallet::dispatch_waiting_list {:?},{:?},{:?}", &waiting_list, &validator_list, &waiting_list,);

			for round_idx in 0usize..Self::get_round(waiting_list.len(), validator_list.len()) {
				log::info!(target: DEBUG_TARGET, "round_idx == {:?}", &round_idx);
				validator_list.iter().enumerate().for_each(|(idx, validator_data)| {
					let round_idx: T::BlockNumber = (round_idx as u32).into();
					let mod_base = current_bn.saturating_add((idx as u32).into());
					let validator_count: T::BlockNumber = (validator_list.len() as u32).into();
					let base_idx = mod_base % validator_count + validator_count.saturating_mul(round_idx);
					let base_idx: u64 = base_idx.try_into().ok().unwrap();
					let base_idx = base_idx as usize;
					let data = waiting_list.get(base_idx);

					// round_idx.
					if data.is_some() {
						PendingSendList::<T>::insert(validator_data.0.clone(), data.clone().unwrap(), current_bn);
					}
				});
			}
			WaitingSendList::<T>::put(ReminderSendList::default());
		}
	}

	impl <T: Config> IOracleAvgPriceEvents<T::BlockNumber, PriceKey, FractionLength> for Pallet<T> {
		fn avg_price_update(symbol: PriceKey, bn: T::BlockNumber, price: u64, fraction_length: FractionLength) {
			log::info!( target: DEBUG_TARGET, "IOracleAvgPriceEvents::avg_price_update {:?},{:?},{:?},{:?}", &symbol, &bn, &price, &fraction_length);

			let push_wait_list = |rid: ReminderIden, bn: T::BlockNumber| {
				let mut send_list = WaitingSendList::<T>::get().unwrap_or(ReminderSendList::default());
				let res = send_list.try_push((rid, bn));
				if res.is_ok() {
					WaitingSendList::<T>::put(send_list);
				}
				res
			};

			//
			let reminder_list = SymbolList::<T>::get(&symbol).unwrap_or(ReminderIdenList::default());

			//
			for rid in reminder_list {
				// Get trigger
				let opt_trigger = ReminderList::<T>::get(rid);
				if opt_trigger.is_none() {
					break;
				}
				let mut trigger = opt_trigger.unwrap();
				if trigger.repeat_count <= 0 {
					break;
				}

				// println!("A1 :: {:?}", opt_trigger.clone());
				let current_bn = bn ;// <frame_system::Pallet<T>>::block_number();
				// check interval
				// if trigger.last_check_infos.is_some() {
				// 	let (_last_check_price, last_check_bn) = trigger.last_check_infos.clone().unwrap();
				// 	if &current_bn.saturating_sub(last_check_bn) <= &trigger.interval_bn {
				// 		break;
				// 	}
				// }
				if trigger.update_bn > 0u32.into() && &current_bn.saturating_sub(trigger.update_bn) <= &trigger.interval_bn {
					break;
				}

				// Checker satisfaction conditions
				let condition = trigger.trigger_condition.clone();
				// Current chain price
				let current_chain_price = ChainPrice::new((price, fraction_length));

				match condition {
					ReminderCondition::TargetPriceModel { price_key, anchor_price} => {
						// first check
						if &price_key == &symbol {

							let current_balance = <ChainPrice as ConvertChainPrice<u64, u32>>::try_to_price(current_chain_price.clone(), 6);
							let snapshot_balance =  <ChainPrice as ConvertChainPrice<u64, u32>>::try_to_price(trigger.price_snapshot.clone(), 6);
							let anchor_balance = <ChainPrice as ConvertChainPrice<u64, u32>>::try_to_price(anchor_price.clone(), 6);
							if snapshot_balance.is_none() || anchor_balance.is_none() {
								log::error!( target: DEBUG_TARGET, "current_balance {:?} or snapshot_balance {:?} or anchor_balance {:?} convert failed.", &current_balance, &trigger.price_snapshot, &anchor_price  );
								break;
							}
							let current_balance = current_balance.unwrap();
							let snapshot_balance = snapshot_balance.unwrap();
							let anchor_balance = anchor_balance.unwrap();
							let last_check_balance = if let Some((last_check_price, _)) = trigger.last_check_infos {
								<ChainPrice as ConvertChainPrice<u64, u32>>::try_to_price(last_check_price, 6)
							}else{
								Some(snapshot_balance.clone())
							};
							if last_check_balance.is_none() {
								log::error!( target: DEBUG_TARGET, "last_check_balance convert failed.");
								break;
							}
							let last_check_balance = last_check_balance.unwrap();

							if anchor_balance < snapshot_balance {
								//
								if last_check_balance > anchor_balance {
									// This condition must be met, that is to say, the last updated price must be above anchor_balance
									if &current_balance <= &anchor_balance {
										let res = push_wait_list(rid, current_bn);
										if res.is_err() {
											log::error!( target: DEBUG_TARGET, "wait list exceeds maximum limit.");
											break;
										}
										trigger.update_bn = current_bn;
									}
								}
							}else if anchor_balance >= snapshot_balance {
								//
								if last_check_balance < anchor_balance {
									if &current_balance >= &anchor_balance {
										let res = push_wait_list(rid, current_bn);
										if res.is_err() {
											log::error!( target: DEBUG_TARGET, "wait list exceeds maximum limit.");
											break;
										}
										trigger.update_bn = current_bn;
									}
								}
							}

							// Update last_check_balance
							trigger.last_check_infos = Some((current_chain_price, current_bn.clone()));
							ReminderList::<T>::insert(rid, trigger);
						}
					}
				}
			}
		}
	}
}

