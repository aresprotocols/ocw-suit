#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::{
	self as system,
	offchain::{AppCrypto, CreateSignedTransaction, SignedPayload, SigningTypes},
};

use ares_oracle_provider_support::crypto::sr25519::AuthorityId;
use codec::{Decode, Encode};
use core::fmt;
use frame_support::sp_runtime::sp_std::convert::TryInto;
use frame_support::sp_runtime::traits::{IsMember, TrailingZeroInput, Verify};
use frame_support::traits::{FindAuthor, Get, OneSessionHandler};
use frame_system::offchain::{SendUnsignedTransaction, Signer};
use lite_json::json::JsonValue;
use lite_json::NumberValue;
pub use pallet::*;
use serde::{Deserialize, Deserializer};
use sp_application_crypto::sp_core::crypto::UncheckedFrom;
use sp_consensus_aura::AURA_ENGINE_ID;
use sp_core::crypto::KeyTypeId;
use sp_core::sr25519::Signature as Sr25519Signature;
use sp_runtime::offchain::storage::StorageValueRef;
use sp_runtime::{
	offchain::{http, Duration},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	AccountId32, RuntimeAppPublic, RuntimeDebug,
};
use sp_std::vec::Vec;
use sp_std::{prelude::*, str};

use crate::traits::*;
use ares_oracle_provider_support::{
	IAresOraclePreCheck, JsonNumberValue, PreCheckList, PreCheckStatus, PreCheckStruct, PreCheckTaskConfig,
};
use frame_support::pallet_prelude::{PhantomData, StorageMap};
use frame_support::sp_runtime::{MultiSignature, MultiSigner, Percent};
use frame_support::storage::bounded_btree_map::BoundedBTreeMap;
use frame_support::weights::Weight;
use frame_support::{BoundedVec, ConsensusEngineId};
use hex;
use oracle_finance::traits::{IForPrice, IForReporter, IForBase};
use oracle_finance::types::{BalanceOf, PurchaseId};
use sp_runtime::app_crypto::Public;
use sp_runtime::traits::{Saturating, UniqueSaturatedInto};
use sp_std::collections::btree_map::BTreeMap;
use types::*;

pub mod crypto2;
pub mod traits;
pub mod types;

pub mod aura_handler;
pub mod babe_handler;
pub mod migrations;

pub const LOCAL_STORAGE_PRICE_REQUEST_MAKE_POOL: &[u8] = b"are-ocw::make_price_request_pool";
pub const LOCAL_STORAGE_PRICE_REQUEST_LIST: &[u8] = b"are-ocw::price_request_list";
pub const LOCAL_STORAGE_PRICE_REQUEST_DOMAIN: &[u8] = b"are-ocw::price_request_domain";
pub const LOCAL_HOST_KEY: &[u8] = b"are-ocw::local_host_key";
pub const CALCULATION_KIND_AVERAGE: u8 = 1;
pub const CALCULATION_KIND_MEDIAN: u8 = 2;

pub const PURCHASED_FINAL_TYPE_IS_THRESHOLD_UP: u8 = 1;
pub const PURCHASED_FINAL_TYPE_IS_FORCE_CLEAN: u8 = 2;

#[cfg(test)]
mod tests;

pub struct AresCrypto<AresPublic>(PhantomData<AresPublic>);

impl<AresPublic: RuntimeAppPublic> frame_system::offchain::AppCrypto<MultiSigner, MultiSignature>
	for AresCrypto<AresPublic>
where
	sp_application_crypto::sr25519::Signature: From<<AresPublic as sp_runtime::RuntimeAppPublic>::Signature>,
	<AresPublic as sp_runtime::RuntimeAppPublic>::Signature: From<sp_application_crypto::sr25519::Signature>,
	sp_application_crypto::sr25519::Public: From<AresPublic>,
	AresPublic: From<sp_application_crypto::sr25519::Public>,
{
	type RuntimeAppPublic = AresPublic;
	type GenericSignature = Sr25519Signature;
	type GenericPublic = sp_core::sr25519::Public;
}

impl<AresPublic: RuntimeAppPublic>
	frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature> for AresCrypto<AresPublic>
