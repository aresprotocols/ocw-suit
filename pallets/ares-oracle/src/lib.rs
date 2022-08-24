//! Oracle's main module, responsible for Oracle data submission,
//! Offchain information verification,
//! paid price and other functions.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::{
	self as system,
	offchain::{AppCrypto, CreateSignedTransaction, SignedPayload, SigningTypes},
};

use ares_oracle_provider_support::crypto::sr25519::AuthorityId;
use codec::{Decode, Encode};
use core::fmt;
use frame_support::sp_runtime::sp_std::convert::TryInto;
use frame_support::sp_runtime::traits::{TrailingZeroInput};
use frame_support::traits::{Get, OneSessionHandler};
use frame_system::offchain::{SendUnsignedTransaction, Signer};
use lite_json::json::JsonValue;
use lite_json::NumberValue;
pub use pallet::*;
use serde::{Deserialize, Deserializer};
// use sp_application_crypto::sp_core::crypto::UncheckedFrom;
use sp_consensus_aura::AURA_ENGINE_ID;
use sp_runtime::offchain::storage::StorageValueRef;
use sp_runtime::{
	offchain::{http, Duration},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeAppPublic, RuntimeDebug,
};
use sp_std::vec::Vec;
use sp_std::{prelude::*, str};
// use pallet_authorship;

use crate::traits::*;
use bound_vec_helper::BoundVecHelper;

use ares_oracle_provider_support::{
	IAresOraclePreCheck,
	JsonNumberValue,
	LOCAL_HOST_KEY,
	LOCAL_STORAGE_PRICE_REQUEST_DOMAIN,
	// LOCAL_STORAGE_PRICE_REQUEST_MAKE_POOL,
	// LOCAL_STORAGE_PRICE_REQUEST_LIST,
	PreCheckList, PreCheckStatus, PreCheckStruct, PreCheckTaskConfig, PriceKey, PriceToken, RawSourceKeys, RequestKeys};
use frame_support::pallet_prelude::{PhantomData, StorageMap};
use frame_support::sp_runtime::{Percent};
// use frame_support::storage::bounded_btree_map::BoundedBTreeMap;
use frame_support::weights::Weight;
use frame_support::{BoundedVec};
// use hex;
use oracle_finance::traits::{IForPrice, IForReporter, IForBase};
use oracle_finance::types::{BalanceOf, PurchaseId};
// use sp_runtime::app_crypto::Public;
use sp_runtime::traits::{Saturating, UniqueSaturatedInto, Zero};
// use sp_std::collections::btree_map::BTreeMap;
use types::*;


/// Modules that provide interface definitions.
pub mod traits;
/// Defines the underlying data types required by the Pallet but does not include storage.
pub mod types;
/// A module that implements [`EventHandler`](pallet_authorship::EventHandler) to record some block authors.
pub mod author_events;
/// Handling ares-authority account.
pub mod authority_helper;
/// Utility trait to be implemented on payloads that can be signed.
pub mod ares_crypto;
/// For config of Oracle related storage
pub mod config_helper;
/// For Offchain-http handler functions.
pub mod http_helper;
/// For Jump-block handler functions.
pub mod jumpblock_helper;
/// Provides [`AresOracleFilter`](offchain_filter::AresOracleFilter) structure to determine whether the offchain transaction in the transaction pool is valid.
pub mod offchain_filter;
/// For Offchain-payload handler functions.
pub mod offchain_payload_helper;
/// For Offchain-storage handler functions
pub mod offchain_storage_helper;
/// For price related functions.
pub mod pricer_helper;
/// Check offchain unsigned call.
pub mod validate_unsigned;

pub mod migrations;


/// Compute price using `Average` on price pool.
pub const CALCULATION_KIND_AVERAGE: u8 = 1;
/// Compute price using `Median` on price pool.
pub const CALCULATION_KIND_MEDIAN: u8 = 2;

const DEBUG_TARGET: &str = "pallet::ocw";
const ERROR_MAX_LENGTH_TARGET: &str = "ares_oracle::max_length";
const ERROR_MAX_LENGTH_DESC: &str = "❗ MaxEncodedLen convert error.";

/// The type of result returned when all nodes participate in the ask price after the quotation is generated.
pub const PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE: u8 = 1;
/// The type of result returned when the threshold is reached but not all nodes participate in the work.
pub const PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE: u8 = 2;



