#![cfg_attr(not(feature = "std"), no_std)]
#[cfg(feature = "std")]
use frame_support::traits::GenesisBuild;

use codec::{Encode, Decode};
use frame_support::pallet_prelude::TypeInfo;
use frame_support::traits::{Currency, ExistenceRequirement, Get};
use frame_support::transactional;
use frame_system::{
	offchain::{
		AppCrypto, CreateSignedTransaction, SendUnsignedTransaction,
		SignedPayload, Signer, SubmitTransaction,
	},
};
// use lite_json::json::JsonValue;
// use sp_core::crypto::KeyTypeId;
use sp_runtime::{traits::Zero, transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction}, RuntimeDebug, DispatchResult, Permill, SaturatedConversion};
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(any(feature = "runtime-benchmarks", test))]
mod benchmarking;

pub mod types;
pub mod migrations;

pub mod weights;

pub use pallet::*;
use crate::types::{AccountParticipateEstimates, BoundedVecOfSymbol, ChainPrice, ChooseTrigerPayload, ChooseType, ChooseWinnersPayload, ConvertChainPrice, EstimatesState, EstimatesType, MaximumOptions, MaximumParticipants, MultiplierOption, SymbolEstimatesConfig};
use frame_support::{BoundedVec, ensure};
use ares_oracle_provider_support::FractionLength;
use sp_runtime::traits::{AccountIdConversion, CheckedDiv};
use sp_runtime::traits::Saturating;
use sp_core::hexdisplay::HexDisplay;

pub const TARGET: &str = "ares::price-estimates";

pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::dispatch::TransactionPriority;
	use frame_support::pallet_prelude::{StorageDoubleMap, StorageMap, StorageValue, TransactionSource, ValueQuery};
	use super::*;
	use frame_support::{Blake2_128Concat, PalletId};
	use frame_support::storage::types::OptionQuery;
	use frame_support::traits::{Hooks, IsType, Len, NamedReservableCurrency};
	use frame_support::weights::Weight;
	use frame_system::pallet_prelude::*;
	use sp_runtime::offchain::storage_lock::{BlockAndTime, StorageLock};
	use sp_runtime::traits::{StaticLookup, ValidateUnsigned};
	use sp_runtime::transaction_validity::TransactionValidityError;
	use ares_oracle::traits::SymbolInfo;
	use ares_oracle::types::OffchainSignature;
	use bound_vec_helper::BoundVecHelper;

	use crate::types::{AccountParticipateEstimates, BoundedVecOfSymbol, BoundedVecOfAdmins, BoundedVecOfBscAddress, BoundedVecOfChooseWinnersPayload, BoundedVecOfCompletedEstimates, MaximumAdmins, MaximumEstimatesPerSymbol, MaximumParticipants, MaximumWinners, StringLimit, SymbolEstimatesConfig, Releases, ChooseType};
	use crate::weights::WeightInfo;

	/// This pallet's configuration trait
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The overarching dispatch call type.
		type Call: From<Call<Self>>;

		/// The identifier type for an offchain worker.
		type OffchainAppCrypto: AppCrypto<Self::Public, Self::Signature>;

		/// The Lottery's pallet id
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type MaxEstimatesPerSymbol: Get<u32>;

		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;

		type Currency: Currency<Self::AccountId> + NamedReservableCurrency<Self::AccountId, ReserveIdentifier = [u8; 8]>;

		type PriceProvider: SymbolInfo<Self::BlockNumber>;

		#[pallet::constant]
		type MaxQuotationDelay: Get<Self::BlockNumber>;

		#[pallet::constant]
		type MaxEndDelay: Get<Self::BlockNumber>;

		#[pallet::constant]
		type MaximumKeepLengthOfOldData: Get<Self::BlockNumber>;

		type WeightInfo: WeightInfo;

	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {

		fn on_initialize(n: T::BlockNumber) -> Weight {
			if !Self::is_active() {
				return 0;
			}
			let mut to_active_symbol: Vec<(BoundedVecOfSymbol, EstimatesType)> = Vec::new();
			PreparedEstimates::<T>::iter().for_each(|(symbol, config)| {
				if config.start <= n {
					log::debug!(
						target: TARGET,
						"symbol: {:?} to active .. config: {:?}",
						&symbol,
						config
					);
					to_active_symbol.push(symbol);
				}
			});
			// -
			to_active_symbol.iter().for_each(|symbol| {
				if !ActiveEstimates::<T>::contains_key(&symbol) &&
					!UnresolvedEstimates::<T>::contains_key(&symbol) {
					let config = PreparedEstimates::<T>::get(&symbol);
					if let Some(mut config) = config {
						// let mut config = config.as_mut().unwrap();
						config.state = EstimatesState::Active;
						PreparedEstimates::<T>::remove(&symbol);
						ActiveEstimates::<T>::insert(&symbol, config)
					}
				}
			});
			0
		}

		/// Offchain Worker entry point.
		fn offchain_worker(block_number: T::BlockNumber) {
			// Self::do_submit_unsigned_test(block_number);
			// Create a lock with the maximum deadline of number of blocks in the unsigned phase.
			// This should only come useful in an **abrupt** termination of execution, otherwise the
			// guard will be dropped upon successful execution.
			let mut lock =
				StorageLock::<BlockAndTime<frame_system::Pallet<T>>>::with_block_deadline(b"price-estimates", 10);

			match lock.try_lock() {
				Ok(_guard) => {
					Self::do_synchronized_offchain_worker(block_number);
				}
				Err(deadline) => {
					log::error!(
						target: TARGET,
						"offchain worker lock not released, deadline is {:?}",
						deadline
					);
				}
			};

			if block_number % 100u32.into() == Zero::zero() {
				Self::do_data_cleaning(block_number);
			}
		}
	}

	/// A public part of the pallet.
	#[pallet::call]
	impl<T: Config> Pallet<T> {

		// #[pallet::weight(0)]
		// pub fn submit_unsigned_test (
		// 	origin: OriginFor<T>,
		// 	block_number: T::BlockNumber,
		// ) -> DispatchResult {
		// 	ensure_none(origin)?;
		// 	<NextUnsignedAt2<T>>::put(block_number);
		// 	Ok(().into())
		// }

		#[pallet::weight(0)]
		#[transactional]
		pub fn data_cleaning (
			origin: OriginFor<T>,
		) -> DispatchResult {
			ensure_none(origin)?;

			// MaximumKeepLengthOfOldData
			let current_bn = frame_system::Pallet::<T>::block_number();
			let unsign_at = NextDataCleanUnsignedAt::<T>::get();
			ensure!(current_bn >= unsign_at.saturating_add(T::MaximumKeepLengthOfOldData::get()), Error::<T>::TooOften);

			NextDataCleanUnsignedAt::<T>::put(current_bn);

			// Loop completedEstimates
			CompletedEstimates::<T>::iter().for_each(|(keypair, mut es_config_vec)|{
				let begin_count = es_config_vec.len();
				let mut removed_estimates = BoundedVecOfCompletedEstimates::<T::BlockNumber, BalanceOf<T>>::default();
				es_config_vec.retain(|es_config|{
					let distribute_bn: T::BlockNumber = es_config.distribute;
					if current_bn.saturating_sub(distribute_bn) >= T::MaximumKeepLengthOfOldData::get() {
						let estimate_id: u64 = es_config.id.clone();
						// estimates.winners
						Winners::<T>::remove(&keypair, estimate_id);
						// estimates.participants
						Participants::<T>::remove(&keypair, estimate_id);
						//
						EstimatesInitDeposit::<T>::remove(&keypair, estimate_id);
						// Add to remove list
						let _res = removed_estimates.try_push(es_config.clone());
						return false;
					}
					return true;
				});
				let last_count = es_config_vec.len();
				if begin_count > last_count {
					CompletedEstimates::<T>::insert(keypair, es_config_vec);
					Self::deposit_event(Event::<T>::RemovedEstimates {
						list: removed_estimates
					});
				}
			});

			Ok(().into())
		}

		#[pallet::weight(0)]
		#[transactional]
		pub fn force_complete(
			origin: OriginFor<T>,
			symbol: Vec<u8>,
			estimates_type: EstimatesType,
			ruling_price: u64,
			ruling_fraction_length: u32,
		) -> DispatchResult {
			ensure!(Self::is_active(), Error::<T>::PalletInactive);

			let caller = ensure_signed(origin.clone())?;
			let members: BoundedVec<T::AccountId, MaximumAdmins> = Self::admins();
			ensure!(members.contains(&caller), Error::<T>::NotMember);

			// Get estimates config
			let symbol: BoundedVec<u8, StringLimit> =
				symbol.clone().try_into().map_err(|_| Error::<T>::BadMetadata)?;
			let storage_key=(symbol.clone(), estimates_type.clone());

			let config = UnresolvedEstimates::<T>::get(&storage_key);
			ensure!(config.is_some(), Error::<T>::UnresolvedEstimatesNotExist);

			let source_acc = Self::account_id(storage_key.clone());
			ensure!(source_acc.is_some(), Error::<T>::AddressInvalid);
			// let source_acc = source_acc.unwrap();
			let config = config.unwrap();

			// let total_reward = T::Currency::free_balance(&source_acc);
			let total_reward = EstimatesInitDeposit::<T>::try_get(storage_key.clone(), config.id);
			ensure!(total_reward.is_ok(), Error::<T>::EstimatesInitDepositNotExist);
			let total_reward = total_reward.unwrap();

			let estimates_type = config.estimates_type.clone();
			let deviation = config.deviation.clone();
			let range = config.range.clone();
			let chain_fraction_length = T::PriceProvider::fraction(&symbol);
			ensure!(chain_fraction_length.is_some(), Error::<T>::SymbolNotSupported);
			let chain_fraction_length = chain_fraction_length.unwrap();
			let price = <ChainPrice as ConvertChainPrice<u64, u32>>::try_to_price(ChainPrice::new((ruling_price, ruling_fraction_length)), chain_fraction_length);
			ensure!(price.is_some(), Error::<T>::PriceInvalid);
			let price = price.unwrap();

			let accounts = Participants::<T>::get(&storage_key, config.id.clone());

			let price = (price, chain_fraction_length, frame_system::Pallet::<T>::block_number());
			// call_winner
			let winners: Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>> = Self::call_winners(
				&config,
				// estimates_type,
				// &symbol,
				total_reward,
				deviation,
				range,
				accounts,
				price, // (u64, FractionLength, T::BlockNumber)
			);

			Self::do_choose_winner(ChooseWinnersPayload {
				block_number: frame_system::Pallet::<T>::block_number(),
				winners: BoundedVecOfChooseWinnersPayload::create_on_vec(winners),
				public: None,
				estimates_id: config.id.clone(),
				symbol: storage_key,
				price: Some(price),
			}, EstimatesState::Unresolved)
		}

		#[pallet::weight(0)]
		#[transactional]
		pub fn new_estimates(
			origin: OriginFor<T>,
			symbol: Vec<u8>,
			start: T::BlockNumber,
			end: T::BlockNumber,
			distribute: T::BlockNumber,
			estimates_type: EstimatesType,
			deviation: Option<Permill>,
			range: Option<Vec<u64>>,
			range_fraction_length: Option<u32>,
			multiplier: Vec<MultiplierOption>,
			#[pallet::compact] init_reward: BalanceOf<T>,
			#[pallet::compact] price: BalanceOf<T>,
		) -> DispatchResult {
			ensure!(Self::is_active(), Error::<T>::PalletInactive);

			let caller = ensure_signed(origin.clone())?;
			let members: BoundedVec<T::AccountId, MaximumAdmins> = Self::admins();
			ensure!(members.contains(&caller), Error::<T>::NotMember);

			let check_price: Result<(u64, FractionLength, T::BlockNumber), ()> = T::PriceProvider::price(&symbol);
			ensure!(check_price.is_ok(), Error::<T>::UnableToGetPrice);
			let chain_fraction_length = T::PriceProvider::fraction(&symbol);
			ensure!(chain_fraction_length.is_some(), Error::<T>::SymbolNotSupported);
			let chain_fraction_length = chain_fraction_length.unwrap();

			ensure!(
				start >= frame_system::Pallet::<T>::block_number(),
				Error::<T>::EstimatesStartTooEarly
			);

			ensure!(
				start < end && end < distribute && start + LockedEstimates::<T>::get() < end,
				Error::<T>::EstimatesConfigInvalid
			);

			ensure!(
				(estimates_type == EstimatesType::DEVIATION && deviation.is_some() && range.is_none())
					|| (estimates_type == EstimatesType::RANGE && deviation.is_none() && range.is_some() && range_fraction_length.is_some() ),
				Error::<T>::EstimatesConfigInvalid
			);

			ensure!(
				price >= MinimumTicketPrice::<T>::get() && init_reward >= MinimumInitReward::<T>::get(),
				Error::<T>::EstimatesConfigInvalid
			);

			let multiplier: BoundedVec<MultiplierOption, MaximumOptions> =
				multiplier.clone().try_into().map_err(|_| Error::<T>::BadMetadata)?;

			// let symbol: BoundedVec<u8, StringLimit> =
			// 	symbol.clone().try_into().map_err(|_| Error::<T>::BadMetadata)?;

			let symbol = BoundedVecOfSymbol::try_create_on_vec(symbol.clone())
				.map_err(|_| Error::<T>::BadMetadata)?;

			let storage_key = (symbol.clone(), estimates_type.clone());

			type BoundedVecOfRange = BoundedVec<u64, MaximumOptions>;
			let mut range_vec = BoundedVecOfRange::default() ;

			let mut _range = range.clone();
			if _range.is_some() {
				let _range = _range.as_mut().unwrap();
				_range.sort();
				let new_range: BoundedVec<u64, MaximumOptions> =
					_range.clone().try_into().map_err(|_| Error::<T>::BadMetadata)?;

				let range_fraction_length = range_fraction_length.unwrap();
				let new_range = new_range.iter().map(|old_range_val|{
					<ChainPrice as ConvertChainPrice<u64, u32>>::try_to_price(ChainPrice::new((*old_range_val, range_fraction_length)), chain_fraction_length)
				}).collect::<Vec<Option<u64>>>();

				for range_value in new_range {
					if let Some(range_value) = range_value {
						let _res = range_vec.try_push(range_value);
					}
				}
			}

			let mut range: Option<BoundedVecOfRange> = None;
			if range_vec.len() > 0 {
				range = Some(range_vec);
			}


			ensure!(
				!PreparedEstimates::<T>::contains_key(&storage_key),
				Error::<T>::PreparedEstimatesExist
			);

			// Get and check subaccount.
			let source_acc = Self::account_id(storage_key.clone());
			ensure!(source_acc.is_some(), Error::<T>::SubAccountGenerateFailed);
			let source_acc = source_acc.unwrap();

			// log::debug!(target: TARGET, "RUN 1 symbol {:?}", &symbol);
			T::Currency::transfer(
				&caller,
				&source_acc,
				init_reward,
				ExistenceRequirement::KeepAlive,
			)?;

			// log::debug!(target: TARGET, "RUN 2 to_account = {:?}", &to_account);

			// let _0 = BalanceOf::<T>::from(0u32);
			let mut estimates_config = SymbolEstimatesConfig {
				symbol: symbol.clone(),
				estimates_type,
				id: 0,
				ticket_price: price,
				symbol_completed_price: 0,
				start,
				end,
				distribute,
				multiplier,
				deviation,
				symbol_fraction: chain_fraction_length,
				total_reward: init_reward,
				state: EstimatesState::InActive,
				range,
			};

			let id = SymbolEstimatesId::<T>::get(&storage_key);

			if let Some(id) = id {
				estimates_config.id = id; // symbol exist
			}

			let current_id = estimates_config.id;
			// Check id not be used.
			ensure!(
				EstimatesInitDeposit::<T>::try_get(storage_key.clone(), estimates_config.id).ok().is_none(),
				Error::<T>::EstimatesConfigInvalid
			);
			EstimatesInitDeposit::<T>::insert(storage_key.clone(), estimates_config.id, init_reward);
			SymbolEstimatesId::<T>::insert(storage_key.clone(), estimates_config.id.clone() + 1); //generate next id
			PreparedEstimates::<T>::insert(storage_key.clone(), &estimates_config);

			if !CompletedEstimates::<T>::contains_key(&storage_key) {
				let val = BoundedVec::<
					SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>,
					MaximumEstimatesPerSymbol,
				>::default();
				//
				CompletedEstimates::<T>::insert(storage_key.clone(), val);
			}

			Participants::<T>::insert(storage_key, current_id, BoundedVec::<AccountParticipateEstimates<T::AccountId, T::BlockNumber>, MaximumParticipants>::default());
			Self::deposit_event(Event::NewEstimates {
				estimate: estimates_config,
				who: caller,
			});

			Ok(())
		}

		#[pallet::weight(<T as Config>::WeightInfo::participate_estimates())]
		pub fn participate_estimates(
			origin: OriginFor<T>,
			symbol: Vec<u8>,
			estimated_type: EstimatesType,
			estimated_price: Option<u64>,
			estimated_fraction_length: Option<u32>,
			range_index: Option<u8>,
			multiplier: MultiplierOption,
			_bsc_address: Option<Vec<u8>>,
		) -> DispatchResult {
			ensure!(Self::is_active(), Error::<T>::PalletInactive);
			let caller = ensure_signed(origin)?;

			let chain_fraction_length = T::PriceProvider::fraction(&symbol);
			ensure!(chain_fraction_length.is_some(), Error::<T>::SymbolNotSupported);

			let per_symbol = symbol.clone().try_into().map_err(|_| Error::<T>::BadMetadata)?;
			let symbol: (BoundedVecOfSymbol, EstimatesType) = (per_symbol, estimated_type);


			// let bsc_address: Option<BoundedVecOfBscAddress> = if let Some(bsc) = bsc_address {
			// 	let bsc: BoundedVec<u8, StringLimit> =
			// 		bsc.clone().try_into().map_err(|_| Error::<T>::BadMetadata)?;
			// 	let mut bytes = [0u8; 40];
			// 	let r = hex::encode_to_slice(&bsc, &mut bytes);
			// 	ensure!(r.is_ok(), Error::<T>::AddressInvalid);
			// 	// ensure!(is_hex_address(&bytes), Error::<T>::AddressInvalid);
			// 	Some(bsc)
			// } else {
			// 	None
			// };

			let bsc_address: Option<BoundedVecOfBscAddress> = None;

			log::info!(target: TARGET, "price={:?}, fraction={:?}", estimated_price, estimated_fraction_length);

			let config = ActiveEstimates::<T>::get(&symbol);
			if config.is_none() {
				return Err(Error::<T>::ActiveEstimatesNotExist.into());
			}
			let config = config.unwrap();
			let start = config.start;
			let end = config.end;
			let estimates_id = config.id;
			let mut ticket_price = config.ticket_price.clone();
			let estimates_type = &config.estimates_type;
			let current = frame_system::Pallet::<T>::block_number();

			ensure!(
				start <= current && current + LockedEstimates::<T>::get() < end,
				Error::<T>::EstimatesStateError
			);

			ensure!(
				config.multiplier.contains(&multiplier),
				Error::<T>::MultiplierNotExist
			);

			match multiplier {
				MultiplierOption::Base(b) => {
					let _base = BalanceOf::<T>::from(b);
					ticket_price = ticket_price * _base;
				}
			}

			let mut acc_estimated_price = None;
			match estimates_type {
				EstimatesType::DEVIATION => {
					ensure!(
						estimated_price.is_some() && estimated_fraction_length.is_some() && range_index.is_none(),
						Error::<T>::ParameterInvalid
					);
					// Format final estimated_price
					let _price: u64 = estimated_price.unwrap();
					let _input_fraction_length: u32 = estimated_fraction_length.unwrap();
					let _chain_fraction_length: u32 = chain_fraction_length.unwrap();

					acc_estimated_price = <ChainPrice as ConvertChainPrice<u64, u32>>::try_to_price(ChainPrice::new((_price, _input_fraction_length)), _chain_fraction_length);

					ensure!(
						acc_estimated_price.is_some() ,
						Error::<T>::ParameterInvalid
					);
				}
				EstimatesType::RANGE => {
					ensure!(
						estimated_price.is_none() && estimated_fraction_length.is_none() && range_index.is_some(),
						Error::<T>::ParameterInvalid
					);
					ensure!(
						usize::from(range_index.unwrap()) <= config.range.unwrap().len(),
						Error::<T>::ParameterInvalid
					)
				}
			}

			Participants::<T>::try_mutate(&symbol, estimates_id, |accounts| -> DispatchResult {
				let found = accounts.iter().find(|account| -> bool { caller == account.account });
				if let Some(_) = found {
					return Err(Error::<T>::AccountEstimatesExist.into());
				}
				ensure!(
					T::Currency::free_balance(&caller) > ticket_price,
					Error::<T>::FreeBalanceTooLow
				);

				let estimates = AccountParticipateEstimates {
					account: caller.clone(),
					end,
					estimates: acc_estimated_price,
					range_index,
					bsc_address: bsc_address,
					multiplier,
					reward: 0,
				};

				// Get and check subaccount.
				let source_acc = Self::account_id(symbol.clone());
				ensure!(source_acc.is_some(), Error::<T>::SubAccountGenerateFailed);
				let source_acc = source_acc.unwrap();

				T::Currency::transfer(
					&caller,
					&source_acc,
					ticket_price,
					ExistenceRequirement::KeepAlive,
				)?;

				let init_deposit = EstimatesInitDeposit::<T>::try_get(symbol.clone(), config.id);
				ensure!(
					init_deposit.is_ok(),
					Error::<T>::EstimatesInitDepositNotExist
				);
				EstimatesInitDeposit::<T>::insert(symbol.clone(), config.id, init_deposit.unwrap().saturating_add(ticket_price));

				accounts
					.try_push(estimates.clone())
					.map_err(|_| Error::<T>::TooMany)?;

				Self::deposit_event(Event::ParticipateEstimates {
					symbol: symbol.0.clone(),
					id: estimates_id,
					estimate: estimates,
					who: caller,
					estimate_type: symbol.1.clone(),
					deposit: ticket_price,
				});
				Ok(())
			})
		}

		#[pallet::weight(0)]
		#[transactional]
		pub fn choose_winner(
			origin: OriginFor<T>,
			trigger_payload: ChooseTrigerPayload<T::Public>,
			_signature: OffchainSignature<T>,
		) -> DispatchResult {
			ensure_none(origin)?;

			ensure!(Self::is_active(), Error::<T>::PalletInactive);

			let symbol = trigger_payload.symbol.clone();

			let config = ActiveEstimates::<T>::get(&symbol);
			ensure!(config.is_some(), Error::<T>::EstimatesConfigInvalid);

			// Get and check subaccount.
			let source_acc = Self::account_id(symbol.clone());
			ensure!(source_acc.is_some(), Error::<T>::SubAccountGenerateFailed);

			let source_acc = source_acc.unwrap();
			let config = config.unwrap();
			let now = frame_system::Pallet::<T>::block_number();
			let end = config.end;
			// let symbol = config.symbol.clone();
			let deviation = config.deviation;
			let range = config.range.clone();
			let estimates_type = config.estimates_type.clone();
			let id = config.id;

			if now >= end {
				let participant_accounts = Participants::<T>::get(&symbol, id);
				let price: Result<(u64, FractionLength, T::BlockNumber), ()> = T::PriceProvider::price(&symbol.0);
				if let Ok(price) = price {
					if participant_accounts.len() == 0 {
						// If no one is involved, it will be forced to end.
						return Self::do_choose_winner(ChooseWinnersPayload {
							block_number: frame_system::Pallet::<T>::block_number(),
							winners: BoundedVecOfChooseWinnersPayload::default(),
							public: Some(trigger_payload.public),
							symbol,
							estimates_id: config.id.clone(),
							price: Some(price),
						}, EstimatesState::Active);
					} else {
						// If has participants
						// let total_reward = T::Currency::free_balance(&source_acc);
						let total_reward = EstimatesInitDeposit::<T>::try_get(symbol.clone(), config.id);
						ensure!(total_reward.is_ok(), Error::<T>::EstimatesInitDepositNotExist);
						let total_reward = total_reward.unwrap();

						if config.end <= now &&
							now.saturating_sub(T::MaxEndDelay::get()) <= config.end &&
							price.2 >= now.saturating_sub(T::MaxQuotationDelay::get())
						{
							let winners: Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>> = Self::call_winners(
								&config,
								// estimates_type,
								// &symbol,
								total_reward,
								deviation,
								range,
								participant_accounts,
								price,
							);

							return Self::do_choose_winner(ChooseWinnersPayload {
								block_number: frame_system::Pallet::<T>::block_number(),
								winners: BoundedVecOfChooseWinnersPayload::create_on_vec(winners),
								public: Some(trigger_payload.public),
								symbol,
								estimates_id: config.id.clone(),
								price: Some(price),
							}, EstimatesState::Active);

						} else {
							log::warn!(
								target: TARGET,
								"The price is too old. chain price create block number {:?} must >= now {:?} - {:?}",
								&price.2,
								&now,
								T::MaxQuotationDelay::get()
							);
							if !UnresolvedEstimates::<T>::contains_key(&symbol) {
								let config = ActiveEstimates::<T>::get(&symbol);
								if let Some(mut config) = config {
									// let mut config = config.as_mut().unwrap();
									config.state = EstimatesState::Unresolved;
									ActiveEstimates::<T>::remove(&symbol);
									UnresolvedEstimates::<T>::insert(&symbol, config)
								}
							}
							return Ok(());
						}
					}
				}
				// return DispatchResult::Err(DispatchError::try_from(Error::<T>::PriceInvalid).unwrap());
				return Err(Error::<T>::PriceInvalid.into())
			}else{
				log::warn!(target: TARGET, "The estimate is not over.");
			}
			// DispatchResult::Err(DispatchError::try_from(Error::<T>::IllegalCall).unwrap());
			Err(Error::<T>::IllegalCall.into())
		}

		// #[pallet::weight(10_000)]
		// pub fn claim(
		// 	origin: OriginFor<T>,
		// 	dest: <T::Lookup as StaticLookup>::Source,
		// 	#[pallet::compact] value: BalanceOf<T>,
		// ) -> DispatchResult {
		// 	let caller = ensure_signed(origin.clone())?;
		// 	let members: BoundedVec<T::AccountId, MaximumAdmins> = Self::admins();
		// 	ensure!(members.contains(&caller), Error::<T>::NotMember);
		//
		// 	let dest = T::Lookup::lookup(dest)?;
		// 	let id = T::PalletId::get().0;
		// 	// T::Currency::unreserve_named(&id, &dest, value);
		// 	Ok(())
		// }

		#[pallet::weight(0)]
		pub fn preference(
			origin: OriginFor<T>,
			admins: Option<Vec<T::AccountId>>,
			// whitelist: Option<Vec<T::AccountId>>,
			locked_estimates: Option<T::BlockNumber>,
			minimum_ticket_price: Option<BalanceOf<T>>,
			minimum_init_reward: Option<BalanceOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;
			if let Some(admins) = admins {
				let members: BoundedVec<T::AccountId, MaximumAdmins> =
					admins.clone().try_into().map_err(|_| Error::<T>::BadMetadata)?;
				Admins::<T>::put(members);
			}

			// if let Some(whitelist) = whitelist {
			// 	let members: BoundedVec<T::AccountId, MaximumWhitelist> =
			// 		whitelist.clone().try_into().map_err(|_| Error::<T>::BadMetadata)?;
			// 	Whitelist::<T>::put(members);
			// }

			if let Some(locked_estimates) = locked_estimates {
				LockedEstimates::<T>::put(locked_estimates);
			}
			if let Some(minimum_ticket_price) = minimum_ticket_price {
				MinimumTicketPrice::<T>::put(minimum_ticket_price);
			}
			if let Some(minimum_init_reward) = minimum_init_reward {
				MinimumInitReward::<T>::put(minimum_init_reward);
			}
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn active_pallet(origin: OriginFor<T>, active: bool) -> DispatchResult {
			ensure_root(origin)?;
			ActivePallet::<T>::put(active);
			Ok(())
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub admins: Vec<T::AccountId>,
		// pub white_list: Vec<T::AccountId>,
		pub locked_estimates: T::BlockNumber,
		pub minimum_ticket_price: BalanceOf<T>,
		pub minimum_init_reward: BalanceOf<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				admins: Vec::<T::AccountId>::new(),
				// white_list: Vec::<T::AccountId>::new(),
				locked_estimates: 0u32.into(),
				minimum_ticket_price: 0u32.into(),
				minimum_init_reward: 0u32.into(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Admins::<T>::put(BoundedVecOfAdmins::create_on_vec(self.admins.clone()));
			// Whitelist::<T>::put( BoundedVecOfWhitelist::create_on_vec(self.white_list.clone()));
			LockedEstimates::<T>::put(self.locked_estimates.clone());
			MinimumTicketPrice::<T>::put(self.minimum_ticket_price.clone());
			MinimumInitReward::<T>::put(self.minimum_init_reward.clone());
			ActivePallet::<T>::put(true);
		}
	}

	/// Events for the pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Deposit {
			who: T::AccountId,
			amount: BalanceOf<T>,
		},

		Reserved {
			id: [u8; 8],
			who: T::AccountId,
			amount: BalanceOf<T>,
		},

		NewEstimates {
			estimate: SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>,
			who: T::AccountId,
		},

		ParticipateEstimates {
			symbol: BoundedVec<u8, StringLimit>,
			id: u64,
			estimate: AccountParticipateEstimates<T::AccountId, T::BlockNumber>,
			who: T::AccountId,
			estimate_type: EstimatesType,
			deposit: BalanceOf<T>,
		},

		CompletedEstimates {
			config: SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>,
			winners: Vec<(T::AccountId, BalanceOf<T>)>
		},

		RemovedEstimates {
			list: BoundedVecOfCompletedEstimates::<T::BlockNumber, BalanceOf<T>>,
		},

		ChooseWinner {
			record: ChooseType<T::AccountId>,
		}
	}



	#[pallet::error]
	pub enum Error<T> {
		/// Invalid metadata given.
		BadMetadata,
		TooMany,
		PalletInactive,
		AddressInvalid,
		PriceInvalid,
		FreeBalanceTooLow,
		SymbolNotSupported,
		PreparedEstimatesExist,
		EstimatesStartTooEarly,
		EstimatesConfigInvalid,
		EstimatesStateError,
		ActiveEstimatesNotExist,
		AccountEstimatesExist,
		AlreadyEnded,
		AlreadyParticipating,
		ParameterInvalid,
		TooManyEstimates,
		NotMember,
		MultiplierNotExist,
		SubAccountGenerateFailed,
		UnableToGetPrice,
		UnresolvedEstimatesNotExist,
		IllegalCall,
		TooOften,
		EstimatesInitDepositNotExist,
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		// fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
		// 	// Firstly let's check that we call the right function.
		// 	if let Call::submit_unsigned_test { block_number } = call {
		// 		ValidTransaction::with_tag_prefix("for_kami_debug")
		// 			.priority(T::UnsignedPriority::get())
		// 			.and_provides(block_number)
		// 			.longevity(5)
		// 			.propagate(true)
		// 			.build()
		// 	} else {
		// 		InvalidTransaction::Call.into()
		// 	}
		// }

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			log::warn!(target: TARGET, "# on validate_unsigned start");

			if !Self::is_active() {
				log::error!(
					target: TARGET,
					"⛔️ validate_unsigned error. ActivePallet is False."
				);
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
			}

			if let Call::choose_winner {
				trigger_payload: ref payload,
				ref signature,
			} = call {
				let symbol = &payload.symbol;
				let config = ActiveEstimates::<T>::get(&symbol);
				if config.is_none() {
					log::error!(
						target: TARGET,
						"⛔️ validate_unsigned error. ActiveEstimates {:?} is None.",
						&symbol
					);
					return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
				}

				let config = config.unwrap();
				let now = frame_system::Pallet::<T>::block_number();

				// check end
				// if !(config.end <= now &&
				// 	now.saturating_sub(T::MaxEndDelay::get()) <= config.end)
				if !(config.end <= now )
				{
					log::error!(
						target: TARGET,
						"⛔️ validate_unsigned error. Not satisfied config.end = {:?} < now = {:?} .",
						// T::MaxEndDelay::get(),
						&config.end,
						now
					);
					return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
				}

				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
				if !signature_valid {
					log::error!(
						target: TARGET,
						"⛔️ Signature invalid. `InvalidTransaction` on force clear estimates."
					);
					return InvalidTransaction::BadProof.into();
				}

				ValidTransaction::with_tag_prefix("ares-estimates")
					.priority(T::UnsignedPriority::get())
					.and_provides(symbol)
					.longevity(5)
					.propagate(true)
					.build()

			}
			// else if let Call::submit_unsigned_test { block_number } = call {
			// 	ValidTransaction::with_tag_prefix("for_kami_debug")
			// 		.priority(T::UnsignedPriority::get())
			// 		.and_provides("kami")
			// 		.longevity(5)
			// 		.propagate(true)
			// 		.build()
			// }
			else {
				InvalidTransaction::Call.into()
			}
		}
	}

	#[pallet::storage]
	pub type StorageVersion<T: Config> = StorageValue<_, Releases, OptionQuery>;

	#[pallet::storage]
	pub type LockedEstimates<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	pub type MinimumInitReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	pub type MinimumTicketPrice<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	pub type ActivePallet<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// admin account
	#[pallet::storage]
	#[pallet::getter(fn admins)]
	pub type Admins<T: Config> =
	StorageValue<_, BoundedVecOfAdmins<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	pub type SymbolEstimatesId<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(BoundedVecOfSymbol,EstimatesType), //symbol
		u64,                         //id
	>;

	#[pallet::storage]
	pub type PreparedEstimates<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(BoundedVecOfSymbol,EstimatesType),
		// BoundedVec<u8, StringLimit>,                            // symbol
		SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>, // config
	>;

	#[pallet::storage]
	pub type ActiveEstimates<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(BoundedVecOfSymbol,EstimatesType),                            // symbol
		SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>, // config
	>;

	#[pallet::storage]
	pub type UnresolvedEstimates<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(BoundedVecOfSymbol,EstimatesType),                            // symbol
		SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>, // config
	>;

	#[pallet::storage]
	pub type CompletedEstimates<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(BoundedVecOfSymbol,EstimatesType),  // symbol btc-sudt => [0, 1, 3]
		BoundedVecOfCompletedEstimates<T::BlockNumber, BalanceOf<T>>, // configs
		ValueQuery,
	>;

	#[pallet::storage]
	pub type Participants<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(BoundedVecOfSymbol,EstimatesType),  // symbol
		Blake2_128Concat,
		u64, // id
		BoundedVec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>, MaximumParticipants>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type EstimatesInitDeposit<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(BoundedVecOfSymbol,EstimatesType),
		Blake2_128Concat,
		u64, // id
		BalanceOf<T>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type Winners<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(BoundedVecOfSymbol, EstimatesType), // symbol
		Blake2_128Concat,
		u64, // id
		BoundedVec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>, MaximumWinners>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn next_unsigned_at)]
	pub(super) type NextDataCleanUnsignedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	// #[pallet::storage]
	// #[pallet::getter(fn next_unsigned_at2)]
	// pub(super) type NextUnsignedAt2<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;
}