where
	sp_application_crypto::sr25519::Signature: From<<AresPublic as sp_runtime::RuntimeAppPublic>::Signature>,
	<AresPublic as sp_runtime::RuntimeAppPublic>::Signature: From<sp_application_crypto::sr25519::Signature>,
	sp_application_crypto::sr25519::Public: From<AresPublic>,
	AresPublic: From<sp_application_crypto::sr25519::Public>,
{
	type RuntimeAppPublic = AresPublic;
	type GenericSignature = Sr25519Signature;
	type GenericPublic = sp_core::sr25519::Public;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use ares_oracle_provider_support::{IAresOraclePreCheck, PreCheckTaskConfig, TokenList};
	use frame_support::pallet_prelude::*;
	use frame_support::sp_runtime::traits::{IdentifyAccount, IsMember};
	use frame_system::offchain::SubmitTransaction;
	use frame_system::pallet_prelude::{BlockNumberFor, OriginFor};
	use frame_system::{ensure_none, ensure_signed};
	use oracle_finance::traits::*;
	use oracle_finance::types::{BalanceOf, OcwPaymentResult, PurchaseId, EraIndex};
	use sp_consensus_aura::ConsensusLog::AuthoritiesChange;
	use sp_core::crypto::UncheckedFrom;
	use staking_extend::IStakingNpos;

	#[pallet::error]
	#[derive(PartialEq, Eq)]
	pub enum Error<T> {
		SubmitThresholdRangeError,
		DruationNumberNotBeZero,
		UnknownAresPriceVersionNum,
		InsufficientBalance,
		InsufficientMaxFee,
		PayToPurchaseFeeFailed,
		PerCheckTaskAlreadyExists,
		PreCheckTokenListNotEmpty,
	}

	/// This pallet's configuration trait
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config + oracle_finance::Config {
		/// The identifier type for an offchain worker.
		type OffchainAppCrypto: AppCrypto<Self::Public, Self::Signature>;

		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The overarching dispatch call type.
		type Call: From<Call<Self>>;

		/// ocw store key pair.
		type AuthorityAres: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Default
			+ Ord
			+ MaybeSerializeDeserialize
			+ UncheckedFrom<[u8; 32]>
			+ From<AuthorityId>;

		type FindAuthor: FindAuthor<u32>;

		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;

		#[pallet::constant]
		type CalculationKind: Get<u8>;

		#[pallet::constant]
		type ErrLogPoolDepth: Get<u32>;

		type RequestOrigin: EnsureOrigin<Self::Origin>;

		type AuthorityCount: ValidatorCount;

		type OracleFinanceHandler: IForPrice<Self> + IForReporter<Self> + IForReward<Self>;

		type AresIStakingNpos: IStakingNpos<
			Self::AuthorityAres,
			Self::BlockNumber,
			StashId = <Self as frame_system::Config>::AccountId,
		>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	// #[pallet::generate_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
		<T as frame_system::Config>::AccountId: From<sp_application_crypto::sr25519::Public>,
	{
		fn on_initialize(n: T::BlockNumber) -> Weight {
			Self::check_and_clean_obsolete_task(14400u32.into())
		}

		fn on_runtime_upgrade() -> frame_support::weights::Weight {
			// To runtime v107
			if StorageVersion::<T>::get() == Releases::V1_0_0_Ancestral
				|| StorageVersion::<T>::get() == Releases::V1_0_1_HttpErrUpgrade
				|| StorageVersion::<T>::get() == Releases::V1_1_0_HttpErrUpgrade
			{
				return migrations::v1_2_0::migrate::<T>();
			}
			T::DbWeight::get().reads(1)
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			let control_setting = <OcwControlSetting<T>>::get();
			let block_author = Self::get_block_author();
			// For debug on test-chain.
			let conf_session_multi = ConfPreCheckSessionMulti::<T>::get();

			if T::AresIStakingNpos::near_era_change(conf_session_multi) {
				log::debug!(
					"**** near_era_change => conf_session_multi = {:?}",
					&conf_session_multi,
				);
				// Get all ares authoritys.
				let authority_list = T::AuthorityAres::all();
				log::debug!(
					"**** near_era_change => authority_list = {:?}",
					&authority_list,
				);
				// Check not be validator.
				let current_validator: Vec<(_, T::AuthorityAres)> = Authorities::<T>::get();
				log::debug!(
					"**** near_era_change => current_validator = {:?}",
					&current_validator,
				);
				let online_authroitys = current_validator
					.into_iter()
					.map(|(_, auth)| auth)
					.collect::<Vec<T::AuthorityAres>>();
				log::debug!(
					"**** near_era_change => online_authroitys = {:?}",
					&online_authroitys,
				);

				let in_list = authority_list
					.iter()
					.any(|local_authority| online_authroitys.contains(local_authority));
				log::debug!(
					"**** near_era_change => in_list = {:?}",
					&in_list,
				);

				// submit offchain tx.
				if !in_list {
					let host_key = Self::get_local_host_key();
					// Get request_domain.
					let request_domain = Self::get_local_storage_request_domain();
					log::debug!(
						"Host_key = {:?}, request_domain = {:?}, authority_list = {:?}",
						host_key,
						request_domain,
						authority_list
					);
					// LocalXRay::<T>::put(host_key, (request_domain, authority_list));
					let call = Call::submit_local_xray {
						host_key,
						request_domain,
						authority_list,
					};
					SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
				}
			}

			match block_author {
				None => {
					log::warn!(target: "ares_oracle::offchain_worker", "❗ Not found author.");
				}
				Some(author) => {
					if control_setting.open_free_price_reporter {
						log::info!("🚅 ❗ ⛔ Ocw offchain start {:?} ", &author);
						if Self::are_block_author_and_sotre_key_the_same(&author) {
							log::debug!("🚅 @ Ares call [1] ares-price-worker.");
							// Try to get ares price.
							match Self::ares_price_worker(block_number.clone(), author.clone()) {
								Ok(_v) => log::debug!("🚅 @ Ares OCW price acquisition completed."),
								Err(e) => log::warn!(
									target: "pallet::ocw::offchain_worker",
									"❗ Ares price has a problem : {:?}",
									e
								),
							}

							let conf_session_multi = ConfPreCheckSessionMulti::<T>::get();
							// Do you need to scan pre-validator data
							if T::AresIStakingNpos::near_era_change(conf_session_multi) {
								log::debug!(" T::AresIStakingNpos::near_era_change is near will get npos data.");
								let pending_npos = T::AresIStakingNpos::pending_npos();
								log::debug!(" ******************* pending_npos: {:?}", pending_npos.clone());
								for (stash_id, auth_id) in pending_npos {
									log::debug!(
										"T::AresIStakingNpos::new validator, stash_id = {:?}, auth_id = {:?}",
										stash_id.clone(),
										&auth_id
									);
									if auth_id.is_none() {
										log::warn!(
											target: "T::AresIStakingNpos",
											"❗ Staking authority is not set, you can use RPC author_insertKey fill that.",
										)
									}
									// let ares_auth_id = Self::get_auth_id(&stash_id);
									log::debug!(" ** Get ares_auth_id = {:?}", &auth_id);
									if !Self::has_pre_check_task(stash_id.clone()) && auth_id.is_some() {
										log::debug!(" ** has_pre_check_task = author = {:?}", &author);
										// Use PreCheckPayload send transaction.
										match Self::save_create_pre_check_task(
											author.clone(),
											stash_id,
											auth_id.unwrap(),
											block_number,
										) {
											Ok(_) => {}
											Err(e) => {
												log::warn!(
													target: "save_create_pre_check_task",
													"❗ An error occurred while creating the pre-checked task {:?}",
													e
												)
											}
										}
									}
								}
							}
						}
					}

					if control_setting.open_paid_price_reporter {
						if let Some(keystore_validator) = Self::keystore_validator_member() {
							log::debug!("🚅 @ Ares call [2] ares-purchased-checker.");
							match Self::ares_purchased_checker(block_number.clone(), keystore_validator) {
								Ok(_v) => log::debug!("🚅 % Ares OCW purchased checker completed."),
								Err(e) => log::warn!(
									target: "pallet::ocw::offchain_worker",
									"❗ Ares purchased price has a problem : {:?}",
									e
								),
							}
						}
					}
				}
			}

			if control_setting.open_paid_price_reporter {
				if let Some(keystore_validator) = Self::keystore_validator_member() {
					log::debug!("🚅 @ Ares call [3] ares-purchased-worker.");
					match Self::ares_purchased_worker(block_number.clone(), keystore_validator) {
						Ok(v) => log::debug!("🚅 ~ Ares OCW purchased price acquisition completed."),
						Err(e) => log::warn!(
							target: "pallet::ocw::offchain_worker",
							"❗ Ares purchased price has a problem : {:?}",
							e
						),
					}
				}
			}

			//
			if let Some((stash, auth, task_at)) = Self::get_pre_task_by_authority_set(Self::get_ares_authority_list()) {
				// Self::create_pre_check_task(stash_id.clone(), block_number);
				log::debug!(
					"Have my own pre-check task. stash = {:?}, auth = {:?}",
					stash.clone(),
					&auth
				);
				// Get pre-check token list.
				let token_list = ConfPreCheckTokenList::<T>::get();
				// Get per-check allowable offset Percent.
				let allowable_offset = ConfPreCheckAllowableOffset::<T>::get();
				// Make check-config struct.
				let check_config = PreCheckTaskConfig {
					check_token_list: token_list,
					allowable_offset: allowable_offset,
				};
				// get check result
				let take_price_list = Self::take_price_for_pre_check(check_config);
				// Sending transaction to chain. Use PreCheckResultPayload
				match Self::save_offchain_pre_check_result(
					stash,
					auth,
					block_number,
					take_price_list,
					task_at
				) {
					Ok(_) => {}
					Err(e) => {
						log::warn!(
							target: "save_offchain_pre_check_result",
							"❗ Pre-check data reception abnormal : {:?}",
							e
						)
					}
				}
			}
		}
	}

	/// A public part of the pallet.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(0)]
		pub fn submit_local_xray(
			origin: OriginFor<T>,
			host_key: u32,
			request_domain: Vec<u8>,
			authority_list: Vec<T::AuthorityAres>,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			LocalXRay::<T>::insert(
				host_key,
				(<system::Pallet<T>>::block_number(), request_domain, authority_list),
			);
			Ok(().into())
		}

		#[pallet::weight(1000)]
		pub fn submit_ask_price(
			origin: OriginFor<T>,
			#[pallet::compact] max_fee: BalanceOf<T>,
			request_keys: Vec<u8>,
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let purchased_default = <PurchasedDefaultSetting<T>>::get();
			let submit_threshold = purchased_default.submit_threshold;
			let max_duration = purchased_default.max_duration;
			let request_keys = Self::extract_purchased_request(request_keys);

			let purchase_id = Self::make_purchase_price_id(who.clone(), 0);
			let purchase_id: PurchaseId = purchase_id.clone().try_into().expect("id is too long");

			let raw_source_keys = Self::filter_raw_price_source_list(request_keys.clone());

			// Filter out unsupported request pairs.
			let request_keys = raw_source_keys.into_iter().map(|(price_key, _, _, _)|{
				price_key
			}).collect::<RequestKeys>();

			let offer = T::OracleFinanceHandler::calculate_fee_of_ask_quantity(request_keys.len() as u32);
			if offer > max_fee {
				return Err(Error::<T>::InsufficientMaxFee.into());
			}

			let payment_result: OcwPaymentResult<BalanceOf<T>> = T::OracleFinanceHandler::reserve_for_ask_quantity(
				who.clone(),
				purchase_id.clone(),
				request_keys.len() as u32,
			);

			match payment_result {
				OcwPaymentResult::InsufficientBalance(_, _balance) => {
					return Err(Error::<T>::InsufficientBalance.into());
				}
				OcwPaymentResult::Success(_, balance) => {
					// Up request on chain.
					let res = Self::ask_price(who, balance, submit_threshold, max_duration, purchase_id, request_keys)?;
				}
			}
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn submit_forced_clear_purchased_price_payload_signed(
			origin: OriginFor<T>,
			price_payload: PurchasedForceCleanPayload<T::Public, T::BlockNumber, T::AuthorityAres>,
			_signature: OffchainSignature<T>,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			let purchase_id_list: Vec<PurchaseId> = price_payload.purchase_id_list;
			purchase_id_list.iter().any(|x| {
				// check that the validator threshold is up to standard.
				if Self::is_validator_purchased_threshold_up_on(x.clone()) {
					// Calculate the average price
					let agg_count = Self::update_purchase_avg_price_storage(x.clone(), PURCHASED_FINAL_TYPE_IS_FORCE_CLEAN);
					// update report work point
					Self::update_reporter_point(x.clone(), agg_count);
					Self::purchased_storage_clean(x.clone());
				} else {
					// Get `ask owner`
					let refund_result = T::OracleFinanceHandler::unreserve_ask(x.clone());
					if refund_result.is_ok() {
						Self::purchased_storage_clean(x.clone());
						Self::deposit_event(Event::InsufficientCountOfValidators {
							purchase_id: x.clone()
						});
					} else {
						log::error!(
							target: "pallet::ocw::submit_forced_clear_purchased_price_payload_signed",
							"⛔️ T::OracleFinanceHandler::unreserve_ask() had an error!"
						);
						Self::deposit_event(Event::ProblemWithRefund);
					}
				}
				false
			});

			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn submit_purchased_price_unsigned_with_signed_payload(
			origin: OriginFor<T>,
			price_payload: PurchasedPricePayload<T::Public, T::BlockNumber, T::AuthorityAres>,
			_signature: OffchainSignature<T>,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			log::info!(
				"🚅 Pre submit purchased price payload on block {:?}",
				<system::Pallet<T>>::block_number()
			);
			// check whether purchased request still exists
			if false == <PurchasedRequestPool<T>>::contains_key(price_payload.purchase_id.clone()) {
				Self::deposit_event(Event::PurchasedRequestWorkHasEnded {
					purchase_id: price_payload.purchase_id.clone(),
					who: Self::get_stash_id(&price_payload.auth.clone()).unwrap(),
					finance_era: T::OracleFinanceHandler::current_era_num(),
				});
				return Ok(().into());
			}

			Self::add_purchased_price(
				price_payload.purchase_id.clone(),
				Self::get_stash_id(&price_payload.auth.clone()).unwrap(),
				price_payload.block_number.clone(),
				price_payload.price,
			);
			// check
			if Self::is_all_validator_submitted_price(price_payload.purchase_id.clone()) {
				// Calculate the average price
				let agg_count = Self::update_purchase_avg_price_storage(
					price_payload.purchase_id.clone(),
					PURCHASED_FINAL_TYPE_IS_THRESHOLD_UP,
				);
				// update report work point
				Self::update_reporter_point(price_payload.purchase_id.clone(), agg_count);
				// clean order pool
				Self::purchased_storage_clean(price_payload.purchase_id.clone());
			}
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn submit_price_unsigned_with_signed_payload(
			origin: OriginFor<T>,
			price_payload: PricePayload<T::Public, T::BlockNumber, T::AuthorityAres>,
			_signature: OffchainSignature<T>,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			log::debug!(
				"🚅 Pre submit price payload on block {:?}",
				<system::Pallet<T>>::block_number()
			);
			// Nodes with the right to increase prices
			let price_list = price_payload.price; // price_list: Vec<(PriceKey, u32)>,
			let mut price_key_list = Vec::new();
			let mut agg_result_list: Vec<(PriceKey, u64, FractionLength, Vec<T::AccountId>)> = Vec::new();
			for PricePayloadSubPrice(price_key, price, fraction_length, json_number_value, timestamp) in
				price_list.clone()
			{
				// Add the price to the on-chain list, but mark it as coming from an empty address.
				if let Some(agg_result) = Self::add_price_and_try_to_agg(
					Self::get_stash_id(&price_payload.auth).unwrap(),
					price.clone(),
					price_key.clone(),
					fraction_length,
					json_number_value,
					Self::get_price_pool_depth(),
					timestamp,
				) {
					agg_result_list.push(agg_result);
				}
				price_key_list.push(price_key.clone());
				// event_result.push((price_key, price, fraction_length));
			}

			// Update agg event.
			if !agg_result_list.is_empty() {
				Self::deposit_event(Event::AggregatedPrice {
					results: agg_result_list,
				});
			}

			log::debug!("🚅 Submit price list on chain, count = {:?}", price_key_list.len());

			// update last author
			Self::update_last_price_list_for_author (
				price_key_list,
				Self::get_stash_id(&price_payload.auth).unwrap(),
			);

			// Set jump block
			let jump_block = price_payload.jump_block;
			if jump_block.len() > 0 {
				for PricePayloadSubJumpBlock(price_key, interval) in jump_block.clone() {
					Self::increase_jump_block_number(price_key, interval as u64);
				}
			}

			log::debug!("🚅 Submit jump block list on chain, count = {:?}", jump_block.len());

			// Self::deposit_event(Event::NewPrice {
			// 	results: event_result,
			// 	jump_blocks: jump_block,
			// 	who: Self::get_stash_id(&price_payload.auth).unwrap(),
			// });

			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn submit_create_pre_check_task(
			origin: OriginFor<T>,
			precheck_payload: PreCheckPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>,
			_signature: OffchainSignature<T>,
		) -> DispatchResult {
			ensure_none(origin)?;
			let result = Self::create_pre_check_task(
				precheck_payload.pre_check_stash.clone(),
				precheck_payload.pre_check_auth.clone(),
				precheck_payload.block_number,
			);
			ensure!(result, Error::<T>::PerCheckTaskAlreadyExists);
			Self::deposit_event(Event::NewPreCheckTask {
				who: precheck_payload.pre_check_stash,
				authority: precheck_payload.pre_check_auth,
				block: precheck_payload.block_number,
			});
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn submit_offchain_pre_check_result(
			origin: OriginFor<T>,
			preresult_payload: PreCheckResultPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>,
			_signature: OffchainSignature<T>,
		) -> DispatchResult {
			ensure_none(origin)?;
			ensure!(
				preresult_payload.pre_check_list.len() > 0,
				Error::<T>::PreCheckTokenListNotEmpty
			);
			let check_result = Self::save_pre_check_result(
				preresult_payload.pre_check_stash.clone(),
				preresult_payload.block_number,
				preresult_payload.pre_check_list.clone(),
			);
			Self::deposit_event(Event::NewPreCheckResult {
				who: preresult_payload.pre_check_stash,
				created_at: preresult_payload.block_number,
				pre_check_list: preresult_payload.pre_check_list,
				task_at: preresult_payload.task_at,
				check_result
			});
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn submit_offchain_http_err_trace_result(
			origin: OriginFor<T>,
			err_payload: HttpErrTracePayload<T::Public, T::BlockNumber, T::AuthorityAres, T::AccountId>,
			_signature: OffchainSignature<T>,
		) -> DispatchResult {
			ensure_none(origin)?;
			let mut http_err = <HttpErrTraceLogV1<T>>::get();
			if http_err.len() as u32 > T::ErrLogPoolDepth::get() {
				http_err.remove(0usize);
			}
			http_err.push(err_payload.trace_data);
			<HttpErrTraceLogV1<T>>::put(http_err);
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn update_purchased_param(
			origin: OriginFor<T>,
			submit_threshold: u8,
			max_duration: u64,
			avg_keep_duration: u64,
		) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			ensure!(
				submit_threshold > 0 && submit_threshold <= 100,
				Error::<T>::SubmitThresholdRangeError
			);
			ensure!(max_duration > 0, Error::<T>::DruationNumberNotBeZero);
			let setting_data = PurchasedDefaultData::new(submit_threshold, max_duration, avg_keep_duration);
			<PurchasedDefaultSetting<T>>::put(setting_data.clone());
			Self::deposit_event(Event::UpdatePurchasedDefaultSetting { setting: setting_data });
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn update_ocw_control_setting(
			origin: OriginFor<T>,
			need_verifier_check: bool,
			open_free_price_reporter: bool,
			open_paid_price_reporter: bool,
		) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			let setting_data = OcwControlData {
				need_verifier_check,
				open_free_price_reporter,
				open_paid_price_reporter,
			};
			<OcwControlSetting<T>>::put(setting_data.clone());
			Self::deposit_event(Event::UpdateOcwControlSetting { setting: setting_data });
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn revoke_update_request_propose(origin: OriginFor<T>, price_key: Vec<u8>) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			let price_key: PriceKey = price_key.clone().try_into().expect("price_key is too long");
			<PricesRequests<T>>::mutate(|prices_request| {
				for (index, (old_price_key, _, _, _, _)) in prices_request.clone().into_iter().enumerate() {
					if &price_key == &old_price_key {
						// remove old one
						prices_request.remove(index);
						Self::clear_price_storage_data(price_key.clone());
						break;
					}
				}
			});
			Self::remove_jump_block_number(price_key.clone());
			Self::deposit_event(Event::RevokePriceRequest { price_key });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn update_request_propose(
			origin: OriginFor<T>,
			price_key: Vec<u8>,
			price_token: Vec<u8>,
			parse_version: u32,
			fraction_num: FractionLength,
			request_interval: RequestInterval,
		) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			let price_key: PriceKey = price_key.clone().try_into().expect("price_key is too long");
			let price_token: PriceToken = price_token.clone().try_into().expect("price_token is too long");
			<PricesRequests<T>>::mutate(|prices_request| {
				let mut find_old = false;
				for (index, (old_price_key, _old_price_token, _old_parse_version, old_fraction_count, _)) in
					prices_request.clone().into_iter().enumerate()
				{
					if &price_key == &old_price_key {
						let price_token_tmp = price_token.to_vec();
						if &"".as_bytes().to_vec() != &price_token_tmp {
							// add input value
							prices_request.push((
								price_key.clone(),
								price_token.clone(),
								parse_version,
								fraction_num.clone(),
								request_interval.clone(),
							));
							Self::deposit_event(Event::UpdatePriceRequest {
								price_key: price_key.clone(),
								price_token: price_token.clone(),
								parse_version: parse_version.clone(),
								fraction: fraction_num.clone(),
							});
						}
						// remove old one
						prices_request.remove(index);
						// check fraction number on change
						if &old_fraction_count != &fraction_num || &"".as_bytes().to_vec() == &price_token_tmp {
							if <AresPrice<T>>::contains_key(&price_key) {
								// if exists will be empty
								<AresPrice<T>>::remove(&price_key);
							}
							// remove avg
							if <AresAvgPrice<T>>::contains_key(&price_key) {
								// if exists will be empty
								<AresAvgPrice<T>>::remove(&price_key);
							}
						}
						// then break for.
						find_old = true;
						break;
					}
				}
				if !find_old {
					prices_request.push((
						price_key.clone(),
						price_token.clone(),
						parse_version.clone(),
						fraction_num.clone(),
						request_interval.clone(),
					));
					Self::deposit_event(Event::AddPriceRequest {
						price_key,
						price_token,
						parse_version,
						fraction: fraction_num,
					});
				}
			});
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn update_allowable_offset_propose(origin: OriginFor<T>, offset: u8) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			<PriceAllowableOffset<T>>::put(offset);
			Self::deposit_event(Event::PriceAllowableOffsetUpdate { offset });
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn update_pool_depth_propose(origin: OriginFor<T>, depth: u32) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			// Judge the value must be greater than 0 and less than the maximum of U32
			assert!(depth > 0 && depth < u32::MAX, "⛔ Depth wrong value range.");
			// get old depth number
			let old_depth = <PricePoolDepth<T>>::get();
			assert_ne!(old_depth, depth, "⛔ Depth of change cannot be the same.");
			<PricePoolDepth<T>>::set(depth.clone());
			Self::deposit_event(Event::PricePoolDepthUpdate { depth });
			if depth < old_depth {
				<PricesRequests<T>>::get().into_iter().any(|(price_key, _, _, _, _)| {
					// clear average.
					let old_price_list = <AresPrice<T>>::get(price_key.clone());
					let depth_usize = depth as usize;
					// check length
					if old_price_list.len() > depth_usize {
						let diff_len = old_price_list.len() - depth_usize;
						let mut new_price_list = Vec::new();
						for (index, value) in old_price_list.into_iter().enumerate() {
							if !(index < diff_len) {
								// kick out old value.
								new_price_list.push(value);
							}
						}
						// Reduce the depth.
						<AresPrice<T>>::insert(price_key.clone(), new_price_list);
					}
					// need to recalculate the average.
					Self::check_and_update_avg_price_storage(price_key, depth);
					false
				});
			}
			Ok(())
		}

		#[pallet::weight(0)]
		pub fn update_pre_check_token_list(origin: OriginFor<T>, token_list: TokenList) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			ensure!(token_list.len() > 0, Error::<T>::PreCheckTokenListNotEmpty);
			<ConfPreCheckTokenList<T>>::put(token_list);
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn update_pre_check_session_multi(origin: OriginFor<T>, multi: T::BlockNumber) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			<ConfPreCheckSessionMulti<T>>::put(multi);
			Ok(().into())
		}

		#[pallet::weight(0)]
		pub fn update_pre_check_allowable_offset(origin: OriginFor<T>, offset: Percent) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			<ConfPreCheckAllowableOffset<T>>::put(offset);
			Ok(().into())
		}
	}

	/// Events for the pallet.
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		// (price_key, price_val, fraction len)
		AggregatedPrice {
			results: Vec<(PriceKey, u64, FractionLength, Vec<T::AccountId>)>,
		},
		// NewPrice {
		// 	results: Vec<(PriceKey, u64, FractionLength)>,
		// 	jump_blocks: PricePayloadSubJumpBlockList,
		// 	who: T::AccountId,
		// },
		// AbnormalPrice(PriceKey, AresPriceData<T::AccountId, T::BlockNumber>),
		// The report request was closed when the price was submitted
		PurchasedRequestWorkHasEnded {
			purchase_id: PurchaseId,
			who: T::AccountId,
			finance_era: EraIndex,
		},
		NewPurchasedPrice {
			created_at: T::BlockNumber,
			price_list: PricePayloadSubPriceList,
			who: T::AccountId,
			finance_era: EraIndex,
			purchase_id: PurchaseId,
		},
		NewPurchasedRequest {
			purchase_id: PurchaseId,
			request_data: PurchasedRequestData<T::AccountId, BalanceOf<T>, T::BlockNumber>,
			value: BalanceOf<T>,
			finance_era: EraIndex,
		},
		PurchasedAvgPrice {
			purchase_id: PurchaseId,
			event_results: Vec<Option<(PriceKey, PurchasedAvgPriceData, Vec<T::AccountId>)>>,
			finance_era: EraIndex,
		},
		UpdatePurchasedDefaultSetting {
			setting: PurchasedDefaultData,
		},
		UpdateOcwControlSetting {
			setting: OcwControlData,
		},
		RevokePriceRequest {
			price_key: PriceKey,
		},
		AddPriceRequest {
			price_key: PriceKey,
			price_token: PriceToken,
			parse_version: u32,
			fraction: FractionLength,
		},
		UpdatePriceRequest {
			price_key: PriceKey,
			price_token: PriceToken,
			parse_version: u32,
			fraction: FractionLength,
		},
		PricePoolDepthUpdate {
			depth: u32,
		},
		PriceAllowableOffsetUpdate {
			offset: u8,
		},
		InsufficientCountOfValidators {
			purchase_id: PurchaseId,
		} ,
		ProblemWithRefund,
		NewPreCheckTask {
			who: T::AccountId,
			authority: T::AuthorityAres,
			block: T::BlockNumber,
		},
		NewPreCheckResult {
			who: T::AccountId,
			created_at: T::BlockNumber,
			pre_check_list: PreCheckList,
			task_at: T::BlockNumber,
			check_result: PreCheckStatus
		},
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit_price_unsigned_with_signed_payload {
				price_payload: ref payload,
				signature: ref signature,
			} = call
			{
				log::debug!(
					"🚅 Validate price payload data, on block: {:?}/{:?}, author: {:?} ",
					payload.block_number.clone(),
					<system::Pallet<T>>::block_number(),
					payload.public.clone()
				);

				if !Self::is_validator_member(&payload.auth) {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on price "
					);
					return InvalidTransaction::BadProof.into();
				}

				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
				if !signature_valid {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Signature invalid. `InvalidTransaction` on price"
					);
					return InvalidTransaction::BadProof.into();
				}

				Self::validate_transaction_parameters_of_ares(&payload.block_number, payload.price.to_vec())
			} else if let Call::submit_purchased_price_unsigned_with_signed_payload {
				price_payload: ref payload,
				ref signature,
			} = call
			{
				log::debug!(
					"🚅 Validate purchased price payload data, on block: {:?} ",
					<system::Pallet<T>>::block_number()
				);

				if !Self::is_validator_member(&payload.auth) {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on purchased price"
					);
					return InvalidTransaction::BadProof.into();
				}

				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());

				if !signature_valid {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Signature invalid. `InvalidTransaction` on purchased price"
					);
					return InvalidTransaction::BadProof.into();
				}

				let priority_num: u64 = T::UnsignedPriority::get();

				ValidTransaction::with_tag_prefix("ares-oracle::validate_transaction_parameters_of_purchased_price")
					.priority(priority_num.saturating_add(1))
					.and_provides(payload.public.clone())
					.longevity(5)
					.propagate(true)
					.build()
			} else if let Call::submit_forced_clear_purchased_price_payload_signed {
				price_payload: ref payload,
				ref signature,
			} = call
			{
				// submit_forced_clear_purchased_price_payload_signed
				log::debug!(
					"🚅 Validate forced clear purchased price payload data, on block: {:?} ",
					<system::Pallet<T>>::block_number()
				);
				if !Self::is_validator_member(&payload.auth) {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on force clear purchased"
					);
					return InvalidTransaction::BadProof.into();
				}
				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
				if !signature_valid {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Signature invalid. `InvalidTransaction` on force clear purchased"
					);
					return InvalidTransaction::BadProof.into();
				}

				let priority_num: u64 = T::UnsignedPriority::get();
				ValidTransaction::with_tag_prefix(
					"ares-oracle::validate_transaction_parameters_of_force_clear_purchased",
				)
				.priority(priority_num.saturating_add(2))
				.and_provides(&payload.block_number) // next_unsigned_at
				.longevity(5)
				.propagate(true)
				.build()
			} else if let Call::submit_create_pre_check_task {
				precheck_payload: ref payload,
				ref signature,
			} = call
			{
				log::debug!(
					"🚅 Validate submit_create_pre_check_task, on block: {:?}/{:?}, author: {:?} ",
					payload.block_number.clone(),
					<system::Pallet<T>>::block_number(),
					payload.public.clone()
				);
				if !Self::is_validator_member(&payload.auth) {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on price "
					);
					return InvalidTransaction::BadProof.into();
				}
				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
				if !signature_valid {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Signature invalid. `InvalidTransaction` on price"
					);
					return InvalidTransaction::BadProof.into();
				}

				ValidTransaction::with_tag_prefix("ares-oracle::submit_create_pre_check_task")
					.priority(T::UnsignedPriority::get())
					.and_provides(payload.pre_check_stash.clone()) // next_unsigned_at
					.longevity(5)
					.propagate(true)
					.build()
			} else if let Call::submit_offchain_http_err_trace_result {
				err_payload: ref payload,
				ref signature,
			} = call
			{
				// submit_offchain_http_err_trace_result
				log::debug!(
					"🚅 Validate purchased price payload data, on block: {:?} ",
					<system::Pallet<T>>::block_number()
				);

				if !Self::is_validator_member(&payload.auth) {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on purchased price"
					);
					return InvalidTransaction::BadProof.into();
				}

				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());

				if !signature_valid {
					log::error!(
						target: "pallet::ocw::validate_unsigned",
						"⛔️ Signature invalid. `InvalidTransaction` on purchased price"
					);
					return InvalidTransaction::BadProof.into();
				}

				let priority_num: u64 = T::UnsignedPriority::get();

				ValidTransaction::with_tag_prefix("ares-oracle::submit_offchain_http_err_trace_result")
					.priority(priority_num.saturating_add(1))
					.and_provides(payload.public.clone())
					.longevity(5)
					.propagate(true)
					.build()
			} else if let Call::submit_offchain_pre_check_result {
				preresult_payload: ref payload,
				ref signature,
			} = call
			{
				log::debug!(
                    "🚅 Validate submit_offchain_pre_check_result, on block: {:?}/{:?}, stash: {:?}, auth: {:?}, pub: {:?}, pre_check_list: {:?}",
                    payload.block_number.clone(),
                    <system::Pallet<T>>::block_number(),
                    payload.pre_check_stash.clone(),
                    payload.pre_check_auth.clone(),
                    payload.public.clone(),
                    payload.pre_check_list.clone(),
                );
				if <system::Pallet<T>>::block_number() - payload.block_number.clone() > 5u32.into() {
					return InvalidTransaction::BadProof.into();
				}
				// check stash status
				let mut auth_list = Vec::new();
				auth_list.push(payload.pre_check_auth.clone());
				if let Some((s_stash, _, _)) = Self::get_pre_task_by_authority_set(auth_list) {
					if &s_stash != &payload.pre_check_stash {
						log::error!(
							target: "pallet::ocw::validate_unsigned - submit_offchain_pre_check_result",
							"⛔️ Stash account is inconsistent!"
						);
						return InvalidTransaction::BadProof.into();
					}
				} else {
					log::error!(
						target: "pallet::ocw::validate_unsigned - submit_offchain_pre_check_result",
						"⛔️ Could not find the pre-check task!"
					);
					return InvalidTransaction::BadProof.into();
				}
				//
				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
				if !signature_valid {
					log::error!(
						target: "pallet::ocw::validate_unsigned - submit_offchain_pre_check_result",
						"⛔️ Signature invalid. `InvalidTransaction` on price"
					);
					return InvalidTransaction::BadProof.into();
				}
				ValidTransaction::with_tag_prefix("ares-oracle::submit_offchain_pre_check_result")
					.priority(T::UnsignedPriority::get())
					.and_provides(&payload.block_number) // next_unsigned_at
					.longevity(5)
					.propagate(true)
					.build()
			} else if let Call::submit_local_xray {
				ref host_key,
				ref request_domain,
				ref authority_list,
			} = call
			{
				log::debug!("*** XRay ValidTransaction: {:?} ", <system::Pallet<T>>::block_number());
				let current_blocknumber: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();
				ValidTransaction::with_tag_prefix("ares-oracle::submit_local_xray")
					.priority(T::UnsignedPriority::get())
					.and_provides(&current_blocknumber.saturating_add((*host_key).into()))
					.longevity(5)
					.propagate(true)
					.build()
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn purchased_default_setting)]
	pub(super) type PurchasedDefaultSetting<T: Config> = StorageValue<_, PurchasedDefaultData, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn ocw_control_setting)]
	pub(super) type OcwControlSetting<T: Config> = StorageValue<_, OcwControlData, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn purchased_request_pool)]
	pub(super) type PurchasedRequestPool<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		PurchaseId, // purchased_id
		PurchasedRequestData<T::AccountId, BalanceOf<T>, T::BlockNumber>,
		ValueQuery,
	>;

	// migrated
	#[pallet::storage]
	#[pallet::getter(fn purchased_price_pool)]
	pub(super) type PurchasedPricePool<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PurchaseId, // purchased_id,
		Blake2_128Concat,
		PriceKey, // price_key,
		Vec<AresPriceData<T::AccountId, T::BlockNumber>>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn purchased_avg_price)]
	pub(super) type PurchasedAvgPrice<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PurchaseId, // purchased_id,
		Blake2_128Concat,
		PriceKey, // price_key,,
		PurchasedAvgPriceData,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn purchased_avg_trace)]
	pub(super) type PurchasedAvgTrace<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		PurchaseId, // pricpurchased_ide_key
		T::BlockNumber,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn purchased_order_pool)]
	pub(super) type PurchasedOrderPool<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PurchaseId, // purchased_id,
		Blake2_128Concat,
		T::AccountId,
		T::BlockNumber,
		ValueQuery,
	>;

	// migrated
	#[pallet::storage]
	#[pallet::getter(fn last_price_author)]
	pub(super) type LastPriceAuthor<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		PriceKey, // price_key
		T::AccountId,
		ValueQuery,
	>;

	// migrated
	/// The lookup table for names.
	#[pallet::storage]
	#[pallet::getter(fn ares_prices)]
	pub(super) type AresPrice<T: Config> =
		StorageMap<_, Blake2_128Concat, PriceKey, Vec<AresPriceData<T::AccountId, T::BlockNumber>>, ValueQuery>;

	// migrated
	/// The lookup table for names.
	#[pallet::storage]
	#[pallet::getter(fn ares_abnormal_prices)]
	pub(super) type AresAbnormalPrice<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		PriceKey,
		Vec<(AresPriceData<T::AccountId, T::BlockNumber>, AvgPriceData)>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ares_avg_prices)]
	pub(super) type AresAvgPrice<T: Config> =
		StorageMap<_, Blake2_128Concat, PriceKey, (u64, FractionLength), ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn price_pool_depth)]
	pub(super) type PricePoolDepth<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn price_allowable_offset)]
	pub(super) type PriceAllowableOffset<T: Config> = StorageValue<_, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn prices_requests)]
	pub(super) type PricesRequests<T: Config> = StorageValue<
		_,
		Vec<(
			PriceKey,   // price key
			PriceToken, // price token
			u32,        // parse version number.
			FractionLength,
			RequestInterval,
		)>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn request_base_onchain)]
	pub(super) type RequestBaseOnchain<T: Config> = StorageValue<_, Vec<u8>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn jump_block_number)]
	pub(super) type JumpBlockNumber<T: Config> = StorageMap<_, Blake2_128Concat, PriceKey, u64>;

	// FinalPerCheckResult
	#[pallet::storage]
	#[pallet::getter(fn final_per_check_result)]
	pub(super) type FinalPerCheckResult<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Option<(T::BlockNumber, PreCheckStatus, Option<PreCheckCompareLog>)>,
	>;

	// PreCheckTaskList
	#[pallet::storage]
	#[pallet::getter(fn per_check_task_list)]
	pub(super) type PreCheckTaskList<T: Config> =
		StorageValue<_, Vec<(T::AccountId, T::AuthorityAres, T::BlockNumber)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn symbol_fraction)]
	pub(super) type SymbolFraction<T: Config> = StorageMap<_, Blake2_128Concat, PriceKey, FractionLength>;

	// pre_check_session_multi A parameter used to determine the pre-verification
	// task a few session cycles before the validator election, the type is BlockNumber.
	#[pallet::storage]
	#[pallet::getter(fn conf_pre_check_session_multi)]
	pub(super) type ConfPreCheckSessionMulti<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	// pre_check_token_list is used to check the token_list with the average value on the chain.
	#[pallet::storage]
	#[pallet::getter(fn conf_pre_check_token_list)]
	pub(super) type ConfPreCheckTokenList<T: Config> = StorageValue<
		_,
		TokenList, // Vec<price_key>
		ValueQuery,
	>;

	// pre_check_allowable_offset is maximum allowable deviation when comparing means.
	#[pallet::storage]
	#[pallet::getter(fn conf_pre_check_allowable_offset)]
	pub(super) type ConfPreCheckAllowableOffset<T: Config> = StorageValue<
		_,
		Percent, //
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn http_err_trace_log)]
	pub(super) type HttpErrTraceLogV1<T: Config> =
		StorageValue<_, Vec<HttpErrTraceData<T::BlockNumber, T::AccountId>>, ValueQuery>;

	#[pallet::storage]
	pub(crate) type StorageVersion<T: Config> = StorageValue<_, Releases, ValueQuery>;

	#[pallet::storage]
	pub(crate) type LocalXRay<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, (T::BlockNumber, Vec<u8>, Vec<T::AuthorityAres>)>;

	/// The current authority set.
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub(super) type Authorities<T: Config> = StorageValue<_, Vec<(T::AccountId, T::AuthorityAres)>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub _phantom: sp_std::marker::PhantomData<T>,
		pub request_base: Vec<u8>,
		pub price_allowable_offset: u8,
		pub price_pool_depth: u32,
		pub price_requests: Vec<(Vec<u8>, Vec<u8>, u32, FractionLength, RequestInterval)>,
		pub authorities: Vec<(T::AccountId, T::AuthorityAres)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				_phantom: Default::default(),
				request_base: Vec::new(),
				price_allowable_offset: 10u8,
				price_pool_depth: 10u32,
				price_requests: Vec::new(),
				authorities: Vec::new(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			if !self.price_requests.is_empty() {
				// let price_requests: Vec<(PriceKey, PriceToken, u32, FractionLength, RequestInterval)> = self
				// 	.price_requests
				// 	.iter()
				// 	.map(|(price_key, price_token, parse_version, fraction, request_interval)| {
				// 		let price_key: PriceKey = price_key.to_vec().try_into().expect("price key is too long");
				// 		let price_token: PriceToken = price_token.to_vec().try_into().expect("price key is too long");
				// 		(price_key, price_token, *parse_version, *fraction, *request_interval)
				// 	})
				// 	.collect();
				// let price_requests: BoundedVec<
				// 	(PriceKey, PriceToken, u32, FractionLength, RequestInterval),
				// 	MaximumPricesRequestList,
				// > = price_requests.try_into().expect("price_requests is too long");
				// PricesRequests::<T>::put(&price_requests);
				PricesRequests::<T>::put(&self.price_requests);
				self.price_requests
					.iter()
					.for_each(|(symbol, _, _, fraction_length, _)| {
						SymbolFraction::<T>::insert(symbol, fraction_length);
					})
			}
			if self.price_pool_depth > 0 {
				PricePoolDepth::<T>::put(&self.price_pool_depth);
			}
			if self.request_base.len() > 0 {
				RequestBaseOnchain::<T>::put(&self.request_base);
			}
			Pallet::<T>::initialize_authorities(self.authorities.clone());
			PriceAllowableOffset::<T>::put(&self.price_allowable_offset);
			// For new vesrion.
			ConfPreCheckAllowableOffset::<T>::put(Percent::from_percent(10));
			let session_multi: T::BlockNumber = 2u32.into();
			ConfPreCheckSessionMulti::<T>::put(session_multi);
			let mut token_list: TokenList = Vec::default();
			token_list.push(b"btc-usdt".to_vec());
			token_list.push(b"eth-usdt".to_vec());
			token_list.push(b"dot-usdt".to_vec());
			// token_list.try_push(b"btc-usdt".to_vec().try_into().expect("symbol is too long"));
			// token_list.try_push(b"eth-usdt".to_vec().try_into().expect("symbol is too long"));
			// token_list.try_push(b"dot-usdt".to_vec().try_into().expect("symbol is too long"));

			ConfPreCheckTokenList::<T>::put(token_list);
			// V1_2_0
			StorageVersion::<T>::put(Releases::V1_2_0);
		}
	}
}