#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use ares_oracle_provider_support::{IAresOraclePreCheck, MaximumPoolSize, PreCheckTaskConfig, PriceKey, PriceToken, RequestKeys, TokenList};
	use frame_support::pallet_prelude::*;
	// use frame_support::sp_runtime::traits::{IdentifyAccount, IsMember};
	use frame_support::traits::Len;
	use frame_system::offchain::SubmitTransaction;
	use frame_system::pallet_prelude::{BlockNumberFor, OriginFor};
	use frame_system::{ensure_none, ensure_signed};
	use oracle_finance::traits::*;
	use oracle_finance::types::{BalanceOf, OcwPaymentResult, PurchaseId, EraIndex};
	// use sp_consensus_aura::ConsensusLog::AuthoritiesChange;
	use sp_core::crypto::UncheckedFrom;
	use staking_extend::IStakingNpos;

	#[pallet::error]
	#[derive(PartialEq, Eq)]
	pub enum Error<T> {
		/// The value range of the ask price threshold value must be 1~100 .
		SubmitThresholdRangeError,
		/// Waiting for delay block must be greater than 0 after ask priced.
		DruationNumberNotBeZero,
		/// Insufficient balance when the ask price to pay.
		InsufficientBalance,
		/// The `MaxFee` of the ask price payment limit is too low.
		InsufficientMaxFee,
		/// No available `Trading pairs` are found.
		NoPricePairsAvailable,
		/// Occurred while requesting payment, please check if `Balance` is sufficient.
		PayToPurchaseFeeFailed,
		/// The validator `pre-check task` already exists, no need to resubmit.
		PerCheckTaskAlreadyExists,
		/// The list of pre-checked token pairs cannot be empty.
		PreCheckTokenListNotEmpty,
		/// The purchase ID was not found in the request pool.
		PurchaseIdNotFoundInThePool,
		/// Block author not found.
		BlockAuthorNotFound,
		/// BoundedVec exceeded its maximum length
		BoundedVecExceededMaxLength,
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
			+ Ord
			+ MaybeSerializeDeserialize
			+ UncheckedFrom<[u8; 32]>
			+ MaxEncodedLen
			+ From<AuthorityId>;

		// type FindAuthor: FindAuthor<u32>;

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
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
	where
		<T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
		<T as frame_system::Config>::AccountId: From<sp_application_crypto::sr25519::Public>,
	{
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			Self::check_and_clean_obsolete_task(14400u32.into())
				+
			Self::check_and_clean_hostkey_list(14400u32.into())
		}

		// fn on_finalize(_: T::BlockNumber) {
		// 	log::debug!(
		// 		target: "runtime::system",
		// 		"finalize() blocknumber = [{:?}]",
		// 		Self::block_number(),
		// 	);
		// }

		// fn on_runtime_upgrade() -> frame_support::weights::Weight {
		// 	// To runtime v107
		// 	if StorageVersion::<T>::get() == Releases::V1_0_0_Ancestral
		// 		|| StorageVersion::<T>::get() == Releases::V1_0_1_HttpErrUpgrade
		// 		|| StorageVersion::<T>::get() == Releases::V1_1_0_HttpErrUpgrade
		// 	{
		// 		return migrations::v1_2_0::migrate::<T>();
		// 	}
		// 	T::DbWeight::get().reads(1)
		// }

		fn offchain_worker(block_number: T::BlockNumber) {
			let control_setting = <OcwControlSetting<T>>::get();
			let block_author = Self::get_block_author();

			// check xray
			if block_number > Zero::zero() && block_number % 20u32.into() == Zero::zero() {
				// Check not be validator.
				let current_validator = Authorities::<T>::get().unwrap_or(Default::default());
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
				// Get all ares authoritys.
				let authority_list = T::AuthorityAres::all();
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

					let authority_list_res = AuthorityAresVec::<T::AuthorityAres>::try_create_on_vec(authority_list);
					if let Ok(authority_list) = authority_list_res {
						// LocalXRay::<T>::put(host_key, (request_domain, authority_list));
						let network_is_validator = sp_io::offchain::is_validator();
						let call = Call::submit_local_xray {
							host_key,
							request_domain,
							authority_list,
							network_is_validator,
						};
						let res = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());
						if res.is_err() {
							log::error!( target: DEBUG_TARGET, "tx submit_unsigned_transaction failed." );
						}
					}else{
						log::error!( target: DEBUG_TARGET, "{}, on {}", ERROR_MAX_LENGTH_DESC, "authority_list" );
					}
				}
			}

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
			}

			match block_author {
				None => {
					log::warn!(target: DEBUG_TARGET, "❗ Not found author.");
				}
				Some(author) => {

					if control_setting.open_free_price_reporter {
						log::debug!("🚅 ❗ ⛔ Ocw offchain start {:?} offchain_bn={:?}, system_bn={:?} ", &author, &block_number, &<system::Pallet<T>>::block_number());
						if Self::check_block_author_and_sotre_key_the_same(&author) {
							// For debug.
							// log::debug!("🚅 @ Ares call [0] find = {:?}, authorship = {:?}.", &author, <pallet_authorship::Pallet<T>>::author());
							log::debug!("🚅 @ Ares call [1] ares-price-worker.");
							// Try to get ares price.
							match Self::ares_price_worker(block_number.clone(), author.clone()) {
								Ok(_v) => log::debug!("🚅 @ Ares OCW price acquisition completed."),
								Err(e) => log::warn!(
									target: DEBUG_TARGET,
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
											target: DEBUG_TARGET,
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
													target: DEBUG_TARGET,
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
									target: DEBUG_TARGET,
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
						Ok(_v) => log::debug!("🚅 ~ Ares OCW purchased price acquisition completed."),
						Err(e) => log::warn!(
							target: DEBUG_TARGET,
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
					task_at,
				) {
					Ok(_) => {}
					Err(e) => {
						log::warn!(
							target: DEBUG_TARGET,
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

		/// An offline method that submits and saves the local publicly data by the validator node to the chain for debugging.
		///
		/// The dispatch origin fo this call must be __none__.
		///
		/// - host_key: A random `u32` for a node
		///	- request_domain: The warehouse parameter currently set by the node.
		///	- authority_list: List of ares-authority public keys stored locally
		///	- network_is_validator: Whether the node validator
		#[pallet::weight(0)]
		pub fn submit_local_xray(
			origin: OriginFor<T>,
			host_key: u32,
			request_domain: RequestBaseVecU8,
			authority_list: AuthorityAresVec<T::AuthorityAres>,
			network_is_validator: bool,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			LocalXRay::<T>::insert(
				host_key,
				(<system::Pallet<T>>::block_number(), request_domain, authority_list, network_is_validator),
			);
			Ok(().into())
		}


		/// Submit an ares-price request, The request is processed by all online validators,
		/// and the aggregated result is returned immediately.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - max_fee: The highest asking fee accepted by the signer.
		/// - request_keys: A list of `Trading pairs`, separated by commas if multiple, such as: `eth-usdt, dot-sudt`, etc.
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

			// Filter out unsupported `Trading pairs`.
			let request_keys = raw_source_keys.into_iter().map(|(price_key, _, _, _)|{
				price_key
			}).collect::<Vec<_>>();

			let request_keys = RequestKeys::try_create_on_vec(request_keys);
			if request_keys.is_err() {
				Self::deposit_event(Event::ToBeConvertedError {
					to_be: DataTipVec::create_on_vec("request_keys".as_bytes().to_vec()),
					size: request_keys.len() as u32
				});
				return Ok(().into());
			}

			let request_keys = request_keys.unwrap();

			ensure!(!request_keys.is_empty(), Error::<T>::NoPricePairsAvailable);

			let offer = T::OracleFinanceHandler::calculate_fee_of_ask_quantity(request_keys.len() as u32);
			if offer > max_fee {
				return Err(Error::<T>::InsufficientMaxFee.into());
			}

			let payment_result: OcwPaymentResult<BalanceOf<T>, PurchaseId> = T::OracleFinanceHandler::reserve_for_ask_quantity(
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
					Self::ask_price(who, balance, submit_threshold, max_duration, purchase_id, request_keys)?;
				}
			}
			Ok(().into())
		}

		/// An offline method that submits and saves the purchase-result data.
		/// If the count of validator submitted by `purchase-id` already
		/// reached the threshold requirements, then average price will be
		/// aggregation and mark [PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE](PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE)
		///
		/// The dispatch origin fo this call must be __none__.
		///
		/// -- price_payload: Submitted data.
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
					let agg_count = Self::update_purchase_avg_price_storage(x.clone(), PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE);
					// update report work point
					let _res = Self::update_reporter_point(x.clone(), agg_count);
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
							target: DEBUG_TARGET,
							"⛔️ T::OracleFinanceHandler::unreserve_ask() had an error!"
						);
						Self::deposit_event(Event::ProblemWithRefund);
					}
				}
				false
			});

			Ok(().into())
		}

		/// An offline method that submits and saves the purchase-result data.
		/// If all validators submit price-result, then average price will be
		/// aggregation and mark [PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE](PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE)
		///
		/// The dispatch origin fo this call must be __none__.
		///
		/// -- price_payload: Submitted data.
		#[pallet::weight(0)]
		pub fn submit_purchased_price_unsigned_with_signed_payload(
			origin: OriginFor<T>,
			price_payload: PurchasedPricePayload<T::Public, T::BlockNumber, T::AuthorityAres>,
			_signature: OffchainSignature<T>,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;

			log::debug!(
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
					PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE,
				);
				// update report work point
				let res = Self::update_reporter_point(price_payload.purchase_id.clone(), agg_count);
				if res.is_err() {
					log::error!(
						target: DEBUG_TARGET,
						"⛔️ tx submit_purchased_price_unsigned_with_signed_payload failed."
					);
				}

				// clean order pool
				Self::purchased_storage_clean(price_payload.purchase_id.clone());
			}
			Ok(().into())
		}

		/// An offline method that submits and saves the free ares-price results.
		///
		/// The dispatch origin fo this call must be __none__.
		///
		/// - price_payload: Ares-price data to be uploaded.
		#[pallet::weight(0)]
		pub fn submit_price_unsigned_with_signed_payload(
			origin: OriginFor<T>,
			price_payload: PricePayload<T::Public, T::BlockNumber, T::AuthorityAres>,
			_signature: OffchainSignature<T>,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			log::debug!(
				"# Pre submit price payload on block {:?}/{:?}",
				&price_payload.block_number,
				<system::Pallet<T>>::block_number(),
			);

			let stash_id_opt = Self::get_stash_id(&price_payload.auth);
			ensure!(
				stash_id_opt.is_some(),
				Error::<T>::BlockAuthorNotFound
			);
			let stash_id = stash_id_opt.unwrap();

			// remove author trace data
			ensure!(
				Self::remove_block_author_on_trace(&stash_id, &price_payload.block_number),
				Error::<T>::BlockAuthorNotFound
			);

			let mut block_author_trace = BlockAuthorTrace::<T>::get().unwrap_or(Default::default());
			let mut pop_arr = AuthorTraceData::<T::AccountId, T::BlockNumber>::default();

			while block_author_trace.len() > 0 && pop_arr.len() < 5 {
				let _res = pop_arr.try_push(block_author_trace.pop().unwrap());
			}
			log::debug!(
				"# Pop block-author {:?}",
				&pop_arr,
			);

			// Nodes with the right to increase prices
			let price_list = price_payload.price; // price_list: Vec<(PriceKey, u32)>,
			let mut price_key_list = Vec::new();
			let mut agg_result_list: Vec<(PriceKey, u64, FractionLength, Vec<(T::AccountId, T::BlockNumber)>, T::BlockNumber)> = Vec::new();
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
					price_payload.block_number.clone(),
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

			// // update last author
			// Self::update_last_price_list_for_author (
			// 	price_key_list,
			// 	Self::get_stash_id(&price_payload.auth).unwrap(),
			// 	price_payload.block_number.clone(),
			// );

			// Set jump block
			let jump_block = price_payload.jump_block;
			if jump_block.len() > 0 {
				for PricePayloadSubJumpBlock(price_key, interval) in jump_block.clone() {
					Self::increase_jump_block_number(price_key, interval as u64);
				}
			}

			log::debug!("🚅 Submit jump block list on chain, count = {:?}", jump_block.len());

			Ok(().into())
		}

		/// Submit a pre-check task.
		/// When a new validator is elected, a pre_check_task task will be submitted through this method
		/// within a specific period.
		///
		/// This task is used to ensure that the ares-price response function of the validator node can be used normally.
		///
		/// The dispatch origin fo this call must be __none__.
		///
		/// - precheck_payload: Pre-Check task data, including validators and their authority account data.
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
			// ensure!(result, Error::<T>::PerCheckTaskAlreadyExists);
			if result {
				Self::deposit_event(Event::NewPreCheckTask {
					who: precheck_payload.pre_check_stash,
					authority: precheck_payload.pre_check_auth,
					block: precheck_payload.block_number,
				});
			}

			Ok(().into())
		}

		/// When the validator responds to the pre-check task, the pre-check result data is submitted to the chain.
		/// If approved, it will be passed in the next election cycle, not immediately.
		///
		/// The dispatch origin fo this call must be __none__.
		///
		/// - preresult_payload: Review response result data, which will be compared on-chain.
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
				preresult_payload.pre_check_auth.clone()
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

		/// When there is an error in the offchain http request, the error data will be submitted to the chain through this Call
		///
		/// The dispatch origin fo this call must be __none__.
		///
		/// - err_payload: Http err data.
		#[pallet::weight(0)]
		pub fn submit_offchain_http_err_trace_result(
			origin: OriginFor<T>,
			err_payload: HttpErrTracePayload<T::Public, T::BlockNumber, T::AuthorityAres, T::AccountId>,
			_signature: OffchainSignature<T>,
		) -> DispatchResult {
			ensure_none(origin)?;
			let mut http_err = <HttpErrTraceLogV1<T>>::get().unwrap_or(Default::default());
			if http_err.len() as u32 > T::ErrLogPoolDepth::get() {
				http_err.remove(0usize);
			}
			let _res = http_err.try_push(err_payload.trace_data);
			<HttpErrTraceLogV1<T>>::put(http_err);
			Ok(().into())
		}


		/// Updating the purchase-related parameter settings requires the `Technical-Committee` signature to execute.
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - submit_threshold: The threshold for aggregation is a percentage
		/// - max_duration: Maximum delay to wait for full node response.
		/// - avg_keep_duration: Maximum length to keep aggregated results.
		#[pallet::weight(0)]
		pub fn update_purchased_param(
			origin: OriginFor<T>,
			// submit_threshold: u8,
			submit_threshold: Percent,
			max_duration: u64,
			avg_keep_duration: u64,
		) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			ensure!(
				submit_threshold > Zero::zero(),
				Error::<T>::SubmitThresholdRangeError
			);
			ensure!(max_duration > 0, Error::<T>::DruationNumberNotBeZero);
			let setting_data = PurchasedDefaultData::new(submit_threshold, max_duration, avg_keep_duration);
			<PurchasedDefaultSetting<T>>::put(setting_data.clone());
			Self::deposit_event(Event::UpdatePurchasedDefaultSetting { setting: setting_data });
			Ok(().into())
		}

		/// Updating the ConfDataSubmissionInterval parameter settings requires the `Technical-Committee` signature to execute.
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - interval: Maximum allowed interval
		#[pallet::weight(0)]
		pub fn update_config_of_data_submission_interval(
			origin: OriginFor<T>,
			interval: T::BlockNumber
		) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;

			<ConfDataSubmissionInterval<T>>::put(interval.clone());
			Self::deposit_event(Event::UpdateDataSubmissionInterval { interval });
			Ok(().into())
		}


		/// Update the control parameters of ares-oracle
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - need_verifier_check: Whether to start the validator checker.
		/// - open_free_price_reporter: Whether the `free-price` moudle is enabled.
		/// - open_paid_price_reporter: Whether the `ask-price` moudle is enabled.
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

		/// Revoke the `key-pair` on the request token list.
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - price_key: A price key, such as `btc-usdt`
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

		/// Modify or add a `key-pair` to the request list.
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - price_key: A price key, such as `btc-usdt`.
		/// - price_token: A price token, such as `btc`.
		/// - parse_version: Parse version, currently only parameter 2 is supported.
		/// - fraction_num: Fractions when parsing numbers.
		/// - request_interval: The interval between validators submitting price on chain.
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
							let _res = prices_request.try_push((
								price_key.clone(),
								price_token.clone(),
								parse_version,
								fraction_num.clone(),
								request_interval.clone(),
							));
							// ensure!(res.is_ok(), Error::<T>::BoundedVecExceededMaxLength);
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
					let _res = prices_request.try_push((
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

		/// Update the value of the `allowable offset` parameter to determine the abnormal range of the submitted price
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - offset: A Percent value.
		#[pallet::weight(0)]
		pub fn update_allowable_offset_propose(origin: OriginFor<T>, offset: Percent) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			<PriceAllowableOffset<T>>::put(offset);
			Self::deposit_event(Event::PriceAllowableOffsetUpdate { offset });
			Ok(())
		}

		/// Update the depth of the price pool. When the price pool reaches the maximum value,
		/// the average price will be aggregated and put on the chain.
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - depth: u32 integer
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
					let depth_usize = depth as usize;
					let old_price_list_opt = <AresPrice<T>>::get(price_key.clone());

					if let Some(old_price_list) = old_price_list_opt {
						// check length
						if old_price_list.len() > depth_usize {
							let diff_len = old_price_list.len() - depth_usize;
							let mut new_price_list = AresPriceDataVecOf::<T::AccountId, T::BlockNumber>::default();
							for (index, value) in old_price_list.into_iter().enumerate() {
								if !(index < diff_len) {
									// kick out old value.
									let _res = new_price_list.try_push(value);
								}
							}
							// Reduce the depth.
							<AresPrice<T>>::insert(price_key.clone(), new_price_list);
						}
					}

					// need to recalculate the average.
					Self::check_and_update_avg_price_storage(price_key, depth);
					false
				});
			}
			Ok(())
		}

		/// Update the pre-checked `Trading pairs` list for checking the validator price feature.
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - token_list: [`PriceToken`](ares_oracle_provider_support::PriceToken)
		#[pallet::weight(0)]
		pub fn update_pre_check_token_list(origin: OriginFor<T>, token_list: TokenList) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			ensure!(token_list.len() > 0, Error::<T>::PreCheckTokenListNotEmpty);
			<ConfPreCheckTokenList<T>>::put(token_list);
			Ok(().into())
		}

		/// `session-multi` indicates the trigger `pre-check` session period before the era.
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - multi: integer
		#[pallet::weight(0)]
		pub fn update_pre_check_session_multi(origin: OriginFor<T>, multi: T::BlockNumber) -> DispatchResult {
			T::RequestOrigin::ensure_origin(origin)?;
			<ConfPreCheckSessionMulti<T>>::put(multi);
			Ok(().into())
		}

		/// The maximum offset allowed for `pre-check` when validation.
		///
		/// The dispatch origin for this call must be _Signed_ of Technical-Committee.
		///
		/// - offset: Percent
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
		AggregatedPrice {
			results: Vec<(PriceKey, u64, FractionLength, Vec<(T::AccountId, T::BlockNumber)>, T::BlockNumber)>,
		},
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
		UpdateDataSubmissionInterval {
			interval: T::BlockNumber,
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
			offset: Percent,
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
		ToBeConvertedError {
			to_be: DataTipVec,
			size: u32,
		},
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit_price_unsigned_with_signed_payload {
				price_payload: ref payload,
				ref signature,
			} = call
			{

				let mut block_author_trace = BlockAuthorTrace::<T>::get().unwrap_or(Default::default());
				let mut pop_arr = AuthorTraceData::<T::AccountId, T::BlockNumber>::default();

				while block_author_trace.len() > 0 && pop_arr.len() < 5 {
					let _res = pop_arr.try_push(block_author_trace.pop().unwrap());
				}

				log::debug!(
					"🚅 Validate price payload data, on block: {:?}/{:?}, author: {:?}, pop_arr: {:?}",
					payload.block_number.clone(), // 236
					<system::Pallet<T>>::block_number(), // 237 (next)
					payload.public.clone(),
					pop_arr,
				);

				// if !Self::is_validator_member(&payload.auth) {
				// 	log::error!(
				// 		target: DEBUG_TARGET,
				// 		"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on price "
				// 	);
				// 	return InvalidTransaction::BadProof.into();
				// }

				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
				if !signature_valid {
					log::error!(
						target: DEBUG_TARGET,
						"⛔️ Signature invalid. `InvalidTransaction` on price"
					);
					return InvalidTransaction::BadProof.into();
				}

				let author_trace = BlockAuthorTrace::<T>::get();
				if author_trace.is_none() {
					return InvalidTransaction::BadProof.into();
				}

				let stash_id = Self::get_stash_id(&payload.auth);
				let stash_id = stash_id.unwrap();

				let author_trace = author_trace.unwrap();
				if !author_trace.iter().any(|(block_author, block_number)|{
					&stash_id == block_author
					&& &payload.block_number == block_number
				}) {
					log::error!(
						target: DEBUG_TARGET,
						"⛔️ Payload block_author error on author-trace. `InvalidTransaction` on price"
					);
					return InvalidTransaction::BadProof.into();
				}

				let pairs_list:PricePayloadSubPriceList = payload.price.clone();
				if !Self::submission_interval_check(pairs_list, payload.block_number.clone()) {
					log::error!(
						target: DEBUG_TARGET,
						"⛔️ Payload block_author error on submission-check. `InvalidTransaction` on price"
					);
					return InvalidTransaction::BadProof.into();
				}

				Self::validate_transaction_parameters_of_ares(&payload.block_number, &payload.auth)
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
						target: DEBUG_TARGET,
						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on purchased price"
					);
					return InvalidTransaction::BadProof.into();
				}

				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());

				if !signature_valid {
					log::error!(
						target: DEBUG_TARGET,
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
						target: DEBUG_TARGET,
						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on force clear purchased"
					);
					return InvalidTransaction::BadProof.into();
				}
				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
				if !signature_valid {
					log::error!(
						target: DEBUG_TARGET,
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
						target: DEBUG_TARGET,
						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on price "
					);
					return InvalidTransaction::BadProof.into();
				}
				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
				if !signature_valid {
					log::error!(
						target: DEBUG_TARGET,
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
						target: DEBUG_TARGET,
						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on purchased price"
					);
					return InvalidTransaction::BadProof.into();
				}

				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());

				if !signature_valid {
					log::error!(
						target: DEBUG_TARGET,
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
							target: DEBUG_TARGET,
							"⛔️ Stash account is inconsistent!"
						);
						return InvalidTransaction::BadProof.into();
					}
				} else {
					log::error!(
						target: DEBUG_TARGET,
						"⛔️ Could not find the pre-check task!"
					);
					return InvalidTransaction::BadProof.into();
				}
				//
				let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
				if !signature_valid {
					log::error!(
						target: DEBUG_TARGET,
						"⛔️ Signature invalid. `InvalidTransaction` on price"
					);
					return InvalidTransaction::BadProof.into();
				}
				ValidTransaction::with_tag_prefix("ares-oracle::submit_offchain_pre_check_result")
					.priority(T::UnsignedPriority::get())
					.and_provides(&payload.block_number) // next_unsigned_at
					// .and_provides(payload.public.clone()) // next_unsigned_at
					.longevity(5)
					.propagate(true)
					.build()
			} else if let Call::submit_local_xray {
				ref host_key,
				ref request_domain,
				ref authority_list,
				ref network_is_validator,
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

	// #[pallet::storage]
	// #[pallet::getter(fn block_author)]
	// pub(super) type BlockAuthor<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn block_author)]
	pub(super) type BlockAuthor<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Save recent block authors for filtering offchain requests submitted by non-block producers.
	pub type AuthorTraceData<AccountId, BlockNumber> = BoundedVec<(AccountId, BlockNumber), MaximumLogsSize> ;

	#[pallet::storage]
	#[pallet::getter(fn block_author_trace)]
	pub(super) type BlockAuthorTrace<T: Config> = StorageValue<_, AuthorTraceData<T::AccountId, T::BlockNumber>, OptionQuery>;

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
		OptionQuery,
	>;

	/// Pool for storing ask-price result data.
	pub type PurchasedPriceDataVec<AccountId, BlockNumber> = BoundedVec<AresPriceData<AccountId, BlockNumber>, MaximumPoolSize>;

	// migrated
	#[pallet::storage]
	#[pallet::getter(fn purchased_price_pool)]
	pub(super) type PurchasedPricePool<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PurchaseId, // purchased_id,
		Blake2_128Concat,
		PriceKey, // price_key,
		PurchasedPriceDataVec<T::AccountId, T::BlockNumber>,
		OptionQuery,
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
		OptionQuery,
	>;

	// migrated
	#[pallet::storage]
	#[pallet::getter(fn last_price_author)]
	pub(super) type LastPriceAuthor<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		PriceKey, // price_key
		(T::AccountId, T::BlockNumber),
		OptionQuery,
	>;

	/// The `Ares-price` pool structure, actually associated with [`AresPriceDataVec`](types::AresPriceDataVec)
	pub type AresPriceDataVecOf<AccountId, BlockNumber> = AresPriceDataVec<AccountId, BlockNumber>;
	// migrated
	/// The lookup table for names.
	#[pallet::storage]
	#[pallet::getter(fn ares_prices)]
	pub(super) type AresPrice<T: Config> =
		StorageMap<
			_,
			Blake2_128Concat,
			PriceKey,
			// Vec<AresPriceData<T::AccountId, T::BlockNumber>>,
			AresPriceDataVecOf<T::AccountId, T::BlockNumber>,
			OptionQuery
		>;

	/// Store `ares-price` data that deviates too much from the average price.
	pub type AbnormalPriceDataVec<AccountId, BlockNumber> = BoundedVec<(AresPriceData<AccountId, BlockNumber>, AvgPriceData), MaximumPoolSize>;

	// migrated
	/// The lookup table for names.
	#[pallet::storage]
	#[pallet::getter(fn ares_abnormal_prices)]
	pub(super) type AresAbnormalPrice<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		PriceKey,
		// Vec<(AresPriceData<T::AccountId, T::BlockNumber>, AvgPriceData)>,
		AbnormalPriceDataVec<T::AccountId, T::BlockNumber>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ares_avg_prices)]
	pub(super) type AresAvgPrice<T: Config> =
		StorageMap<_, Blake2_128Concat, PriceKey, (u64, FractionLength, T::BlockNumber), OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn price_pool_depth)]
	pub(super) type PricePoolDepth<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn price_allowable_offset)]
	pub(super) type PriceAllowableOffset<T: Config> = StorageValue<_, Percent, ValueQuery>;

	/// Stores a list of data defined by `Trading pairs`.
	pub type PricesRequestsVec = BoundedVec<(
		PriceKey,   // price key
		PriceToken, // price token
		u32,        // parse version number.
		FractionLength,
		RequestInterval,
	), MaximumPoolSize>;


	#[pallet::storage]
	#[pallet::getter(fn prices_requests)]
	pub(super) type PricesRequests<T: Config> = StorageValue<
		_,
		PricesRequestsVec,
		ValueQuery,
	>;

	/// For BoundedVec max length.
	pub type MaximumRequestBaseSize = ConstU32<500>;
	/// Store data read from werehouse.
	pub type RequestBaseVecU8 = BoundedVec<u8, MaximumRequestBaseSize>;

	#[pallet::storage]
	#[pallet::getter(fn request_base_onchain)]
	pub(super) type RequestBaseOnchain<T: Config> = StorageValue<_, RequestBaseVecU8, ValueQuery>;

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
		Option<(T::BlockNumber, PreCheckStatus, Option<PreCheckCompareLog>, T::AuthorityAres)>,
	>;

	/// Stores a list of pre-check tasks.
	pub type PreCheckTaskListVec<AccountId, AuthorityAres, BlockNumber> = BoundedVec<(
		AccountId,
		AuthorityAres,
		BlockNumber
	), MaximumPoolSize>;

	// PreCheckTaskList
	#[pallet::storage]
	#[pallet::getter(fn per_check_task_list)]
	pub(super) type PreCheckTaskList<T: Config> =
		StorageValue<_,
			// Vec<(T::AccountId, T::AuthorityAres, T::BlockNumber)>,
			PreCheckTaskListVec<T::AccountId, T::AuthorityAres, T::BlockNumber>,
			OptionQuery
		>;

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
	#[pallet::getter(fn conf_data_submission_interval)]
	pub(super) type ConfDataSubmissionInterval<T: Config> = StorageValue<
		_,
		T::BlockNumber, //
		ValueQuery,
	>;

	/// For BoundedVec max length.
	pub type MaximumLogsSize = ConstU32<5000>;
	/// BoundedVec to storage [`HttpErrTraceData`](`types::HttpErrTraceData`)
	pub type HttpErrTraceVec<AccountId, BlockNumber> = BoundedVec<HttpErrTraceData<BlockNumber, AccountId>, MaximumLogsSize>;

	#[pallet::storage]
	#[pallet::getter(fn http_err_trace_log)]
	pub(super) type HttpErrTraceLogV1<T: Config> =
		StorageValue<_,
			// Vec<HttpErrTraceData<T::BlockNumber, T::AccountId>>,
			HttpErrTraceVec<T::AccountId, T::BlockNumber>,
			OptionQuery
		>;

	#[pallet::storage]
	pub(crate) type StorageVersion<T: Config> = StorageValue<_, Releases, ValueQuery>;


	/// For BoundedVec max length.
	pub type MaximumIdenSize = ConstU32<50>;
	/// BoundedVec to store Ares-Authorities
	pub type AuthorityAresVec<AuthorityAres> = BoundedVec<AuthorityAres, MaximumPoolSize>;

	#[pallet::storage]
	pub(crate) type LocalXRay<T: Config> =
		StorageMap<
			_,
			Blake2_128Concat,
			u32,
			(
				T::BlockNumber,
				// Vec<u8>,
				RequestBaseVecU8,
				// Vec<T::AuthorityAres>,
				AuthorityAresVec<T::AuthorityAres>,
				// network_is_validator
				bool,
			)
		>;

	/// List of stash and authority
	pub type StashAndAuthorityVec<AccountId, AuthorityAres> = BoundedVec<(AccountId, AuthorityAres), MaximumPoolSize>;

	/// The current authority set.
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub(super) type Authorities<T: Config> = StorageValue<
		_,
		// Vec<(T::AccountId, T::AuthorityAres)>,
		StashAndAuthorityVec<T::AccountId, T::AuthorityAres>,
		OptionQuery
	>;

	#[pallet::storage]
	#[pallet::getter(fn price_counter)]
	pub(super) type PriceCounter<T: Config> = StorageMap<_, Blake2_128Concat, PriceKey, u32>;

	// AggregatedMission
	#[pallet::storage]
	#[pallet::getter(fn aggregated_mission)]
	pub(super) type AggregatedMission<T: Config> = StorageMap<_, Blake2_128Concat, T::AuthorityAres, (PriceKey, T::BlockNumber, bool)>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub _phantom: sp_std::marker::PhantomData<T>,
		pub request_base: Vec<u8>,
		pub price_allowable_offset: Percent,
		pub price_pool_depth: u32,
		pub price_requests: Vec<(Vec<u8>, Vec<u8>, u32, FractionLength, RequestInterval)>,
		pub authorities: Vec<(T::AccountId, T::AuthorityAres)>,
		pub data_submission_interval: u32,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig {
				_phantom: Default::default(),
				request_base: Default::default(),
				price_allowable_offset: Percent::from_percent(10),
				price_pool_depth: 10u32,
				price_requests: Default::default(),
				authorities: Default::default(),
				data_submission_interval: 100u32.into()
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			if !self.price_requests.is_empty() {
				let price_request_list = self.price_requests.clone().into_iter().map(|(price_key, price_token,version, fraction_length, request_interval )| {
					(
						PriceKey::create_on_vec(price_key),
						PriceToken::create_on_vec(price_token),
						version,
						fraction_length,
						request_interval,
					)
				}).collect::<Vec<_>>() ;
				PricesRequests::<T>::put(PricesRequestsVec::create_on_vec(price_request_list) );
				self.price_requests
					.iter()
					.for_each(|(symbol, _, _, fraction_length, _)| {
						SymbolFraction::<T>::insert( PriceKey::create_on_vec(symbol.clone()) , fraction_length);
					})
			}
			if self.price_pool_depth > 0 {
				PricePoolDepth::<T>::put(&self.price_pool_depth);
			}
			if self.request_base.len() > 0 {
				RequestBaseOnchain::<T>::put(RequestBaseVecU8::create_on_vec(self.request_base.clone()));
			}
			Pallet::<T>::initialize_authorities(StashAndAuthorityVec::<T::AccountId, T::AuthorityAres>::create_on_vec(self.authorities.clone()));
			PriceAllowableOffset::<T>::put(&self.price_allowable_offset);
			// For new vesrion.
			ConfPreCheckAllowableOffset::<T>::put(Percent::from_percent(10));
			let session_multi: T::BlockNumber = 2u32.into();
			ConfPreCheckSessionMulti::<T>::put(session_multi);
			let mut token_list: TokenList = Default::default();
			let _res = token_list.try_push(PriceToken::create_on_vec(b"btc-usdt".to_vec()));
			let _res = token_list.try_push(PriceToken::create_on_vec(b"eth-usdt".to_vec()));
			let _res = token_list.try_push(PriceToken::create_on_vec(b"dot-usdt".to_vec()));
			ConfPreCheckTokenList::<T>::put(token_list);
			// V1_2_0
			StorageVersion::<T>::put(Releases::V1_2_0);
			// submission_interval
			let submission_interval: T::BlockNumber = self.data_submission_interval.clone().into();
			ConfDataSubmissionInterval::<T>::put(submission_interval);
		}
	}
}