impl<T: Config> Pallet<T> {

	/// For test
	// fn do_submit_unsigned_test(block_number: T::BlockNumber) -> Result<(), &'static str> {
	// 	let call = Call::submit_unsigned_test { block_number };
	//
	// 	SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
	// 		.map_err(|()| {
	// 			log::error!(
	// 				target: TARGET,
	// 				"offchain worker submit_unsigned_test faild",
	// 			);
	// 		});
	// 	Ok(())
	// }

	fn do_data_cleaning(_block_number: T::BlockNumber) {
		let call = Call::data_cleaning { };
		let _res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|()| {
				log::error!(
					target: TARGET,
					"offchain worker submit_unsigned_test faild",
				);
			});
	}

	pub fn is_active() -> bool {
		let active = ActivePallet::<T>::try_get();
		return if active.is_err() { true } else { active.unwrap() };
	}

	//
	// pub fn account_id(symbol: Vec<u8>) -> Option<T::AccountId> {
	// 	if symbol.len() > 20usize {
	// 		return None;
	// 	}
	// 	let mut u8list: [u8; 20] = [0; 20];
	// 	u8list[..symbol.len()].copy_from_slice(symbol.as_slice());
	// 	T::PalletId::get().try_into_sub_account(u8list)
	// }

	pub fn account_id(sign: (BoundedVecOfSymbol, EstimatesType)) -> Option<T::AccountId> {
		let mut symbol = sign.0;
		let estimat_type: u8 = sign.1.get_type_number();
		symbol.try_push(estimat_type);

		if symbol.len() > 20usize {
			return None;
		}

		let mut u8list: [u8; 20] = [0; 20];
		u8list[..symbol.len()].copy_from_slice(symbol.as_slice());
		T::PalletId::get().try_into_sub_account(u8list)
	}

	fn do_choose_winner(
		winner_payload: ChooseWinnersPayload<T::Public, T::AccountId, T::BlockNumber>,
		estimates_state: EstimatesState,
		// estimate_type: EstimatesType,
	) -> DispatchResult {

		let now = frame_system::Pallet::<T>::block_number();
		let winners = winner_payload.winners;
		let symbol = winner_payload.symbol;
		let id = winner_payload.estimates_id;
		let mut config = None;
		// let storage_key = (symbol,estimate_type);
		let storage_key = symbol;
		match estimates_state {
			EstimatesState::InActive => {}
			EstimatesState::Active => {
				config = ActiveEstimates::<T>::get(&storage_key);
			}
			EstimatesState::WaitingPayout => {}
			EstimatesState::Completed => {}
			EstimatesState::Unresolved => {
				config = UnresolvedEstimates::<T>::get(&storage_key);
			}
		}

		let mut total_reward = BalanceOf::<T>::from(0u32);

		// let price = winner_payload.price.0;
		if config.is_some() {
			// check price
			if let Some((price, _, _)) = winner_payload.price {
				let mut config = config.unwrap();
				let end = config.end;
				let state = config.state.clone();

				if now >= end && state == estimates_state {
					// Get and check subaccount.
					let source_acc = Self::account_id(storage_key.clone());
					ensure!(source_acc.is_some(), Error::<T>::SubAccountGenerateFailed);
					let source_acc = source_acc.unwrap();

					let symbol_account = source_acc.clone();
					for winner in winners.clone() {
						let reward: BalanceOf<T> = (winner.reward).saturated_into();
						total_reward = total_reward + reward;
					}
					ensure!(
							T::Currency::free_balance(&symbol_account) >= total_reward,
							Error::<T>::FreeBalanceTooLow
						);
					// let storage_key = (symbol.clone(), estimate_type.clone());
					// let storage_key = symbol;

					Winners::<T>::insert(&storage_key, id, winners.clone());
					ActiveEstimates::<T>::remove(&storage_key);
					UnresolvedEstimates::<T>::remove(&storage_key);

					let mut winner_events: Vec<(T::AccountId, BalanceOf<T>)> = Vec::new();
					for winner in winners {
						let reward: BalanceOf<T> = (winner.reward).saturated_into();
						T::Currency::transfer(
							&source_acc,
							&winner.account,
							reward.clone(),
							ExistenceRequirement::AllowDeath,
						)?;
						winner_events.push((winner.account, reward));
						// let id = T::PalletId::get().0;
						// T::Currency::reserve_named(&id, &winner.account, reward)?;
					}

					CompletedEstimates::<T>::try_mutate(&storage_key, |configs| {
						config.state = EstimatesState::Completed;
						config.symbol_completed_price = price;
						config.total_reward = total_reward;
						configs.try_push(config.clone()).map_err(|_|Error::<T>::TooMany)
					})?;

					Self::deposit_event(Event::<T>::CompletedEstimates {
						config,
						winners: winner_events,
					});
				}
			}
		}
		Ok(())
	}

	fn do_synchronized_offchain_worker(now: T::BlockNumber) {
		ActiveEstimates::<T>::iter_keys().for_each(|symbol| {
			let config = ActiveEstimates::<T>::get(&symbol);
			let config = config.unwrap();
			let end = config.end;
			let symbol = config.symbol.clone();
			let _deviation = config.deviation;
			let _range = config.range.clone();
			let estimates_type = config.estimates_type.clone();
			let _id = config.id;

			// Get and check subaccount.
			let storage_key = (symbol, estimates_type);
			let source_acc = Self::account_id(storage_key);

			log::info!(
				target: TARGET,
				"!️ source_acc is : {:?} ",
				&source_acc
			);

			if now >= end && source_acc.is_some(){
				// let accounts = Participants::<T>::get(&symbol, id);
				// let price: Result<(u64, FractionLength, T::BlockNumber), ()> = T::PriceProvider::price(&symbol);
				Self::send_signed_triger( &config);
			}else{
				if source_acc.is_none() {
					log::warn!(target: TARGET, "Sub account generation failed");
				}
			}

		});
	}

	pub fn send_signed_triger(
		config: &SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>,
	) {

		log::info!(
			target: TARGET,
			"!️ price-estimates on send_signed_triger : {:?} ",
			&config
		);
		let results = Signer::<T, T::OffchainAppCrypto>::any_account()
			.send_unsigned_transaction(
				|account| ChooseTrigerPayload {
					public: account.public.clone(),
					symbol: (config.symbol.clone(), config.estimates_type.clone()),
				},
				|payload, signature| {
					log::info!(
						target: TARGET,
						"!!!!!!!!!!!!!!!!!️ price-estimates in send_unsigned_transaction sign = {:?},  payload = {:?}",
						&signature,
						&payload
					);
					Call::choose_winner {
						trigger_payload: payload,
						signature
					}
				}
			);

		for (acc, res) in &results {
			match res {
				Ok(()) => log::info!(target: TARGET, "[{:?}]: submit transaction success.", acc.id),
				Err(e) => log::error!(
						target: TARGET,
						"{:?}: submit transaction failure. Reason: {:?}",
						acc.id,
						e
					),
			}
		}
	}

	pub fn call_winners(
		config: &SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>,
		// estimates_type: EstimatesType,
		// _symbol: &Vec<u8>,
		total_reward: BalanceOf<T>,
		deviation: Option<Permill>,
		range: Option<BoundedVec<u64, MaximumOptions>>,
		accounts: BoundedVec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>, MaximumParticipants>,
		price: (u64, FractionLength, T::BlockNumber),
	) -> Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>> {
		let mut count: u32 = 0;
		let estimates_type = config.estimates_type.clone();
		let mut push_winner =
			|winners: &mut Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>>,
			 winner: AccountParticipateEstimates<T::AccountId, T::BlockNumber>| {
				match winner.multiplier {
					MultiplierOption::Base(b) => count += b as u32,
				}
				winners.push(winner);
			};

		let calculate_reward = |winners: &mut Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>>,
								count: u32| {
			let mut avg_reward = BalanceOf::<T>::from(0u32);
			// let i:u32 = 0;

			if count > 0 {
				//avg_reward = total_reward / BalanceOf::<T>::from(count);
				let _avg_reward: Option<BalanceOf::<T>> = total_reward.checked_div(&BalanceOf::<T>::from(count));
				if let Some(_avg_reward) = _avg_reward {
					avg_reward = _avg_reward;
				}
			}

			winners.into_iter().try_for_each(|winner| {
				match winner.multiplier {
					MultiplierOption::Base(b) => {
						let _b = BalanceOf::<T>::from(b);
						//winner.reward = TryInto::<u128>::try_into(avg_reward * _b).ok().unwrap();
						let _reward_opt: Option<u128> = avg_reward.saturating_mul(_b).try_into().ok();
						if let Some(reward) = _reward_opt{
							winner.reward = reward;
						}
					}
				};
				Some(())
			});
		};

		let mut winners: Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>> = Vec::new();
		let price = <ChainPrice as ConvertChainPrice<u64, u32>>::try_to_price(ChainPrice::new((price.0, price.1)), config.symbol_fraction);
		if let Some(price) = price {
			match estimates_type {
				EstimatesType::DEVIATION => {
					// let _base: u64 = 10;
					// let fraction = base.pow(price.1);
					let div_price: u64 = deviation.unwrap_or(Permill::from_percent(0)).mul_ceil(price);
					let low_price: u64 = price.saturating_sub(div_price);
					let high_price: u64 = price.saturating_add(div_price) ;

					let mut check_list: Vec<(T::AccountId, Option<u64>)> = Vec::new();

					for x in accounts {
						if let Some(estimates) = x.estimates {
							check_list.push((x.account.clone(), x.estimates.clone()));
							if low_price <= estimates && estimates <= high_price {
								push_winner(&mut winners, x);
							}
						}
					}



					Self::deposit_event(Event::<T>::ChooseWinner {
						record: ChooseType::DEVIATION {
							low_price,
							high_price,
							check_list
						}
					});
				}
				EstimatesType::RANGE => {
					let range = range.unwrap();

					let mut check_list: Vec<(T::AccountId, Option<u8>)> = Vec::new();

					for x in accounts {
						check_list.push((x.account.clone(), x.range_index.clone()));
						let range_index = usize::from(x.range_index.unwrap());
						if range_index == 0 && price <= range[0] {
							push_winner(&mut winners, x);
						} else if range_index == range.len() && price > range[range.len() - 1] {
							push_winner(&mut winners, x);
						} else if 0 < range_index
							&& range_index < range.len()
							&& range[range_index-1] < price
							&& price <= range[range_index]
						{
							push_winner(&mut winners, x);
						}
					}

					Self::deposit_event(Event::<T>::ChooseWinner {
						record: ChooseType::RANGE {
							range,
							check_list
						}
					});

				}
			};
		}

		//calculate reward
		if winners.len()>0 {
			calculate_reward(&mut winners, count);
		}
		winners
	}

	// pub fn hex_display_estimates_config(estimates_config: &SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>) {
	// 	// use sp_core::hexdisplay::HexDisplay;
	// 	// let hash = Blake2_128Concat::hash(estimates_config.encode().as_slice());
	// 	let encode = estimates_config.encode();
	// 	log::info!(
	// 		target: TARGET,
	// 		"estimates_encode: {:?}",
	// 		HexDisplay::from(&encode) // encode
	// 	);
	// }

}