impl<T: Config> Pallet<T> {
	fn change_authorities(new: Vec<(T::AccountId, T::AuthorityAres)>) {
		// let new: BoundedVec<(T::AccountId, T::AuthorityAres), MaximumAuthorities> =
		// 	new.try_into().expect("authorities is too long");
		<Authorities<T>>::put(&new);
	}

	fn initialize_authorities(authorities: Vec<(T::AccountId, T::AuthorityAres)>) {
		if !authorities.is_empty() {
			assert!(
				<Authorities<T>>::get().is_empty(),
				"Ares Authorities are already initialized!"
			);
			// let authorities: BoundedVec<(T::AccountId, T::AuthorityAres), MaximumAuthorities> =
			// 	authorities.try_into().expect("authorities is too long");
			<Authorities<T>>::put(authorities);
		}
	}

	fn get_auth_id(stash: &T::AccountId) -> Option<T::AuthorityAres> {
		for (storage_stash, auth) in <Authorities<T>>::get().into_iter() {
			if stash == &storage_stash {
				return Some(auth);
			}
		}
		None
	}

	fn get_stash_id(auth: &T::AuthorityAres) -> Option<T::AccountId> {
		for (stash, storage_auth) in <Authorities<T>>::get().into_iter() {
			if auth == &storage_auth {
				return Some(stash);
			}
		}
		None
	}