impl<T: Config> Pallet<T> {

	pub fn remove_block_author_on_trace(del_stash: &T::AccountId, del_bn: &T::BlockNumber) -> bool {

		let old_trace = <BlockAuthorTrace<T>>::take();
		if let Some(trace_data) = old_trace.clone() {
			for (index, (old_stash, old_bn)) in trace_data.iter().enumerate() {
				if  old_stash == del_stash && old_bn == del_bn {
					if let Some(mut new_data) = old_trace {
						new_data.remove(index);
						BlockAuthorTrace::<T>::put(new_data);
						return true;
					}
				}
			}
		}
		false
	}

	/// Update the new validator set.
	fn change_authorities(new: StashAndAuthorityVec<T::AccountId, T::AuthorityAres>) {
		<Authorities<T>>::put(&new);
	}

	/// Used to initialize the ares-oracle validator data in the genesis configuration.
	fn initialize_authorities(authorities: StashAndAuthorityVec<T::AccountId, T::AuthorityAres>) {
		if !authorities.is_empty() {
			assert!(
				<Authorities<T>>::get().is_none(),
				"Ares Authorities are already initialized!"
			);
			// let authorities: BoundedVec<(T::AccountId, T::AuthorityAres), MaximumAuthorities> =
			// 	authorities.try_into().expect("authorities is too long");
			<Authorities<T>>::put(authorities);
		}
	}