	fn get_stash_id_or_default(auth: &T::AuthorityAres) -> T::AccountId {
		if let Some(stash_id) = Self::get_stash_id(auth) {
			return stash_id;
		}
		Default::default()
	}

	fn are_block_author_and_sotre_key_the_same(block_author: &T::AuthorityAres) -> bool {
		let mut is_same = !<OcwControlSetting<T>>::get().need_verifier_check;
		if is_same {
			log::warn!(
				target: "pallet::are_block_author_and_sotre_key_the_same",
				"❗❗❗ verifier_check is disable, current status is debug."
			);
			return true;
		}
		let worker_ownerid_list = Self::get_ares_authority_list();
		worker_ownerid_list.iter().any(|local_auth| local_auth == block_author)
	}

	/// Obtain ares price and submit it.
	fn ares_price_worker(block_number: T::BlockNumber, account_id: T::AuthorityAres) -> Result<(), &'static str>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let res = Self::save_fetch_ares_price_and_send_payload_signed(block_number.clone(), account_id.clone()); // PriceKey::PRICE_KEY_IS_ETH
		if let Err(e) = res {
			log::error!(
				target: "pallet::ocw::ares_price_worker",
				"⛔ block number = {:?}, account = {:?}, error = {:?}",
				block_number, account_id, e
			);
		}
		Ok(())
	}

	// Dispose purchased price request.
	fn ares_purchased_worker(block_number: T::BlockNumber, account_id: T::AuthorityAres) -> Result<(), &'static str>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let res = Self::save_fetch_purchased_price_and_send_payload_signed(block_number.clone(), account_id.clone());
		if let Err(e) = res {
			log::error!(
				target: "pallet::ocw::purchased_price_worker",
				"⛔ block number = {:?}, account = {:?}, error = {:?}",
				block_number, account_id, e
			);
		}
		Ok(())
	}

	// Dispose purchased price request.
	fn ares_purchased_checker(block_number: T::BlockNumber, account_id: T::AuthorityAres) -> Result<(), &'static str>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let res = Self::save_forced_clear_purchased_price_payload_signed(block_number.clone(), account_id.clone());
		if let Err(e) = res {
			log::error!(
				target: "pallet::ocw::ares_purchased_checker",
				"⛔ block number = {:?}, account = {:?}, error = {:?}",
				block_number, account_id, e
			);
		}
		Ok(())
	}

	// get uri key raw of ARES price
	fn get_raw_price_source_list() -> Vec<(PriceKey, PriceToken, u32, FractionLength, RequestInterval)> {
		let result: Vec<(PriceKey, PriceToken, u32, FractionLength, RequestInterval)> = <PricesRequests<T>>::get()
			.into_iter()
			.map(
				|(price_key, price_token, parse_version, fraction_length, request_interval)| {
					(
						price_key,
						// sp_std::str::from_utf8(&request_url).unwrap().clone(),
						price_token,
						parse_version,
						fraction_length,
						request_interval,
					)
				},
			)
			.collect();
		result
	}

	fn get_local_host_key() -> u32 {
		let storage_local_host_key = StorageValueRef::persistent(LOCAL_HOST_KEY);

		let old_key = storage_local_host_key.get::<u32>();
		if let Some(storage_key) = old_key.unwrap() {
			return storage_key;
		}

		let seed = sp_io::offchain::random_seed();
		let random = <u32>::decode(&mut TrailingZeroInput::new(seed.as_ref()));
		if random.is_ok() {
			let random = random.unwrap();
			storage_local_host_key.set(&random);
			return random;
		}
		0
	}

	// Get request domain, include TCP protocol, example: http://www.xxxx.com
	fn get_local_storage_request_domain() -> Vec<u8> {
		let request_base_onchain = RequestBaseOnchain::<T>::get();
		if request_base_onchain.len() > 0 {
			return request_base_onchain;
		}

		let storage_request_base = StorageValueRef::persistent(LOCAL_STORAGE_PRICE_REQUEST_DOMAIN);

		if let Some(request_base) = storage_request_base.get::<Vec<u8>>().unwrap_or(Some(Vec::new())) {
			if let Ok(result_base_str) = sp_std::str::from_utf8(&request_base) {
				log::info!("🚅 Ares local request base: {:?} .", &result_base_str);
			}
			return request_base;
		} else {
			log::warn!(
				target: "pallet::ocw::get_local_storage_request_domain",
				"❗ Not found request base url."
			);
		}
		// log::info!("Ares local request base : {:?} .", &result_base_str);
		Vec::new()
	}

	// fn make_local_storage_request_uri_by_str(sub_path: &str) -> Vec<u8> {
	//     Self::make_local_storage_request_uri_by_vec_u8(sub_path.as_bytes().to_vec())
	// }

	fn make_local_storage_request_uri_by_vec_u8(sub_path: Vec<u8>) -> Vec<u8> {
		let domain = Self::get_local_storage_request_domain(); //.as_bytes().to_vec();
		[domain, sub_path].concat()
	}

	fn get_block_author() -> Option<T::AuthorityAres> {
		let digest = <frame_system::Pallet<T>>::digest();
		let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
		let idx = <T as pallet::Config>::FindAuthor::find_author(pre_runtime_digests)?;
		let validators = <Pallet<T>>::authorities();
		validators.get(idx as usize).map(|(_, k)| k.clone())
	}

	fn save_forced_clear_purchased_price_payload_signed(
		block_number: T::BlockNumber,
		account_id: T::AuthorityAres,
	) -> Result<(), &'static str>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let force_request_list = Self::get_expired_purchased_transactions();

		log::debug!("🚅 Force request list length: {:?} .", &force_request_list.len());
		if force_request_list.len() > 0 {
			let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
			// Singer
			let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
				.with_filter(sign_public_keys)
				.send_unsigned_transaction(
					|account| PurchasedForceCleanPayload {
						block_number,
						purchase_id_list: force_request_list.clone(),
						auth: account_id.clone(),
						public: account.public.clone(),
					},
					|payload, signature| Call::submit_forced_clear_purchased_price_payload_signed {
						price_payload: payload,
						signature,
					},
				)
				.ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
			result.map_err(|()| "⛔ Unable to submit transaction")?;
		}
		Ok(())
	}

	fn handler_get_sign_public_keys(account_id: T::AuthorityAres) -> Vec<<T as SigningTypes>::Public>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let mut sign_public_keys: Vec<<T as SigningTypes>::Public> = Vec::new();
		let encode_data: Vec<u8> = account_id.encode();
		assert!(32 == encode_data.len());
		let raw_data = encode_data.try_into();
		let raw_data = raw_data.unwrap();
		// let new_account = T::AuthorityAres::unchecked_from(raw_data);
		let new_account = sp_core::sr25519::Public::from_raw(raw_data);
		sign_public_keys.push(new_account.into());
		sign_public_keys
	}

	//
	fn save_fetch_purchased_price_and_send_payload_signed(
		block_number: T::BlockNumber,
		account_id: T::AuthorityAres,
	) -> Result<(), &'static str>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let mut price_list = Vec::new();
		// Get purchased request by AccountId
		let purchased_key = Self::fetch_purchased_request_keys(account_id.clone());

		if purchased_key.is_none() {
			log::info!("🚅 Waiting for purchased service.");
			return Ok(());
		}

		let purchased_key = purchased_key.unwrap();
		if 0 == purchased_key.raw_source_keys.len() {
			log::warn!(
				target: "pallet::ocw::save_fetch_purchased_price_and_send_payload_signed",
				"❗ Purchased raw key is empty."
			);
			return Ok(());
		}

		let fetch_http_result = Self::fetch_bulk_price_with_http(purchased_key.clone().raw_source_keys);

		let price_result = fetch_http_result;
		if price_result.is_err() {
			log::error!(
				target: "pallet::ocw::save_fetch_purchased_price_and_send_payload_signed",
				"⛔ Ocw network error."
			);
			// Record http error.
			Self::trace_network_error(
				account_id,
				purchased_key.clone().raw_source_keys,
				price_result.err().unwrap(),
				"purchased_worker".as_bytes().to_vec(),
			);
			return Ok(());
		}

		for (price_key, price_option, fraction_length, json_number_value, timestamp) in price_result.unwrap() {
			if price_option.is_some() {
				// record price to vec!
				price_list.push(PricePayloadSubPrice(
					price_key,
					price_option.unwrap(),
					fraction_length,
					JsonNumberValue::new(json_number_value),
					timestamp,
				));
			}
		}
		let price_list: PricePayloadSubPriceList = price_list.try_into().expect("price list is too long");
		log::debug!("🚅 fetch purchased price count: {:?}", price_list.len());
		if price_list.len() > 0 {
			let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
			// Singer
			let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
				.with_filter(sign_public_keys)
				.send_unsigned_transaction(
					|account| PurchasedPricePayload {
						price: price_list.clone(),
						block_number,
						purchase_id: purchased_key.purchase_id.clone(),
						auth: account_id.clone(),
						public: account.public.clone(),
					},
					|payload, signature| Call::submit_purchased_price_unsigned_with_signed_payload {
						price_payload: payload,
						signature,
					},
				)
				.ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
			result.map_err(|()| "⛔ Unable to submit transaction")?;
		}
		Ok(())
	}

	fn save_offchain_pre_check_result(
		stash_id: T::AccountId,
		auth_id: T::AuthorityAres,
		block_number: T::BlockNumber,
		pre_check_list: PreCheckList,
		task_at: T::BlockNumber,
	) -> Result<(), &'static str>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let sign_public_keys = Self::handler_get_sign_public_keys(auth_id.clone());
		// Singer
		let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
			.with_filter(sign_public_keys)
			.send_unsigned_transaction(
				|account| PreCheckResultPayload {
					pre_check_stash: stash_id.clone(),
					pre_check_auth: auth_id.clone(),
					block_number,
					pre_check_list: pre_check_list.clone(),
					task_at,
					public: account.public.clone(),
				},
				|payload, signature| Call::submit_offchain_pre_check_result {
					preresult_payload: payload,
					signature,
				},
			)
			.ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
		result.map_err(|()| "⛔ Unable to submit transaction")?;
		Ok(())
	}

	fn trace_network_error(
		account_id: T::AuthorityAres,
		_format_arr: RawSourceKeys,
		http_err: HttpError,
		tip: Vec<u8>,
	) -> Result<(), &'static str>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
		// let tip: BoundedVec<u8, MaximumErrorTip> = tip.try_into().expect("tip is too long");
		// Singer
		let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
			.with_filter(sign_public_keys)
			.send_unsigned_transaction(
				|account| HttpErrTracePayload {
					trace_data: HttpErrTraceData {
						block_number: <system::Pallet<T>>::block_number(),
						// request_list: format_arr.clone(),
						err_auth: Self::get_stash_id(&account_id.clone()).unwrap(),
						err_status: http_err.clone(),
						tip: tip.clone(),
					},
					auth: account_id.clone(),
					public: account.public.clone(),
				},
				|payload, signature| Call::submit_offchain_http_err_trace_result {
					err_payload: payload,
					signature,
				},
			)
			.ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
		result.map_err(|()| "⛔ Unable to submit transaction")?;
		Ok(())
	}

	fn save_create_pre_check_task(
		account_id: T::AuthorityAres,
		stash_id: T::AccountId,
		auth_id: T::AuthorityAres,
		block_number: T::BlockNumber,
	) -> Result<(), &'static str>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());

		let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
			.with_filter(sign_public_keys)
			.send_unsigned_transaction(
				|account| PreCheckPayload {
					block_number,
					pre_check_stash: stash_id.clone(),
					pre_check_auth: auth_id.clone(),
					auth: account_id.clone(),
					public: account.public.clone(),
				},
				|payload, signature| Call::submit_create_pre_check_task {
					precheck_payload: payload,
					signature,
				},
			)
			.ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
		result.map_err(|()| "⛔ Unable to submit transaction")?;
		Ok(())
	}

	fn save_fetch_ares_price_and_send_payload_signed(
		block_number: T::BlockNumber,
		account_id: T::AuthorityAres,
	) -> Result<(), &'static str>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
	{
		let mut price_list = Vec::new();
		// Get raw request.
		let format_arr = Self::make_bulk_price_format_data(block_number);
		// Filter jump block info
		let (format_arr, jump_block) = Self::filter_jump_block_data(
			format_arr.clone(),
			Self::get_stash_id(&account_id).unwrap(),
			block_number,
		);
		let price_result = Self::fetch_bulk_price_with_http(format_arr.clone());
		if price_result.is_err() {
			log::error!(
				target: "pallet::ocw::save_fetch_purchased_price_and_send_payload_signed",
				"⛔ Ocw network error."
			);
			Self::trace_network_error(
				account_id,
				format_arr.clone(),
				price_result.err().unwrap(),
				"ares_price_worker".as_bytes().to_vec(),
			);
			return Ok(());
		}
		let price_result = price_result.unwrap();
		for (price_key, price_option, fraction_length, json_number_value, timestamp) in price_result {
			if price_option.is_some() {
				// record price to vec!
				price_list.push(PricePayloadSubPrice(
					price_key,
					price_option.unwrap(),
					fraction_length,
					JsonNumberValue::new(json_number_value),
					timestamp,
				));
			}
		}
		log::debug!(
			"🚅 fetch price count: {:?}, jump block count: {:?}",
			price_list.len(),
			jump_block.len()
		);
		let price_list: PricePayloadSubPriceList = price_list.try_into().expect("price_list is too long");
		if price_list.len() > 0 || jump_block.len() > 0 {
			let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
			// Singer
			let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
				.with_filter(sign_public_keys)
				.send_unsigned_transaction(
					|account| PricePayload {
						price: price_list.clone(),
						jump_block: jump_block.clone(),
						block_number,
						auth: account_id.clone(),
						public: account.public.clone(),
					},
					|payload, signature| Call::submit_price_unsigned_with_signed_payload {
						price_payload: payload,
						signature,
					},
				)
				.ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
			result.map_err(|()| "⛔ Unable to submit transaction")?;
		}
		Ok(())
	}

	//
	fn make_bulk_price_request_url(format: RawSourceKeys) -> (Vec<u8>, Vec<u8>) {
		let raw_request_url = Self::make_local_storage_request_uri_by_vec_u8(
			"/api/getBulkCurrencyPrices?currency=usdt".as_bytes().to_vec(),
		);

		let mut request_url = Vec::new();
		for (_, extract_key, _) in format {
			if request_url.len() == 0 {
				request_url = [
					raw_request_url.clone(),
					"&symbol=".as_bytes().to_vec(),
					extract_key.to_vec(),
				]
				.concat();
			} else {
				request_url = [request_url, "_".as_bytes().to_vec(), extract_key.to_vec()].concat();
			}
		}
		(request_url, "usdt".as_bytes().to_vec())
	}

	// Use to filter out those format_data of price that need to jump block.
	fn filter_jump_block_data(
		format_data: Vec<(PriceKey, PriceToken, FractionLength, RequestInterval)>,
		account: T::AccountId,
		_block_number: T::BlockNumber,
	) -> (RawSourceKeys, PricePayloadSubJumpBlockList) {
		// isNeedUpdateJumpBlock
		let mut new_format_data: RawSourceKeys = Vec::default();
		let mut jump_format_data: PricePayloadSubJumpBlockList = Vec::default();

		format_data
			.iter()
			.any(|(price_key, price_token, fraction_length, request_interval)| {
				if Self::is_need_update_jump_block(price_key.clone(), account.clone()) {
					jump_format_data.push(PricePayloadSubJumpBlock(price_key.clone(), *request_interval));
				} else {
					new_format_data.push((price_key.clone(), price_token.clone(), *fraction_length));
				}
				false
			});
		(new_format_data, jump_format_data)
	}

	// Judge the author who submitted the price last time, and return true if it is consistent with this
	// time.
	fn is_need_update_jump_block(price_key: PriceKey, account: T::AccountId) -> bool {
		if !Self::is_aura() || 1 == T::AuthorityCount::get_validators_count() {
			return false;
		}
		match Self::get_last_price_author(price_key) {
			None => false,
			Some(x) => x == account,
		}
	}

	fn keystore_validator_member() -> Option<T::AuthorityAres>
	where
		<T as frame_system::Config>::AccountId: From<sp_application_crypto::sr25519::Public>,
	{
		let local_keys: Vec<T::AuthorityAres> = Self::get_ares_authority_list();
		for local_auth in local_keys.into_iter() {
			if Self::is_validator_member(&local_auth) {
				return Some(local_auth);
			}
		}
		None
	}

	fn is_validator_member(validator: &T::AuthorityAres) -> bool {
		// let mut find_validator = !T::NeedVerifierCheck::get();
		let mut find_validator = !<OcwControlSetting<T>>::get().need_verifier_check;
		if find_validator {
			log::warn!(
				target: "ares_oracle::is_validator_member",
				"❗ Currently in debug mode because need_verifier_check is set to true."
			);
			return true;
		}
		<Authorities<T>>::get().iter().any(|(_, auth)| auth == validator)
	}

	// Store the list of authors in the price list.
	fn update_last_price_list_for_author(price_list: Vec<PriceKey>, author: T::AccountId) {
		price_list.iter().any(|price_key| {
			// find_author()
			<LastPriceAuthor<T>>::insert(price_key.clone(), author.clone());
			false
		});
	}

	// Get
	fn get_last_price_author(price_key: PriceKey) -> Option<T::AccountId> {
		if <LastPriceAuthor<T>>::contains_key(&price_key) {
			return Some(<LastPriceAuthor<T>>::get(price_key).into());
		}
		None
	}

	fn is_aura() -> bool {
		let digest = frame_system::Pallet::<T>::digest();
		let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
		// let author = T::FindAuthor::find_author(pre_runtime_digests.clone());
		let mut is_aura = false;
		for (id, _) in pre_runtime_digests {
			if id == AURA_ENGINE_ID {
				is_aura = true;
			}
		}
		is_aura
	}

	// Make bulk request format array.
	fn make_bulk_price_format_data(
		block_number: T::BlockNumber,
	) -> Vec<(PriceKey, PriceToken, FractionLength, RequestInterval)> {
		let mut format = Vec::new();

		// price_key, request_url, parse_version, fraction_length
		let source_list = Self::get_raw_price_source_list();

		// In the new version, it is more important to control the request interval here.
		for (price_key, extract_key, parse_version, fraction_length, request_interval) in source_list {
			if 2 == parse_version {
				let mut round_number: u64 = block_number.unique_saturated_into();
				round_number += Self::get_jump_block_number(price_key.clone());
				let remainder: u64 = (round_number % request_interval as u64).into();
				if 0 == remainder {
					format.push((price_key, extract_key, fraction_length, request_interval));
				}
			}
		}
		log::debug!("🚅 Ares will be request list count {:?}", format.len());
		format
	}

	//
	fn fetch_bulk_price_with_http(
		format_arr: RawSourceKeys,
	) -> Result<Vec<(PriceKey, Option<u64>, FractionLength, NumberValue, u64)>, HttpError> {
		// make request url
		let (request_url_vu8, base_coin) = Self::make_bulk_price_request_url(format_arr.clone());
		let request_url = sp_std::str::from_utf8(&request_url_vu8).unwrap();
		// let bounded_request_url: BoundedVec<u8, MaximumURLLength> =
		// 	request_url_vu8.clone().try_into().expect("url is too long");

		// request and return http body.
		if "" == request_url {
			log::warn!(target: "pallet::ocw::fetch_bulk_price_with_http", "❗ Ares http requests cannot be empty.");
			return Ok(Vec::new());
		}
		log::info!("🚅 Batch price request address: {:?}", request_url);
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(4_000));
		let request = http::Request::get(request_url.clone());
		let pending = request
			.deadline(deadline)
			.send()
			.map_err(|_| HttpError::IoErr(request_url_vu8.clone()))?;
		let response = pending.try_wait(deadline).map_err(|e| {
			log::warn!(
				target: "pallet::ocw::fetch_bulk_price_with_http",
				"❗ The network cannot connect. http::Error::DeadlineReached error = {:?}",
				e
			);
			// http::Error::DeadlineReached
			// HttpError::TimeOut
		});

		if response.is_err() {
			return Err(HttpError::TimeOut(request_url_vu8.clone()));
		}

		let response = response.unwrap();

		if response.is_err() {
			log::warn!(
				target: "pallet::ocw::fetch_bulk_price_with_http",
				"❗ Https is not currently supported.",
			);
			return Err(HttpError::IoErr(request_url_vu8.clone()));
		}

		let response = response.unwrap();

		if response.code != 200 {
			log::warn!(
				target: "pallet::ocw::fetch_bulk_price_with_http",
				"❗ Unexpected http status code: {}",
				response.code
			);
			// return Err(http::Error::Unknown);
			return Err(HttpError::StatusErr(request_url_vu8.clone(), response.code));
		}
		let body = response.body().collect::<Vec<u8>>();
		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!(
				target: "pallet::ocw::fetch_bulk_price_with_http",
				"❗ Extracting body error, No UTF8 body!"
			);
			// http::Error::IoError
			HttpError::ParseErr(request_url_vu8.clone())
		})?;
		Ok(Self::bulk_parse_price_of_ares(body_str, base_coin, format_arr))
	}

	// handler for bulk_parse_price_of_ares
	fn extract_bulk_price_by_json_value(
		json_val: JsonValue,
		find_key: &str,
		param_length: FractionLength,
	) -> Option<(u64, NumberValue, u64)> {
		assert!(param_length <= 6, "Fraction length must be less than or equal to 6");
		let price_value = match json_val {
			JsonValue::Object(obj) => {
				// TODO:: need test again.
				let find_result = obj.into_iter().find(|(k, _)| {
					// let tmp_k = k.iter().copied();
					k.iter().copied().eq(find_key.chars())
				});

				match find_result {
					None => {
						return None;
					}
					Some((_, sub_val)) => match sub_val {
						JsonValue::Object(price_obj) => {
							let (_, price) = price_obj
								.clone()
								.into_iter()
								.find(|(k, _)| k.iter().copied().eq("price".chars()))
								.unwrap();
							let match_number = match price {
								JsonValue::Number(number) => number,
								_ => return None,
							};
							let (_, timestamp) = price_obj
								.into_iter()
								.find(|(k, _)| k.iter().copied().eq("timestamp".chars()))
								.unwrap();
							let match_timestamp = match timestamp {
								JsonValue::Number(timestamp) => timestamp.integer as u64,
								_ => return None,
							};
							Some((match_number, match_timestamp))
						}
						_ => {
							return None;
						}
					},
				}
			}
			_ => {
				return None;
			}
		};

		if price_value.is_none() {
			return None;
		}
		let price_value = price_value.unwrap();

		// Make u64 with fraction length
		let result_price = JsonNumberValue::new(price_value.0.clone()).to_price(param_length);

		// A price of 0 means that the correct result of the data is not obtained.
		if result_price == 0 {
			return None;
		}
		Some((result_price, price_value.0, price_value.1))
	}

	fn bulk_parse_price_of_ares(
		price_str: &str,
		base_coin: Vec<u8>,
		format: RawSourceKeys,
	) -> Vec<(PriceKey, Option<u64>, FractionLength, NumberValue, u64)> {
		let val = lite_json::parse_json(price_str);

		let mut result_vec = Vec::new();
		match val.ok() {
			None => {
				return Vec::new();
			}
			Some(obj) => {
				match obj {
					JsonValue::Object(obj) => {
						// find code root.
						let (_, v_data) = obj
							.into_iter()
							.find(|(k, _)| {
								let _tmp_k = k.iter().copied();
								k.iter().copied().eq("data".chars())
							})
							.unwrap();

						for (price_key, extract_key, fraction_length) in format {
							let new_extract_key = [extract_key.to_vec(), base_coin.clone()].concat();
							let extract_key = sp_std::str::from_utf8(&new_extract_key).unwrap();
							let extract_price_grp =
								Self::extract_bulk_price_by_json_value(v_data.clone(), extract_key, fraction_length);

							if extract_price_grp.is_some() {
								let (extract_price, json_number_value, timestamp) = extract_price_grp.unwrap();
								// TODO::need recheck why use Some(extract_price)
								result_vec.push((
									price_key,
									Some(extract_price),
									fraction_length,
									json_number_value,
									timestamp,
								));
							}
						}
					}
					_ => return Vec::new(),
				}
			}
		}
		result_vec
	}

	fn parse_price_of_ares(price_str: &str, param_length: FractionLength) -> Option<u64> {
		assert!(param_length <= 6, "Fraction length must be less than or equal to 6");

		let val = lite_json::parse_json(price_str);
		let price = match val.ok()? {
			JsonValue::Object(obj) => {
				// find code root.
				let (_, v_data) = obj.into_iter().find(|(k, _)| {
					let _tmp_k = k.iter().copied();
					k.iter().copied().eq("data".chars())
				})?;
				// find price value
				match v_data {
					JsonValue::Object(obj) => {
						let (_, v) = obj.into_iter().find(|(k, _)| {
							// let tmp_k = k.iter().copied();
							k.iter().copied().eq("price".chars())
						})?;
						match v {
							JsonValue::Number(number) => number,
							_ => return None,
						}
					}
					_ => return None,
				}
			}
			_ => return None,
		};
		// Some(Self::format_price_fraction_to_u64(price, param_length))
		Some(JsonNumberValue::new(price).to_price(param_length))
	}

	// Get price pool size
	fn get_price_pool_depth() -> u32 {
		// T::PriceVecMaxSize::get()
		<PricePoolDepth<T>>::get()
	}

	fn get_ares_authority_list() -> Vec<T::AuthorityAres> {
		let authority_list = T::AuthorityAres::all(); // T::AuthorityAres::all();
		authority_list
	}

	fn filter_raw_price_source_list(
		request_data: RequestKeys,
	) -> Vec<(PriceKey, PriceToken, FractionLength, RequestInterval)> {
		let source_list = Self::get_raw_price_source_list();
		let mut result = Vec::new();
		source_list
			.iter()
			.any(|(price_key, price_token, _version, fraction, interval)| {
				if request_data.iter().any(|x| x == price_key) {
					result.push((price_key.clone(), price_token.clone(), *fraction, *interval));
				}
				false
			});
		result
	}

	fn purchased_storage_clean(p_id: PurchaseId) {
		<PurchasedPricePool<T>>::remove_prefix(p_id.clone(), None);
		<PurchasedRequestPool<T>>::remove(p_id.clone());
		<PurchasedOrderPool<T>>::remove_prefix(p_id.clone(), None);
	}

	//
	fn fetch_purchased_request_keys(who: T::AuthorityAres) -> Option<PurchasedSourceRawKeys> // Vec<(Vec<u8>, Vec<u8>, FractionLength, RequestInterval)>
	{
		let mut raw_source_keys = Vec::new();
		let mut raw_purchase_id: PurchaseId = Vec::default();

		let stash_id = Self::get_stash_id(&who).unwrap();
		// Iter
		<PurchasedRequestPool<T>>::iter().any(|(purchase_id, request_data)| {
			if false == <PurchasedOrderPool<T>>::contains_key(purchase_id.clone(), stash_id.clone()) {
				raw_purchase_id = purchase_id.clone();
				raw_source_keys = Self::filter_raw_price_source_list(request_data.request_keys)
					.iter()
					.map(|(price_key, parse_key, fraction_len, _)| {
						(price_key.clone(), parse_key.clone(), *fraction_len)
					})
					.collect();
				return true;
			}
			false
		});

		if raw_source_keys.len() == 0 {
			return None;
		}
		let raw_source_keys: RawSourceKeys = raw_source_keys.clone().try_into().expect("raw_source_keys is too long");
		Some(PurchasedSourceRawKeys {
			purchase_id: raw_purchase_id,
			raw_source_keys,
		})
	}

	fn make_purchase_price_id(who: T::AccountId, add_up: u8) -> Vec<u8> {
		// check add up
		if add_up == u8::MAX {
			panic!("⛔ Add up number too large.");
		}
		let mut account_vec: Vec<u8> = who.encode();
		// Get block number to u64
		let current_block_num: T::BlockNumber = <system::Pallet<T>>::block_number();
		let current_blocknumber: u64 = current_block_num.unique_saturated_into();
		let mut current_bn_vec: Vec<u8> = current_blocknumber.encode();
		account_vec.append(&mut current_bn_vec);
		// Get add up number
		let mut add_u8_vec: Vec<u8> = add_up.encode();
		account_vec.append(&mut add_u8_vec);

		let purchase_id: PurchaseId = account_vec.clone().try_into().expect("id is too long");
		// Check id exists.
		if PurchasedRequestPool::<T>::contains_key(&purchase_id) {
			return Self::make_purchase_price_id(who, add_up.saturating_add(1));
		}
		account_vec
	}

	// submit price on chain.
	fn ask_price(
		who: T::AccountId,
		offer: BalanceOf<T>,
		submit_threshold: u8,
		max_duration: u64,
		purchase_id: PurchaseId,
		request_keys: RequestKeys,
	) -> Result<PurchaseId, Error<T>> {
		// Judge submit_threshold range is (0,100]
		if 0 >= submit_threshold || 100 < submit_threshold {
			return Err(Error::<T>::SubmitThresholdRangeError);
		}
		if 0 >= max_duration {
			return Err(Error::<T>::DruationNumberNotBeZero);
		}
		let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();
		let request_data = PurchasedRequestData {
			account_id: who,
			offer,
			submit_threshold,
			create_bn: <system::Pallet<T>>::block_number(),
			max_duration: current_block.saturating_add(max_duration),
			request_keys,
		};
		<PurchasedRequestPool<T>>::insert(purchase_id.clone(), request_data.clone());
		Self::deposit_event(Event::NewPurchasedRequest {
			purchase_id: purchase_id.clone(),
			request_data,
			value: offer,
			finance_era: T::OracleFinanceHandler::current_era_num(),
		});
		Ok(purchase_id)
	}

	// add price on chain
	fn add_price_and_try_to_agg(
		who: T::AccountId,
		price: u64,
		price_key: PriceKey,
		fraction_length: FractionLength,
		json_number_value: JsonNumberValue,
		max_len: u32,
		timestamp: u64,
	) -> Option<(PriceKey, u64, FractionLength, Vec<T::AccountId>)> {
		let key_str = price_key;
		let current_block = <system::Pallet<T>>::block_number();
		// 1. Check key exists
		if <AresPrice<T>>::contains_key(key_str.clone()) {
			// get and reset .
			let old_price = <AresPrice<T>>::get(key_str.clone());
			let mut is_fraction_changed = false;
			// check fraction length inconsistent.
			for (_index, price_data) in old_price.clone().iter().enumerate() {
				if &price_data.fraction_len != &fraction_length {
					is_fraction_changed = true;
					break;
				}
			}
			//
			let mut new_price = Vec::new();
			let max_len: usize = max_len.clone() as usize;

			for (index, value) in old_price.iter().enumerate() {
				if old_price.len() >= max_len && 0 == index {
					continue;
				}

				let mut old_value = (*value).clone();
				if is_fraction_changed {
					old_value = (*value).clone();
					old_value.price = old_value.raw_number.to_price(fraction_length.clone());
					old_value.fraction_len = fraction_length.clone();
				}

				// let new_value = old_value;
				new_price.push(old_value);
			}
			new_price.push(AresPriceData {
				price: price.clone(),
				account_id: who.clone(),
				create_bn: current_block,
				fraction_len: fraction_length,
				raw_number: json_number_value,
				timestamp,
			});
			<AresPrice<T>>::insert(key_str.clone(), new_price);
		} else {
			// push a new value.
			let mut new_price: Vec<AresPriceData<T::AccountId, T::BlockNumber>> = Vec::new();
			new_price.push(AresPriceData {
				price: price.clone(),
				account_id: who.clone(),
				create_bn: current_block,
				fraction_len: fraction_length,
				raw_number: json_number_value,
				timestamp,
			});
			<AresPrice<T>>::insert(key_str.clone(), new_price);
		}

		let avg_check_result = Self::check_and_update_avg_price_storage(key_str.clone(), max_len);
		// Check if the price request exists, if not, clear storage data.
		let price_request_list = <PricesRequests<T>>::get();
		let has_key = price_request_list
			.iter()
			.any(|(any_price, _, _, _, _)| any_price == &key_str);
		if !has_key {
			Self::clear_price_storage_data(key_str);
		}
		avg_check_result
	}

	fn check_and_update_avg_price_storage(key_str: PriceKey, max_len: u32) -> Option<(PriceKey, u64, FractionLength, Vec<T::AccountId>)> {
		let ares_price_list_len = <AresPrice<T>>::get(key_str.clone()).len();
		if ares_price_list_len >= max_len as usize && ares_price_list_len > 0 {
			return Self::update_avg_price_storage(key_str.clone());
		}
		None
	}

	fn update_avg_price_storage(key_str: PriceKey) -> Option<(PriceKey, u64, FractionLength, Vec<T::AccountId>)> {
		let prices_info = <AresPrice<T>>::get(key_str.clone());
		let average_price_result = Self::average_price(prices_info, T::CalculationKind::get());
		if let Some((average, fraction_length, account_list)) = average_price_result {
			let mut price_list_of_pool = <AresPrice<T>>::get(key_str.clone());
			// Abnormal price index list
			let mut abnormal_price_index_list = Vec::new();
			// Pick abnormal price.
			if 0 < price_list_of_pool.len() {
				for (index, check_price) in price_list_of_pool.iter().enumerate() {
					let offset_percent = match check_price.price {
						x if &x > &average => ((x - average) * 100) / average,
						x if &x < &average => ((average - x) * 100) / average,
						_ => 0,
					};
					if offset_percent > <PriceAllowableOffset<T>>::get() as u64 {
						// Set price to abnormal list and pick out check_price
						<AresAbnormalPrice<T>>::append(
							key_str.clone(),
							(
								check_price,
								AvgPriceData {
									integer: average,
									fraction_len: fraction_length,
								},
							),
						);
						// abnormal_price_index_list
						abnormal_price_index_list.push(index);
					}
				}

				let mut remove_count = 0;
				// has abnormal price.
				if abnormal_price_index_list.len() > 0 {
					// pick out abnormal
					abnormal_price_index_list.iter().any(|remove_index| {
						price_list_of_pool.remove(*remove_index - remove_count);
						remove_count += 1;
						false
					});
					// reset price pool
					<AresPrice<T>>::insert(key_str.clone(), price_list_of_pool);
					return Self::update_avg_price_storage(key_str.clone());
				}

				// Update avg price
				<AresAvgPrice<T>>::insert(key_str.clone(), (average, fraction_length));
				// Clear price pool.
				<AresPrice<T>>::remove(key_str.clone());
				//
				return Some((key_str.clone(), average, fraction_length, account_list));
			}
		}
		None
	}
	// to determine whether the submit price period has expired but there is still no submit.
	// This function returns a list of purchased_id.
	fn get_expired_purchased_transactions() -> Vec<PurchaseId> {
		// let a : PurchasedRequestData<T> ;
		let mut purchased_id_list: Vec<PurchaseId> = Vec::default();
		let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();

		//TODO should be return Result<>
		<PurchasedRequestPool<T>>::iter().any(|(p_id, p_d)| {
			if current_block >= p_d.max_duration {
				purchased_id_list.push(p_id.clone());
			}
			false
		});
		purchased_id_list
	}

	fn handler_purchase_avg_price_storage(
		purchase_id: PurchaseId,
		price_key: PriceKey,
		mut prices_info: Vec<AresPriceData<T::AccountId, T::BlockNumber>>,
		reached_type: u8,
	) -> Option<(PriceKey, PurchasedAvgPriceData, Vec<T::AccountId>)> {
		let (average, fraction_length, account_list) =
			Self::average_price(prices_info.clone(), T::CalculationKind::get()).expect("The average is not empty.");

		// Abnormal price index list
		let mut abnormal_price_index_list = Vec::new();
		// Pick abnormal price.
		if 0 < prices_info.len() {
			for (index, check_price) in prices_info.iter().enumerate() {
				let offset_percent = match check_price.price {
					x if &x > &average => ((x - average) * 100) / average,
					x if &x < &average => ((average - x) * 100) / average,
					_ => 0,
				};
				if offset_percent > <PriceAllowableOffset<T>>::get() as u64 {
					// Set price to abnormal list and pick out check_price
					// TODO:: need update struct of AresAbnormalPrice , add the comparison value of the current
					// deviation
					<AresAbnormalPrice<T>>::append(
						price_key.clone(),
						(
							check_price,
							AvgPriceData {
								integer: average,
								fraction_len: fraction_length,
							},
						),
					);
					// abnormal_price_index_list
					abnormal_price_index_list.push(index);
				}
			}

			let mut remove_count = 0;
			// has abnormal price.
			if abnormal_price_index_list.len() > 0 {
				// pick out abnormal
				abnormal_price_index_list.iter().any(|remove_index| {
					prices_info.remove(*remove_index - remove_count);
					remove_count += 1;
					false
				});
				// reset price pool
				return Self::handler_purchase_avg_price_storage(
					purchase_id.clone(),
					price_key.clone(),
					prices_info.clone(),
					reached_type,
				);
			}
			// let current_block = <system::Pallet<T>>::block_number() as u64;
			let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();
			// get valid request price accounts
			let valid_request_account_id_list: Vec<T::AccountId> = prices_info
				.into_iter()
				.map(|x| {
					// Self::get_stash_id(&x.account_id).unwrap()
					x.account_id
				})
				.collect();

			let avg_price_data = PurchasedAvgPriceData {
				create_bn: current_block,
				reached_type,
				price_data: (average.clone(), fraction_length.clone()),
			};
			// Update avg price (average, fraction_length)
			<PurchasedAvgPrice<T>>::insert(purchase_id.clone(), price_key.clone(), avg_price_data.clone());
			<PurchasedAvgTrace<T>>::insert(purchase_id.clone(), <system::Pallet<T>>::block_number());
			return Some((price_key.clone(), avg_price_data, valid_request_account_id_list));
		}
		None
	}

	// Return aggregation count
	fn update_purchase_avg_price_storage(purchase_id: PurchaseId, reached_type: u8) -> usize {
		// Get purchase price pool
		let price_key_list = <PurchasedPricePool<T>>::iter_key_prefix(purchase_id.clone()).collect::<Vec<_>>();
		//
		let mut event_result_list = Vec::new();
		//
		price_key_list.iter().any(|x| {
			let prices_info = <PurchasedPricePool<T>>::get(purchase_id.clone(), x.clone());
			let result =
				Self::handler_purchase_avg_price_storage(purchase_id.clone(), x.clone(), prices_info, reached_type);
			event_result_list.push(result);
			false
		});

		let result_count =event_result_list.len();
		Self::deposit_event(Event::PurchasedAvgPrice {
			purchase_id: purchase_id.clone(),
			event_results: event_result_list,
			finance_era: T::OracleFinanceHandler::current_era_num(),
		});
		let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();
		Self::check_and_clear_expired_purchased_average_price_storage(purchase_id, current_block);

		result_count
	}

	fn check_and_clear_expired_purchased_average_price_storage(
		purchase_id: PurchaseId,
		current_block_num: u64,
	) -> bool {
		if !<PurchasedAvgTrace<T>>::contains_key(&purchase_id) {
			return false;
		}
		let avg_trace = <PurchasedAvgTrace<T>>::get(purchase_id.clone());
		let avg_trace_num: u64 = avg_trace.unique_saturated_into();
		let purchase_setting = <PurchasedDefaultSetting<T>>::get();
		let comp_blocknum = purchase_setting.avg_keep_duration.saturating_add(avg_trace_num);
		if current_block_num > comp_blocknum {
			<PurchasedAvgPrice<T>>::remove_prefix(purchase_id.clone(), None);
			<PurchasedAvgTrace<T>>::remove(purchase_id.clone());
			return true;
		}
		false
	}

	//
	fn clear_price_storage_data(price_key: PriceKey) {
		<AresPrice<T>>::remove(&price_key);
		<AresAvgPrice<T>>::remove(&price_key);
	}

	/// Calculate current average price. // fraction_length: FractionLength
	fn average_price(
		prices_info: Vec<AresPriceData<T::AccountId, T::BlockNumber>>,
		kind: u8,
	) -> Option<(u64, FractionLength, Vec<T::AccountId>)> {
		let mut fraction_length_of_pool: FractionLength = 0;
		// Check and get fraction_length.
		prices_info.clone().into_iter().any(|tmp_price_data| {
			if 0 == fraction_length_of_pool {
				fraction_length_of_pool = tmp_price_data.fraction_len;
			}
			if fraction_length_of_pool != tmp_price_data.fraction_len {
				panic!("Inconsistent of fraction lenght.")
			}
			false
		});

		let mut prices: Vec<u64> = Vec::new(); // prices_info.clone().into_iter().map(|price_data| price_data.price).collect();
		let mut account_list: Vec<T::AccountId> = Vec::new(); // prices_info.into_iter().map(|price_data| price_data.account_id).collect();

		for price_info in prices_info {
			prices.push(price_info.price);
			account_list.push(price_info.account_id);
		}

		if prices.is_empty() {
			return None;
		}

		match Self::calculation_average_price(prices, kind) {
			Some(price_value) => {
				return Some((price_value, fraction_length_of_pool, account_list));
			}
			_ => {}
		}
		None
	}

	// handleCalculateJumpBlock((interval,jump_block))
	fn handle_calculate_jump_block(jump_format: (u64, u64)) -> (u64, u64) {
		let (interval, jump_block) = jump_format;
		assert!(interval > 0, "The minimum interval value is 1");

		let new_jump_block = match jump_block.checked_sub(1) {
			None => interval.saturating_sub(1),
			Some(_x) => jump_block.saturating_sub(1),
		};
		(interval, new_jump_block)
	}

	fn add_purchased_price(
		purchase_id: PurchaseId,
		account_id: T::AccountId,
		block_number: T::BlockNumber,
		price_list: PricePayloadSubPriceList,
	) {
		if <PurchasedOrderPool<T>>::contains_key(purchase_id.clone(), account_id.clone()) {
			return ();
		}
		let current_block = <system::Pallet<T>>::block_number();
		price_list.iter().any(|PricePayloadSubPrice(a, b, c, d, timestamp)| {
			let mut price_data_vec = <PurchasedPricePool<T>>::get(purchase_id.clone(), a.clone());
			price_data_vec.push(AresPriceData {
				price: *b,
				account_id: account_id.clone(),
				create_bn: current_block,
				fraction_len: *c,
				raw_number: d.clone(),
				timestamp: timestamp.clone(),
			});
			//
			<PurchasedPricePool<T>>::insert(purchase_id.clone(), a.clone(), price_data_vec);
			false
		});

		<PurchasedOrderPool<T>>::insert(purchase_id.clone(), account_id.clone(), current_block.clone());

		Self::deposit_event(Event::NewPurchasedPrice {
			created_at: block_number.clone(),
			price_list,
			who: account_id,
			finance_era: T::OracleFinanceHandler::current_era_num(),
			purchase_id
		});
	}

	// Judge whether the predetermined conditions of the validator of the current
	// purchased_id meet the requirements, and return true if it is
	fn is_validator_purchased_threshold_up_on(purchase_id: PurchaseId) -> bool {
		if false == <PurchasedRequestPool<T>>::contains_key(purchase_id.clone()) {
			return false;
		}

		let reporter_count = <PurchasedOrderPool<T>>::iter_prefix_values(purchase_id.clone()).count();
		if 0 == reporter_count {
			return false;
		}

		let validator_count = T::AuthorityCount::get_validators_count();

		let reporter_count = reporter_count as u64;

		let div_val = (reporter_count * 100) / (validator_count);

		let purchased_request = <PurchasedRequestPool<T>>::get(purchase_id.clone());
		let submit_threshold = purchased_request.submit_threshold as u64;

		div_val >= submit_threshold
	}

	fn is_all_validator_submitted_price(purchased_id: PurchaseId) -> bool {
		if false == <PurchasedRequestPool<T>>::contains_key(purchased_id.clone()) {
			return false;
		}

		let reporter_count = PurchasedOrderPool::<T>::iter_prefix_values(purchased_id.clone()).count();
		if 0 == reporter_count {
			return false;
		}

		let validator_count = T::AuthorityCount::get_validators_count();

		(reporter_count as u64) >= validator_count
	}

	// Split purchased_request by the commas.
	fn extract_purchased_request(request: Vec<u8>) -> RequestKeys {
		// convert to str
		let purchased_str = sp_std::str::from_utf8(request.as_slice());
		if purchased_str.is_err() {
			return Vec::default();
		}
		let purchased_str = purchased_str.unwrap();
		// remove space char.
		// let purchased_str: Vec<char> = purchased_str.chars().filter(|c|{!c.is_whitespace()}).collect();
		// let purchased_str: Vec<u8> = purchased_str.iter().map(|x|{ x.as_()}).collect();
		// // let purchased_str = sp_std::str::from_utf8(purchased_str.as_slice()).unwrap();

		let purchased_vec: Vec<&str> = purchased_str.split(',').collect();

		let mut result = Vec::new();
		for x in purchased_vec {
			if x != "" {
				if false == result.iter().any(|a| a == &x.as_bytes().to_vec()) {
					let price_key: PriceKey = x.as_bytes().to_vec().try_into().expect("price_key is too long");
					result.push(price_key);
				}
			}
		}
		let result: RequestKeys = result.clone().try_into().expect("request_keys is too long");
		result
	}

	// Increase jump block number, PARAM:: price_key, interval
	fn increase_jump_block_number(price_key: PriceKey, interval: u64) -> (u64, u64) {
		let (old_interval, jump_number) =
			Self::handle_calculate_jump_block((interval, Self::get_jump_block_number(price_key.clone())));
		<JumpBlockNumber<T>>::insert(&price_key, jump_number.clone());
		(old_interval, jump_number)
	}

	// Get jump block number
	fn get_jump_block_number(price_key: PriceKey) -> u64 {
		<JumpBlockNumber<T>>::get(&price_key).unwrap_or(0)
	}

	// delete a jump block info.
	fn remove_jump_block_number(price_key: PriceKey) {
		<JumpBlockNumber<T>>::remove(&price_key);
	}

	fn calculation_average_price(mut prices: Vec<u64>, kind: u8) -> Option<u64> {
		if 2 == kind {
			// use median
			prices.sort();
			if prices.len() % 2 == 0 {
				// get 2 mid element then calculation average.
				return Some((prices[prices.len() / 2] + prices[prices.len() / 2 - 1]) / 2);
			} else {
				// get 1 mid element and return.
				return Some(prices[prices.len() / 2]);
			}
		}
		if 1 == kind {
			return Some(prices.iter().fold(0_u64, |a, b| a.saturating_add(*b)) / prices.len() as u64);
		}
		None
	}

	fn validate_transaction_parameters_of_ares(
		block_number: &T::BlockNumber,
		// _price_list: Vec<(Vec<u8>, u64, FractionLength, JsonNumberValue)>,
		_price_list: Vec<PricePayloadSubPrice>,
	) -> TransactionValidity {
		// Let's make sure to reject transactions from the future.
		let current_block = <system::Pallet<T>>::block_number();
		if &current_block < block_number {
			return InvalidTransaction::Future.into();
		}

		ValidTransaction::with_tag_prefix("ares-oracle::validate_transaction_parameters_of_ares")
			.priority(T::UnsignedPriority::get())
			.and_provides(block_number) // next_unsigned_at
			.longevity(5)
			.propagate(true)
			.build()
	}

	fn update_reporter_point(purchase_id: PurchaseId, agg_count: usize) -> Result<(), Error<T>> {
		// transfer balance to pallet account.
		if T::OracleFinanceHandler::pay_to_ask(purchase_id.clone(), agg_count).is_err() {
			return Err(Error::<T>::PayToPurchaseFeeFailed);
		}

		let request_mission: PurchasedRequestData<T::AccountId, BalanceOf<T>, T::BlockNumber> =
			PurchasedRequestPool::<T>::get(purchase_id.clone());
		// update report work point
		<PurchasedOrderPool<T>>::iter_prefix(purchase_id.clone())
			.into_iter()
			.any(|(acc, _)| {
				let _res = T::OracleFinanceHandler::record_submit_point(
					acc,
					purchase_id.clone(),
					request_mission.create_bn,
					agg_count as u32,
				);
				false
			});
		Ok(())
	}
}

pub fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: &str = Deserialize::deserialize(de)?;
	Ok(s.as_bytes().to_vec())
}

#[derive(Deserialize, Encode, Decode, Clone, Default)]
struct LocalPriceRequestStorage {
	// Specify our own deserializing function to convert JSON string to vector of bytes
	#[serde(deserialize_with = "de_string_to_bytes")]
	price_key: Vec<u8>,
	#[serde(deserialize_with = "de_string_to_bytes")]
	request_url: Vec<u8>,
	parse_version: u32,
}

impl fmt::Debug for LocalPriceRequestStorage {
	// `fmt` converts the vector of bytes inside the struct back to string for
	//  more friendly display.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"{{ price_key: {}, request_url: {}, parse_version: {}}}",
			str::from_utf8(&self.price_key).map_err(|_| fmt::Error)?,
			str::from_utf8(&self.request_url).map_err(|_| fmt::Error)?,
			&self.parse_version,
		)
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T>
// where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>
{
	type Public = T::AuthorityAres;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T>
// where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>
{
	type Key = T::AuthorityAres;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityAres)>,
	{
		let authorities = validators
			.map(|(stash, auth)| (stash.clone(), auth))
			.collect::<Vec<_>>();
		Self::initialize_authorities(authorities);
	}

	fn on_new_session<'a, I: 'a>(changed: bool, validators: I, _queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityAres)>,
	{
		if changed {
			let next_authorities = validators
				.map(|(stash, auth)| (stash.clone(), auth))
				.collect::<Vec<_>>();
			let last_authorities = Self::authorities().to_vec();
			if next_authorities != last_authorities {
				Self::change_authorities(next_authorities);
			}
		}
	}

	fn on_disabled(_i: u32) {}
}