	/// Get the `ares-authority` of the block producer of the current block,
	/// which needs to configure the EventHandler of `pallet_authorship`
	fn get_block_author() -> Option<T::AuthorityAres> {
		if let Some(block_author) = BlockAuthor::<T>::get() {
			return Authorities::<T>::get().map_or(None, |sets| {
				let mut authority_ares = None;
				sets.iter().any(|(acc, authority)| {
					log::debug!("get_block_author acc == &block_author {:?} =? {:?}", acc, &block_author);
					if acc == &block_author {
						authority_ares = Some(authority.clone());
						return true;
					}
					false
				});
				return authority_ares;
			});
		}
		None
	}

	fn keystore_validator_member() -> Option<T::AuthorityAres>
		where <T as frame_system::Config>::AccountId: From<sp_application_crypto::sr25519::Public>,
	{
		let local_keys: Vec<T::AuthorityAres> = Self::get_ares_authority_list();
		for local_auth in local_keys.into_iter() {
			if Self::is_validator_member(&local_auth) {
				return Some(local_auth);
			}
		}
		None
	}

	/// Determining that an authority is a validator
	fn is_validator_member(validator: &T::AuthorityAres) -> bool {
		// let mut find_validator = !T::NeedVerifierCheck::get();
		let find_validator = !<OcwControlSetting<T>>::get().need_verifier_check;
		if find_validator {
			log::warn!(
				target: DEBUG_TARGET,
				"❗ Currently in debug mode because need_verifier_check is set to true."
			);
			return true;
		}
		<Authorities<T>>::get().unwrap_or(Default::default()).iter().any(|(_, auth)| auth == validator)
	}