pub struct FindAresAccountFromAuthority<T, Inner>(sp_std::marker::PhantomData<(T, Inner)>);

impl<T: Config, Inner: FindAuthor<T::AuthorityAres>> FindAuthor<T::AccountId> for FindAresAccountFromAuthority<T, Inner>
where
	<T as frame_system::Config>::AccountId: From<sp_runtime::AccountId32>,
{
	fn find_author<'a, I>(digests: I) -> Option<T::AccountId>
	where
		I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
	{
		let find_auraid = Inner::find_author(digests)?;
		let mut a = [0u8; 32];
		a[..].copy_from_slice(&find_auraid.to_raw_vec());
		// extract AccountId32 from store keys
		let owner_account_id32 = sp_runtime::AccountId32::new(a);
		let authro_account_id = owner_account_id32.clone().into();
		Some(authro_account_id)
	}
}

impl<T: Config> SymbolInfo for Pallet<T> {
	fn price(symbol: &Vec<u8>) -> Result<(u64, FractionLength), ()> {
		// let bounded_symbol: PriceKey = symbol.clone().try_into().expect("symbol is too long");
		AresAvgPrice::<T>::try_get(symbol)
	}

	fn fraction(symbol: &Vec<u8>) -> Option<FractionLength> {
		// let bounded_symbol: PriceKey = symbol.clone().try_into().expect("symbol is too long");
		SymbolFraction::<T>::get(symbol)
	}
}

impl<T: Config> IAresOraclePreCheck<T::AccountId, T::AuthorityAres, T::BlockNumber> for Pallet<T> {
	//
	fn has_pre_check_task(stash: T::AccountId) -> bool {
		if let Some((_, check_status)) = Self::get_pre_check_status(stash.clone()) {
			match check_status {
				PreCheckStatus::Review => {
					return false;
				}
				PreCheckStatus::Pass => {
					return true;
				}
				PreCheckStatus::Prohibit => {
					return false;
				}
			}
		}
		if <PreCheckTaskList<T>>::get().len() == 0 {
			return false;
		}
		let task_list = <PreCheckTaskList<T>>::get();
		task_list.iter().any(|(storage_stash, _, _)| &stash == storage_stash)
	}

	//
	fn get_pre_task_by_authority_set(
		auth_list: Vec<T::AuthorityAres>,
	) -> Option<(T::AccountId, T::AuthorityAres, T::BlockNumber)> {
		let task_list = <PreCheckTaskList<T>>::get();
		for (stash, auth, bn) in task_list {
			if auth_list.iter().any(|x| x == &auth) {
				// Check status
				match Self::get_pre_check_status(stash.clone()) {
					None => {}
					Some((_, pre_status)) => match pre_status {
						PreCheckStatus::Review => {
							return Some((stash, auth, bn));
						}
						_ => {}
					},
				}
			}
		}
		None
	}