	/// Determine whether it is aura consensus.
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

	/// Clear the stored data corresponding to a `purchase-id`.
	fn purchased_storage_clean(p_id: PurchaseId) {
		<PurchasedPricePool<T>>::remove_prefix(p_id.clone(), None);
		<PurchasedRequestPool<T>>::remove(p_id.clone());
		<PurchasedOrderPool<T>>::remove_prefix(p_id.clone(), None);
	}

	/// Find those requests that have exceeded the `max_duration` limit,
	/// but are still in the commit state, return `purchase-id` list.
	fn get_expired_purchased_transactions() -> Vec<PurchaseId> {
		let mut purchased_id_list: Vec<PurchaseId> = Vec::default();
		let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();

		<PurchasedRequestPool<T>>::iter().any(|(p_id, p_d)| {
			if current_block >= p_d.max_duration {
				purchased_id_list.push(p_id.clone());
			}
			false
		});
		purchased_id_list
	}



	/// Check and clear old data, the duration of data saving is stored in `PurchasedDefaultSetting<T>`,
	/// you can modify it by the [update_purchased_param()](pallet::update_purchased_param)
	fn check_and_clear_expired_purchased_average_price_storage(
		// purchase_id: PurchaseId,
		current_block_num: u64,
	) -> bool {

		let purchase_setting = <PurchasedDefaultSetting<T>>::get();
		let mut has_remove = false;
		PurchasedAvgTrace::<T>::iter().for_each(|(pid, bn)|{
			let comp_blocknum = purchase_setting.avg_keep_duration.saturating_add(bn.unique_saturated_into());
			if current_block_num > comp_blocknum {
				<PurchasedAvgPrice<T>>::remove_prefix(pid.clone(), None);
				<PurchasedAvgTrace<T>>::remove(pid.clone());
				has_remove = true;
			}
		});

		return has_remove;
	}

	/// Clear on-chain data of a `trading-pair`, including its average price.
	fn clear_price_storage_data(price_key: PriceKey) {
		<AresPrice<T>>::remove(&price_key);
		<AresAvgPrice<T>>::remove(&price_key);
	}

	// Judge whether the predetermined conditions of the validator of the current
	// purchased_id meet the requirements, and return true if it is
	fn is_validator_purchased_threshold_up_on(purchase_id: PurchaseId) -> bool {

		let reporter_count = <PurchasedOrderPool<T>>::iter_prefix_values(purchase_id.clone()).count();
		if 0 == reporter_count {
			return false;
		}

		let purchased_request = <PurchasedRequestPool<T>>::get(purchase_id.clone());
		if purchased_request.is_none() {
			return false;
		}

		let purchased_request = purchased_request.unwrap();

		let validator_count = T::AuthorityCount::get_validators_count();

		let reporter_count = reporter_count as u64;

		// let div_val = (reporter_count * 100) / (validator_count);
		// let submit_threshold = purchased_request.submit_threshold as u64;
		// div_val >= submit_threshold

		let div_val = Percent::from_rational(reporter_count,validator_count);
		div_val >= purchased_request.submit_threshold
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
			return Default::default();
		}
		let purchased_str = purchased_str.unwrap();

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