	//
	fn check_and_clean_obsolete_task(maximum_due: T::BlockNumber) -> Weight {
		let mut old_task_list = <PreCheckTaskList<T>>::get();
		let current_block_num: T::BlockNumber = <system::Pallet<T>>::block_number();
		if old_task_list.len() > 0 {
			let old_count = old_task_list.len();
			old_task_list.retain(|(_, _, bn)| {
				let duration_bn = current_block_num.saturating_sub(*bn);
				duration_bn <= maximum_due
			});
			if old_count > old_task_list.len() {
				<PreCheckTaskList<T>>::put(old_task_list);
			}
		}

		// let mut old_final_check_list = <FinalPerCheckResult<T>>::;
		for (key, val) in <FinalPerCheckResult<T>>::iter() {
			if let Some((bn, _per_status, _)) = val {
				// if per_status == PreCheckStatus::Pass {
				//     let duration_bn = current_block_num.saturating_sub(bn);
				//     if duration_bn > maximum_due {
				//         <FinalPerCheckResult<T>>::remove(key);
				//     }
				// }
				let duration_bn = current_block_num.saturating_sub(bn);
				if duration_bn > maximum_due {
					<FinalPerCheckResult<T>>::remove(key);
				}
			}
		}
		0
	}

	// Obtain a set of price data according to the task configuration structure.
	fn take_price_for_pre_check(check_config: PreCheckTaskConfig) -> PreCheckList {
		let mut raw_price_source_list = Self::get_raw_price_source_list();
		raw_price_source_list.retain(|x| {
			check_config.check_token_list.clone().iter().any(|check_price_key| {
				// println!("check_price_key == &x.0, {:?} == {:?}", sp_std::str::from_utf8(check_price_key),
				// sp_std::str::from_utf8(&x.0)) ;
				check_price_key == &x.0
			})
		});

		let format_data: Vec<(PriceKey, PriceToken, FractionLength)> = raw_price_source_list
			.into_iter()
			.map(|(price_key, parse_key, _version, fraction_len, _request_interval)| {
				(price_key, parse_key, fraction_len)
			})
			.collect();
		let format_data: RawSourceKeys = format_data.clone().try_into().expect("format_data is too long");
		// // check_config
		let response_result = Self::fetch_bulk_price_with_http(format_data);
		let response_result = response_result.unwrap_or(Vec::new());
		let response_result: Vec<PreCheckStruct> = response_result
			.into_iter()
			.map(|(price_key, _parse_key, _fraction_len, number_val, timestamp)| {
				let number_val = JsonNumberValue::new(number_val);
				PreCheckStruct {
					price_key,
					number_val,
					max_offset: check_config.allowable_offset.clone(),
					timestamp,
				}
			})
			.collect();
		let response_result: PreCheckList = response_result.try_into().expect("PreCheckList is too long");
		return response_result;
	}

	// Record the per check results and add them to the storage structure.
	fn save_pre_check_result(stash: T::AccountId, bn: T::BlockNumber, pre_check_list: PreCheckList) -> PreCheckStatus {
		assert!(pre_check_list.len() > 0, "⛔️ Do not receive empty result check.");
		// get avg price.
		// let mut chain_avg_price_list = BoundedBTreeMap::<PriceKey, (u64, FractionLength),
		// MaximumMapCapacity>::new(); let mut validator_up_price_list = BoundedBTreeMap::<PriceKey, (u64,
		// FractionLength), MaximumMapCapacity>::new();
		let mut chain_avg_price_list = BTreeMap::<Vec<u8>, (u64, FractionLength)>::new();
		let mut validator_up_price_list = BTreeMap::<Vec<u8>, (u64, FractionLength)>::new();

		let check_result = pre_check_list.iter().all(|checked_struct| {
			// Check price key exists.
			if !<AresAvgPrice<T>>::contains_key(&checked_struct.price_key) {
				return false;
			}
			// Get avg price struct.
			let (avg_price_val, avg_fraction_len) = <AresAvgPrice<T>>::get(&checked_struct.price_key);

			chain_avg_price_list.insert(checked_struct.price_key.clone(), (avg_price_val, avg_fraction_len));
			validator_up_price_list.insert(
				checked_struct.price_key.clone(),
				(checked_struct.number_val.to_price(avg_fraction_len), avg_fraction_len),
			);

			let max_price = checked_struct.number_val.to_price(avg_fraction_len).max(avg_price_val);
			let min_price = checked_struct.number_val.to_price(avg_fraction_len).min(avg_price_val);
			// println!("max_price={}, min_price={}", max_price, min_price);
			let diff = max_price - min_price;
			// println!("diff <= checked_struct.max_offset * avg_price_val = {} <= {}", diff,
			// checked_struct.max_offset * avg_price_val);
			diff <= checked_struct.max_offset * avg_price_val
		});

		let mut per_checkstatus = PreCheckStatus::Prohibit;
		if check_result {
			per_checkstatus = PreCheckStatus::Pass;
		}

		// make pre check log.
		let pre_check_log = PreCheckCompareLog {
			chain_avg_price_list,
			validator_up_price_list,
			raw_precheck_list: pre_check_list.clone(),
		};
		<FinalPerCheckResult<T>>::insert(stash.clone(), Some((bn, per_checkstatus.clone(), Some(pre_check_log))));
		let mut task_list = <PreCheckTaskList<T>>::get();
		task_list.retain(|(old_acc, _, _)| &stash != old_acc);
		<PreCheckTaskList<T>>::put(task_list);

		per_checkstatus.clone()
	}

	//
	fn get_pre_check_status(stash: T::AccountId) -> Option<(T::BlockNumber, PreCheckStatus)> {
		if let Some((bn, check_status, _)) = <FinalPerCheckResult<T>>::get(stash).unwrap_or(None) {
			return Some((bn, check_status));
		}
		None
	}

	fn clean_pre_check_status(stash: T::AccountId) {
		<FinalPerCheckResult<T>>::remove(stash);
	}

	//
	fn create_pre_check_task(stash: T::AccountId, auth: T::AuthorityAres, bn: T::BlockNumber) -> bool {
		let mut task_list = <PreCheckTaskList<T>>::get();
		let mut is_exists = false;
		task_list.retain(|(old_acc, old_auth, _)| {
			if &stash != old_acc {
				return true;
			} else {
				if &auth == old_auth {
					is_exists = true;
				}
			}
			false
		});
		if is_exists {
			return false;
		}

		task_list.push((stash.clone(), auth, bn));
		<PreCheckTaskList<T>>::put(task_list);
		<FinalPerCheckResult<T>>::insert(
			stash.clone(),
			Some((bn, PreCheckStatus::Review, Option::<PreCheckCompareLog>::None)),
		);
		true
	}
}

impl<T: Config> ValidatorCount for Pallet<T> {
	fn get_validators_count() -> u64 {
		<Pallet<T>>::authorities().len() as u64
	}
}