	/// For offchian validate.
	fn validate_transaction_parameters_of_ares(
		block_number: &T::BlockNumber,
		// _price_list: Vec<(Vec<u8>, u64, FractionLength, JsonNumberValue)>,
		auth: &T::AuthorityAres,
	) -> TransactionValidity {
		// Let's make sure to reject transactions from the future.
		let current_block = <system::Pallet<T>>::block_number();
		if &current_block < block_number {
			return InvalidTransaction::Future.into();
		}

		log::debug!("ValidTransaction::provides source = {:?}, {:?}", block_number, auth);
		let mut provides = block_number.clone().encode();
		provides.append(&mut auth.clone().encode());
		log::debug!("ValidTransaction::provides result = {:?}", &provides);

		ValidTransaction::with_tag_prefix("ares-oracle::validate_transaction_parameters_of_ares")
			.priority(T::UnsignedPriority::get())
			.and_provides(provides) // next_unsigned_at
			.longevity(5)
			.propagate(true)
			.build()
	}

	/// When `ask-price` completes, the function updates the points to the financial system.
	fn update_reporter_point(purchase_id: PurchaseId, agg_count: usize) -> Result<(), Error<T>> {
		let request_mission = PurchasedRequestPool::<T>::get(purchase_id.clone());
		if request_mission.is_none() {
			return Err(Error::<T>::PurchaseIdNotFoundInThePool);
		}
		let request_mission = request_mission.unwrap();

		// transfer balance to pallet account.
		if T::OracleFinanceHandler::pay_to_ask(purchase_id.clone(), agg_count).is_err() {
			return Err(Error::<T>::PayToPurchaseFeeFailed);
		}

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

	/// Check if the blocks submitted by the transaction pair in the list meet the minimum interval requirement.
	/// If not, there may be behaviors using `old-block` attacks and need to be careful.
	fn submission_interval_check(pairs_list: PricePayloadSubPriceList, check_bn: T::BlockNumber) -> bool {
		let price_request_vec = PricesRequests::<T>::get();
		for price_pair in pairs_list {
			let is_ok = price_request_vec.clone().into_iter().any(|(check_price_key, _, _, _, check_interval)|{
				if price_pair.0 == check_price_key {
					// Find out last submit price
					if let Some((_, last_submit_bn)) = Self::get_last_price_author(check_price_key) {
						// Compare whether the current check_bn is greater than or equal to
						// the last submit bn + check_interval of the last quotation.
						return check_bn >= last_submit_bn.saturating_add(check_interval.into());
					}
					return true;
				}
				false
			});
			if !is_ok {
				return false;
			}
		}
		true
	}
}

fn de_string_to_bytes<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	let s: &str = Deserialize::deserialize(de)?;
	Ok(s.as_bytes().to_vec())
}

#[derive(Deserialize, Encode, Decode, Clone, Default)]
struct LocalPriceRequestStorage {
	#[serde(deserialize_with = "de_string_to_bytes")]
	price_key: Vec<u8>,
	#[serde(deserialize_with = "de_string_to_bytes")]
	request_url: Vec<u8>,
	parse_version: u32,
}

impl fmt::Debug for LocalPriceRequestStorage {
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
{
	type Public = T::AuthorityAres;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T>
{
	type Key = T::AuthorityAres;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityAres)>,
	{
		let authorities = validators
			.map(|(stash, auth)| (stash.clone(), auth))
			.collect::<Vec<_>>();
		Self::initialize_authorities(StashAndAuthorityVec::<T::AccountId, T::AuthorityAres>::create_on_vec(authorities));
	}

	fn on_new_session<'a, I: 'a>(changed: bool, validators: I, _queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityAres)>,
	{
		if changed {
			let next_authorities = validators
				.map(|(stash, auth)| (stash.clone(), auth))
				.collect::<Vec<_>>();
			let next_authorities_res = StashAndAuthorityVec::<T::AccountId, T::AuthorityAres>::try_create_on_vec(next_authorities);
			if let Ok(next_authorities) = next_authorities_res {
				let last_authorities = <Authorities<T>>::get().unwrap_or(Default::default());
				if next_authorities != last_authorities {
					Self::change_authorities(next_authorities);
				}
			}else{
				log::error!( target: DEBUG_TARGET, "{}, on {}", ERROR_MAX_LENGTH_DESC, "authority_list" );
			}
		}
	}

	fn on_disabled(_i: u32) {}
}

impl<T: Config> SymbolInfo<T::BlockNumber> for Pallet<T> {
	fn price(symbol: &Vec<u8>) -> Result<(u64, FractionLength, T::BlockNumber), ()> {
		let symbol_res = PriceKey::try_create_on_vec(symbol.clone());
		if let Ok(symbol) = symbol_res {
			return AresAvgPrice::<T>::try_get(symbol);
		}else{
			log::error!( target: DEBUG_TARGET, "{}, on {}", ERROR_MAX_LENGTH_DESC, "SymbolInfo::price" );
		}
		Err(())
	}

	fn fraction(symbol: &Vec<u8>) -> Option<FractionLength> {
		let symbol_res = PriceKey::try_create_on_vec(symbol.clone());
		if let Ok(symbol) = symbol_res {
			return SymbolFraction::<T>::get(symbol);
		}else{
			log::error!( target: DEBUG_TARGET, "{}, on {}", ERROR_MAX_LENGTH_DESC, "SymbolInfo::fraction" );
		}
		None
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
		if <PreCheckTaskList<T>>::get().unwrap_or(Default::default()).len() == 0 {
			return false;
		}
		let task_list = <PreCheckTaskList<T>>::get().unwrap_or(Default::default());
		task_list.iter().any(|(storage_stash, _, _)| &stash == storage_stash)
	}

	//
	fn get_pre_task_by_authority_set(
		auth_list: Vec<T::AuthorityAres>,
	) -> Option<(T::AccountId, T::AuthorityAres, T::BlockNumber)> {
		let task_list = <PreCheckTaskList<T>>::get().unwrap_or(Default::default());
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

	fn check_and_clean_obsolete_task(maximum_due: T::BlockNumber) -> Weight {
		let mut old_task_list = <PreCheckTaskList<T>>::get().unwrap_or(Default::default());
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
			if let Some((bn, _per_status, _, _)) = val {
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

		log::debug!(
			"Pre-check check_token_list = {:?}",
			&check_config.check_token_list,
		);

		raw_price_source_list.retain(|x| {
			check_config.check_token_list.clone().iter().any(|check_price_key| {
				check_price_key == &x.0
			})
		});

		if raw_price_source_list.len() == 0 as usize {
			log::warn!(
					target: DEBUG_TARGET,
					"❗ PricesRequests can not be empty.",
			);
			return PreCheckList::default();
		}

		let format_data_source_list: Vec<(PriceKey, PriceToken, FractionLength)> = raw_price_source_list
			.into_iter()
			.map(|(price_key, parse_key, _version, fraction_len, _request_interval)| {
				(price_key, parse_key, fraction_len)
			})
			.collect();
		//
		let format_data_res = RawSourceKeys::try_create_on_vec(format_data_source_list);
		if let Ok(format_data) = format_data_res {
			// check_config
			let response_result = Self::fetch_bulk_price_with_http(format_data);
			let response_result = response_result.unwrap_or(Vec::new());
			let mut response_result: Vec<Option<PreCheckStruct>> = response_result
				.into_iter()
				.map(|(price_key, _parse_key, _fraction_len, number_val, timestamp)| {
					let number_val = JsonNumberValue::try_new(number_val);
					if number_val.is_none() {
						return None;
					}
					Some(PreCheckStruct {
						price_key,
						number_val: number_val.unwrap(),
						max_offset: check_config.allowable_offset.clone(),
						timestamp,
					})
				})
				.collect();

			// Remove the None elements
			response_result.retain(|x|{
				x.is_some()
			});

			let response_result: Vec<PreCheckStruct> = response_result.into_iter().map(|x|{
				x.unwrap()
			}).collect();

			let response_result: PreCheckList = response_result.try_into().expect("PreCheckList is too long");
			return response_result;
		}

		log::warn!(
				target: DEBUG_TARGET,
				"❗ PreCheckList can not be empty.",
		);
		PreCheckList::default()
	}

	// Record the per check results and add them to the storage structure.
	fn save_pre_check_result(stash: T::AccountId, bn: T::BlockNumber, pre_check_list: PreCheckList, pre_check_auth: T::AuthorityAres) -> PreCheckStatus {
		assert!(pre_check_list.len() > 0, "⛔️ Do not receive empty result check.");
		// get avg price.
		// let mut chain_avg_price_list = BoundedBTreeMap::<PriceKey, (u64, FractionLength),
		// MaximumMapCapacity>::new(); let mut validator_up_price_list = BoundedBTreeMap::<PriceKey, (u64,
		// FractionLength), MaximumMapCapacity>::new();
		let mut chain_avg_price_list =  CompareLogBTreeMap::new(); // BTreeMap::<Vec<u8>, (u64, FractionLength)>::new();
		let mut validator_up_price_list = CompareLogBTreeMap::new(); // BTreeMap::<Vec<u8>, (u64, FractionLength)>::new();

		let mut check_result = pre_check_list.iter().all(|checked_struct| {
			// Check price key exists.
			// if !<AresAvgPrice<T>>::contains_key(&checked_struct.price_key) {
			// 	return false;
			// }
			// Get avg price struct.
			if let Some((avg_price_val, avg_fraction_len, _update_bn)) = <AresAvgPrice<T>>::get(&checked_struct.price_key) {
				let res = chain_avg_price_list.try_insert(checked_struct.price_key.clone(), (avg_price_val, avg_fraction_len));
				if res.is_err() {
					return false;
				}
				let res = validator_up_price_list.try_insert(
					checked_struct.price_key.clone(),
					(checked_struct.number_val.to_price(avg_fraction_len), avg_fraction_len),
				);
				if res.is_err() {
					return false;
				}

				let max_price = checked_struct.number_val.to_price(avg_fraction_len).max(avg_price_val);
				let min_price = checked_struct.number_val.to_price(avg_fraction_len).min(avg_price_val);
				let diff = max_price - min_price;
				// checked_struct.max_offset * avg_price_val);
				return diff <= checked_struct.max_offset * avg_price_val
			} else {
				return false;
			}

		});

		// Check whether it is consistent with the number of the count.
		let token_list = ConfPreCheckTokenList::<T>::get();
		if pre_check_list.len() as u32 == 0 || pre_check_list.len() != token_list.len() {
			check_result = false;
		}

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
		<FinalPerCheckResult<T>>::insert(stash.clone(), Some((bn, per_checkstatus.clone(), Some(pre_check_log), pre_check_auth.clone())));
		let mut task_list = <PreCheckTaskList<T>>::get().unwrap_or(Default::default());
		task_list.retain(|(old_acc, _, _)| &stash != old_acc);
		<PreCheckTaskList<T>>::put(task_list);

		per_checkstatus.clone()
	}

	//
	fn get_pre_check_status(stash: T::AccountId) -> Option<(T::BlockNumber, PreCheckStatus)> {
		if let Some((bn, check_status, _, _)) = <FinalPerCheckResult<T>>::get(stash).unwrap_or(None) {
			return Some((bn, check_status));
		}
		None
	}

	fn clean_pre_check_status(stash: T::AccountId) {
		<FinalPerCheckResult<T>>::remove(stash);
	}

	//
	fn create_pre_check_task(stash: T::AccountId, auth: T::AuthorityAres, bn: T::BlockNumber) -> bool {
		let mut task_list = <PreCheckTaskList<T>>::get().unwrap_or(Default::default());
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

		let _res = task_list.try_push((stash.clone(), auth.clone(), bn));
		<PreCheckTaskList<T>>::put(task_list);
		<FinalPerCheckResult<T>>::insert(
			stash.clone(),
			Some((bn, PreCheckStatus::Review, Option::<PreCheckCompareLog>::None,auth.clone(), )),
		);
		true
	}
}

impl<T: Config> ValidatorCount for Pallet<T> {
	fn get_validators_count() -> u64 {
		<Pallet<T>>::authorities().unwrap_or(Default::default()).len() as u64
	}
}
