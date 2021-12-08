#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::{
    self as system,
    offchain::{AppCrypto, CreateSignedTransaction, SignedPayload, SigningTypes},
};

use codec::{Decode, Encode};
use core::fmt;
use lite_json::json::JsonValue;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
    offchain::{http, Duration},
    transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
    AccountId32, RuntimeAppPublic, RuntimeDebug,
};
use sp_std::vec::Vec;

use frame_support::traits::{FindAuthor, Get, ValidatorSet, OneSessionHandler};
use serde::{Deserialize, Deserializer};
use sp_std::{prelude::*, str};

#[cfg(test)]
mod tests;

pub mod aura_handler;

pub const LOCAL_STORAGE_PRICE_REQUEST_MAKE_POOL: &[u8] = b"are-ocw::make_price_request_pool";
pub const LOCAL_STORAGE_PRICE_REQUEST_LIST: &[u8] = b"are-ocw::price_request_list";
pub const LOCAL_STORAGE_PRICE_REQUEST_DOMAIN: &[u8] = b"are-ocw::price_request_domain";
pub const CALCULATION_KIND_AVERAGE: u8 = 1;
pub const CALCULATION_KIND_MEDIAN: u8 = 2;

pub const PURCHASED_FINAL_TYPE_IS_THRESHOLD_UP: u8 = 1;
pub const PURCHASED_FINAL_TYPE_IS_FORCE_CLEAN: u8 = 2;


/// The keys can be inserted manually via RPC (see `author_insertKey`).
// pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ares"); // sp_application_crypto::key_types::BABE ; //
pub const KEY_TYPE: KeyTypeId = sp_application_crypto::key_types::AURA;
/// the types with this pallet-specific identifier.
pub mod crypto {
    use super::KEY_TYPE;
    use crate::pallet;
    use frame_support::pallet_prelude::PhantomData;
    use sp_core::sr25519::Signature as Sr25519Signature;
    use sp_runtime::{
        app_crypto::{app_crypto, sr25519},
        traits::Verify,
        MultiSignature, MultiSigner,
    };

    app_crypto!(sr25519, KEY_TYPE);

    // struct fro production
    pub struct OcwAuthId<T>(PhantomData<T>);

    impl<T: pallet::Config> frame_system::offchain::AppCrypto<MultiSigner, MultiSignature>
    for OcwAuthId<T>
        where
            sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
            u64: From<<T as frame_system::Config>::BlockNumber>,
    {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }

    impl<T: pallet::Config>
    frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
    for OcwAuthId<T>
        where
            sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
            u64: From<<T as frame_system::Config>::BlockNumber>,
    {
        type RuntimeAppPublic = Public;
        type GenericSignature = sp_core::sr25519::Signature;
        type GenericPublic = sp_core::sr25519::Public;
    }

    /// An i'm online identifier using sr25519 as its crypto.
    pub type AuthorityId = self::Public;
}

use crate::crypto::OcwAuthId;
use frame_support::sp_runtime::app_crypto::{Public};
use frame_support::sp_runtime::sp_std::convert::TryInto;
use frame_support::sp_runtime::traits::{IsMember};
use frame_system::offchain::{SendUnsignedTransaction, Signer};
use lite_json::NumberValue;
pub use pallet::*;
use sp_application_crypto::sp_core::crypto::UncheckedFrom;
use sp_consensus_aura::{AURA_ENGINE_ID, ConsensusLog, AuthorityIndex};
use sp_runtime::offchain::storage::StorageValueRef;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::sp_runtime::traits::{IdentifyAccount, IsMember};
    use sp_core::crypto::UncheckedFrom;
    use oracle_finance::traits::*;
    use oracle_finance::types::{BalanceOf, OcwPaymentResult};
    use frame_system::pallet_prelude::{BlockNumberFor, OriginFor};
    use frame_system::{ensure_signed, ensure_none};
    use staking_extend::IStakingNpos;
    use ares_oracle_provider_support::{IAresOraclePreCheck, JsonNumberValue, PreCheckTaskConfig, PreCheckStruct};


    #[pallet::error]
    #[derive(PartialEq, Eq)]
    pub enum Error<T> {
        //
        SubmitThresholdRangeError,
        DruationNumberNotBeZero,
        UnknownAresPriceVersionNum,
        InsufficientBalance,
        InsufficientMaxFee,
        PayToPurchaseFeeFailed,
        // InsufficientCountOfValidators,
        PerCheckTaskAlreadyExists,
        //
        PreCheckTokenListNotEmpty,

    }

    /// This pallet's configuration trait
    #[pallet::config]
    pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config + oracle_finance::Config
    where
        sp_runtime::AccountId32: From<<Self as frame_system::Config>::AccountId>,
        u64: From<<Self as frame_system::Config>::BlockNumber>,
    {
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
            + UncheckedFrom<[u8; 32]>;

        type FindAuthor: FindAuthor<Self::AccountId>;

        type ValidatorAuthority: IsType<<Self as frame_system::Config>::AccountId> + Member;

        type VMember: IsMember<Self::ValidatorAuthority>;

        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        // #[pallet::constant]
        // type NeedVerifierCheck: Get<bool>;

        // Used to confirm RequestPropose.
        type RequestOrigin: EnsureOrigin<Self::Origin>;

        #[pallet::constant]
        type FractionLengthNum: Get<u32>;

        #[pallet::constant]
        type CalculationKind: Get<u8>;

        type AuthorityCount: ValidatorCount;

        type OcwFinanceHandler: IForPrice<Self> + IForReporter<Self> + IForReward<Self>;

        type AresIStakingNpos: IStakingNpos<Self::AuthorityAres, Self::BlockNumber, StashId = <Self as frame_system::Config>::AccountId> ;
    }

    pub trait ValidatorCount {
        fn get_validators_count() -> u64;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
    where
        sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
        u64: From<<T as frame_system::Config>::BlockNumber>,
        <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
        <T as frame_system::Config>::AccountId: From<sp_application_crypto::sr25519::Public>,
    {

        fn on_runtime_upgrade() -> frame_support::weights::Weight {
            log::debug!("********** on_runtime_upgrade C1 **************************************************");
            if ConfPreCheckTokenList::<T>::get().len() == 0 {
                log::debug!("********** on_runtime_upgrade C2 **************************************************");
                ConfPreCheckAllowableOffset::<T>::put(Percent::from_percent(10));
                let session_multi: T::BlockNumber= 2u32.into();
                ConfPreCheckSessionMulti::<T>::put(session_multi);
                let mut token_list = Vec::new();
                token_list.push("btc-usdt".as_bytes().to_vec());
                token_list.push("eth-usdt".as_bytes().to_vec());
                token_list.push("dot-usdt".as_bytes().to_vec());
                ConfPreCheckTokenList::<T>::put(token_list);
                return T::DbWeight::get().reads_writes(1,5);
            }
            0
        }

        /// You can use `Local Storage` API to coordinate runs of the worker.
        fn offchain_worker(block_number: T::BlockNumber) {

            let control_setting = <OcwControlSetting<T>>::get();

            // T::AuthorityAres::unchecked_from()

            let block_author = Self::get_block_author();
            match block_author {
                None => {
                    log::warn!(target: "pallet::ocw::offchain_worker", "❗ Not found author.");
                }
                Some(author) => {
                    if control_setting.open_free_price_reporter {
                        log::info!("🚅 ❗ ⛔ Ocw offchain start {:?} ", &author);
                        // if Self::are_block_author_and_sotre_key_the_same(<pallet_authorship::Pallet<T>>::author()) {
                        if Self::are_block_author_and_sotre_key_the_same(author.clone()) {
                            log::debug!("🚅 @ Ares call [1] ares-price-worker.");
                            // Try to get ares price.
                            match Self::ares_price_worker(block_number.clone(), author.clone()) {
                                Ok(v) => log::debug!("🚅 @ Ares OCW price acquisition completed."),
                                Err(e) => log::warn!(
                                    target: "pallet::ocw::offchain_worker",
                                    "❗ Ares price has a problem : {:?}",
                                    e
                                ),
                            }

                            let conf_session_multi = ConfPreCheckSessionMulti::<T>::get();
                            // Do you need to scan pre-validator data
                            if T::AresIStakingNpos::near_era_change(conf_session_multi) {
                                log::debug!(" **** T::AresIStakingNpos::near_era_change is near will get data. RUN 1 ");
                                let pending_npos = T::AresIStakingNpos::pending_npos();
                                pending_npos.into_iter().any(|(stash_id, auth_id)| {
                                    // for v3.4.x --
                                    log::debug!(" **** T::AresIStakingNpos:: RUN 2 has pending stash stash_id = {:?} ", stash_id.clone());
                                    if auth_id.is_none() {
                                        log::warn!(
                                            target: "pallet::ocw::offchain_worker",
                                            "❗ Staking authority is not set, you can use RPC author_insertKey fill that.",
                                        )
                                    }
                                    if !Self::has_pre_check_task(stash_id.clone()) && auth_id.is_some() {
                                        log::debug!(" **** T::AresIStakingNpos:: RUN 2.1 auth_id = {:?}", auth_id.clone());
                                        // Self::create_pre_check_task(stash_id.clone(), auth_id.unwrap(), block_number);\
                                        Self::save_create_pre_check_task(author.clone(), stash_id, auth_id.unwrap(), block_number);
                                    }else{
                                        log::debug!(" **** T::AresIStakingNpos:: RUN 2.2");
                                    }
                                    false
                                });

                            }
                        }
                    }

                    if control_setting.open_paid_price_reporter {
                        if let Some(keystore_validator) = Self::keystore_validator_member() {
                            log::debug!("🚅 @ Ares call [2] ares-purchased-checker.");
                            match Self::ares_purchased_checker(block_number.clone(), keystore_validator) {
                                Ok(v) => log::debug!("🚅 % Ares OCW purchased checker completed."),
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

                // for v3.4.x . TO
                if let Some((stash, auth, _)) = Self::get_pre_task_by_authority_set(T::AuthorityAres::all()) {
                    // check authority id is own.
                    // Self::create_pre_check_task(stash_id.clone(), block_number);
                    log::debug!(" ********* get_pre_task_by_authority_set Is Running!!!  ");

                    // let mut token_list = Vec::new();
                    // token_list.push("eth-usdt".as_bytes().to_vec());
                    // token_list.push("btc-usdt".as_bytes().to_vec());

                    let token_list = ConfPreCheckTokenList::<T>::get();
                    let allowable_offset = ConfPreCheckAllowableOffset::<T>::get();

                    let check_config = PreCheckTaskConfig{
                        check_token_list: token_list,
                        allowable_offset: allowable_offset,
                    };

                    // get check result
                    let take_price_list = Self::take_price_for_per_check(check_config);
                    log::debug!(" ******* take_price_list = {:?}", &take_price_list);

                    Self::save_offchain_pre_check_result(stash, auth, block_number, take_price_list);
                }
            }
        }
    }

    /// A public part of the pallet.
    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
        u64: From<<T as frame_system::Config>::BlockNumber>,
        // <T as frame_system::offchain::SigningTypes>::Public: From<<T as pallet::Config>::AuthorityAres>,
    {
        #[pallet::weight(1000)]
        pub fn submit_ask_price (
            origin: OriginFor<T>,
            max_fee: BalanceOf<T>,
            request_keys: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            let purchased_default = <PurchasedDefaultSetting<T>>::get();
            let submit_threshold =purchased_default.submit_threshold;
            let max_duration = purchased_default.max_duration;
            let request_keys = Self::extract_purchased_request(request_keys);
            let offer = T::OcwFinanceHandler::calculate_fee_of_ask_quantity(request_keys.len() as u32);
            if offer > max_fee {
                return Err(Error::<T>::InsufficientMaxFee.into());
            }

            let purchase_id = Self::make_purchase_price_id(who.clone(), 0);

            let payment_result: OcwPaymentResult<T> = T::OcwFinanceHandler::reserve_for_ask_quantity(who.clone(), purchase_id.clone(), request_keys.len() as u32);
            // let payment_result: OcwPaymentResult<T> = OcwPaymentResult::<T>::Success(purchase_id.clone(), 0u32.into());
            match payment_result {
                OcwPaymentResult::InsufficientBalance(_, _balance) => {
                    return Err(Error::<T>::InsufficientBalance.into());
                }
                OcwPaymentResult::Success(_, balance) => {
                    // Up request on chain.
                    Self::ask_price(who, balance, submit_threshold, max_duration, purchase_id, request_keys);
                }
            }

            // Self::ask_price(who, amount, submit_threshold, max_duration, purchase_id, request_keys);
            Ok(().into())
        }


        #[pallet::weight(0)]
        pub fn submit_forced_clear_purchased_price_payload_signed (
            origin: OriginFor<T>,
            price_payload: PurchasedForceCleanPayload<T::Public, T::BlockNumber>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo  {
            ensure_none(origin)?;

            let purchase_id_list: Vec<Vec<u8>> = price_payload.purchase_id_list;
            purchase_id_list.iter().any(|x| {
                // check that the validator threshold is up to standard.
                if Self::is_validator_purchased_threshold_up_on(x.to_vec()) {
                    // println!("is_validator_purchased_threshold_up_on 1");
                    // update report work point
                    if Self::update_reporter_point(x.to_vec()).is_ok() {
                        // Calculate the average price
                        Self::update_purchase_avg_price_storage(x.to_vec(), PURCHASED_FINAL_TYPE_IS_FORCE_CLEAN);
                        // println!("is_validator_purchased_threshold_up_on 2");
                    }
                    // println!("is_validator_purchased_threshold_up_on 3");
                    Self::purchased_storage_clean(x.to_vec());
                }else{
                    // println!("refund_ask_paid p_id = {:?}", x.to_vec());
                    // Get `ask owner`
                    let refund_result = T::OcwFinanceHandler::unreserve_ask(x.to_vec());
                    // println!("refund_result = {:?}", &refund_result);
                    if refund_result.is_ok() {
                        // Remove chain storage.
                        // <PurchasedPricePool<T>>::remove_prefix(x.to_vec(), None);
                        // <PurchasedRequestPool<T>>::remove(x.to_vec());
                        // <PurchasedOrderPool<T>>::remove_prefix(x.to_vec(), None);
                        Self::purchased_storage_clean(x.to_vec());
                        Self::deposit_event(Event::InsufficientCountOfValidators);
                    } else {
                        log::error!(
                            target: "pallet::ocw::submit_forced_clear_purchased_price_payload_signed",
                            "⛔️ T::OcwFinanceHandler::unreserve_ask() had an error!"
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
            price_payload: PurchasedPricePayload<T::Public, T::BlockNumber>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo  {
            ensure_none(origin)?;

            log::info!(
                "🚅 Pre submit purchased price payload on block {:?}",
                <system::Pallet<T>>::block_number()
            );

            // check whether purchased request still exists
            if false == <PurchasedRequestPool<T>>::contains_key(price_payload.purchase_id.clone()) {
                Self::deposit_event(Event::PurchasedRequestWorkHasEnded(
                    price_payload.purchase_id.clone(),
                    price_payload.public.clone().into_account(),
                ));
                return Ok(().into());
            }

            Self::add_purchased_price(price_payload.purchase_id.clone(),
                                      price_payload.public.clone().into_account(),
                                      price_payload.block_number.clone(),
                                      price_payload.price.clone());


            // check
            if Self::is_all_validator_submitted_price(price_payload.purchase_id.clone()) {

                // update report work point
                if Self::update_reporter_point(price_payload.purchase_id.clone()).is_ok() {
                    // Calculate the average price
                    Self::update_purchase_avg_price_storage(price_payload.purchase_id.clone(), PURCHASED_FINAL_TYPE_IS_THRESHOLD_UP);
                }
                // clean order pool
                // <PurchasedOrderPool<T>>::remove_prefix(price_payload.purchase_id.clone(), None);
                Self::purchased_storage_clean(price_payload.purchase_id.clone());
            }

            // check that the validator threshold is up to standard.
            // if Self::is_validator_purchased_threshold_up_on(price_payload.purchase_id.clone()) {
            //     // Calculate the average price
            //     Self::update_purchase_avg_price_storage(price_payload.purchase_id.clone(), PURCHASED_FINAL_TYPE_IS_THRESHOLD_UP);
            //     // clean order pool
            //     <PurchasedOrderPool<T>>::remove_prefix(price_payload.purchase_id.clone(), None);
            // }

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn submit_price_unsigned_with_signed_payload(
            origin: OriginFor<T>,
            price_payload: PricePayload<T::Public, T::BlockNumber>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            log::debug!(
                "🚅 Pre submit price payload on block {:?}",
                <system::Pallet<T>>::block_number()
            );

            // Nodes with the right to increase prices
            let price_list = price_payload.price; // price_list: Vec<(PriceKey, u32)>,

            let mut event_result: Vec<(Vec<u8>, u64, FractionLength)> = Vec::new();

            let mut price_key_list = Vec::new();
            // for (price_key, price, fraction_length, json_number_value) in price_list.clone() {
            for PricePayloadSubPrice(price_key, price, fraction_length, json_number_value) in
                price_list.clone()
            {
                // Add the price to the on-chain list, but mark it as coming from an empty address.
                // log::info!(" Call add_price {:?}", sp_std::str::from_utf8(&price_key));
                Self::add_price(
                    price_payload.public.clone().into_account(),
                    price.clone(),
                    price_key.clone(),
                    fraction_length,
                    json_number_value,
                    Self::get_price_pool_depth(),
                );
                price_key_list.push(price_key.clone());
                event_result.push((price_key, price, fraction_length));
            }

            log::debug!(
                "🚅 Submit price list on chain, count = {:?}",
                price_key_list.len()
            );

            // update last author
            Self::update_last_price_list_for_author(
                price_key_list,
                price_payload.public.clone().into_account(),
            );

            // Set jump block
            let jump_block = price_payload.jump_block;
            if jump_block.len() > 0 {
                for PricePayloadSubJumpBlock(price_key, interval) in jump_block.clone() {
                    Self::increase_jump_block_number(price_key, interval as u64);
                }
            }

            log::debug!(
                "🚅 Submit jump block list on chain, count = {:?}",
                jump_block.len()
            );

            Self::deposit_event(Event::NewPrice(
                event_result,
                jump_block,
                price_payload.public.clone().into_account(),
            ));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn submit_create_pre_check_task (
            origin: OriginFor<T>,
            precheck_payload: PerCheckPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>,
            _signature: T::Signature,
        ) -> DispatchResult {
            ensure_none(origin)?;
            let result = Self::create_pre_check_task(precheck_payload.stash, precheck_payload.auth, precheck_payload.block_number);

            ensure!(result, Error::<T>::PerCheckTaskAlreadyExists );

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn submit_offchain_pre_check_result (
            origin: OriginFor<T>,
            preresult_payload: PreCheckResultPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>,
            _signature: T::Signature,
        ) -> DispatchResult {
            ensure_none(origin)?;

            log::debug!(" ------ debug . on RUN A4");
            Self::save_pre_check_result(
                preresult_payload.stash,
                preresult_payload.block_number,
                preresult_payload.per_check_list
            );
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn update_purchased_param(origin: OriginFor<T>, submit_threshold: u8, max_duration: u64, avg_keep_duration: u64, unit_price: u64) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;


            ensure!(submit_threshold>0 && submit_threshold <= 100 , Error::<T>::SubmitThresholdRangeError);
            ensure!(max_duration > 0 , Error::<T>::DruationNumberNotBeZero);

            let setting_data = PurchasedDefaultData::new(submit_threshold, max_duration, avg_keep_duration, unit_price);

            <PurchasedDefaultSetting<T>>::put(setting_data.clone());
            Self::deposit_event(Event::UpdatePurchasedDefaultSetting(setting_data));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn update_ocw_control_setting(origin: OriginFor<T>, need_verifier_check: bool, open_free_price_reporter: bool, open_paid_price_reporter: bool) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;

            let setting_data = OcwControlData {
                need_verifier_check,
                open_free_price_reporter,
                open_paid_price_reporter,
            };

            <OcwControlSetting<T>>::put(setting_data.clone());
            Self::deposit_event(Event::UpdateOcwControlSetting(setting_data));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn revoke_update_request_propose(origin: OriginFor<T>, price_key: Vec<u8>) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;

            <PricesRequests<T>>::mutate(|prices_request| {
                for (index, (old_price_key, _, _, _, _)) in
                    prices_request.clone().into_iter().enumerate()
                {
                    if &price_key == &old_price_key {
                        // remove old one
                        prices_request.remove(index);
                        Self::clear_price_storage_data(price_key.clone());
                        break;
                    }
                }
            });

            Self::remove_jump_block_number(price_key.clone());

            Self::deposit_event(Event::RevokePriceRequest(price_key));

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn update_request_propose(
            origin: OriginFor<T>,
            price_key: Vec<u8>,
            prase_token: Vec<u8>,
            parse_version: u32,
            fraction_num: FractionLength,
            request_interval: RequestInterval,
        ) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;

            <PricesRequests<T>>::mutate(|prices_request| {
                let mut find_old = false;
                for (
                    index,
                    (old_price_key, _old_prase_token, _old_parse_version, old_fraction_count, _),
                ) in prices_request.clone().into_iter().enumerate()
                {
                    if &price_key == &old_price_key {
                        if &"".as_bytes().to_vec() != &prase_token {
                            // add input value
                            prices_request.push((
                                price_key.clone(),
                                prase_token.clone(),
                                parse_version,
                                fraction_num.clone(),
                                request_interval.clone(),
                            ));
                            Self::deposit_event(Event::UpdatePriceRequest(
                                price_key.clone(),
                                prase_token.clone(),
                                parse_version.clone(),
                                fraction_num.clone(),
                            ));
                        }

                        // remove old one
                        prices_request.remove(index);

                        // check fraction number on change
                        if &old_fraction_count != &fraction_num
                            || &"".as_bytes().to_vec() == &prase_token
                        {
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
                        prase_token.clone(),
                        parse_version.clone(),
                        fraction_num.clone(),
                        request_interval.clone(),
                    ));
                    Self::deposit_event(Event::AddPriceRequest(
                        price_key,
                        prase_token,
                        parse_version,
                        fraction_num,
                    ));
                }
            });

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn update_allowable_offset_propose(origin: OriginFor<T>, offset: u8) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;
            <PriceAllowableOffset<T>>::put(offset);
            Self::deposit_event(Event::PriceAllowableOffsetUpdate(offset));

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
            Self::deposit_event(Event::PricePoolDepthUpdate(depth));

            if depth > old_depth {
                // <PricesRequests<T>>::get().into_iter().any(|(price_key,_,_,_,_)|{
                //     // clear average.
                //     <AresAvgPrice<T>>::remove(&price_key);
                //     false
                // }) ;
            } else {
                <PricesRequests<T>>::get()
                    .into_iter()
                    .any(|(price_key, _, _, _, _)| {
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
        pub fn update_pre_check_token_list(
            origin: OriginFor<T>,
            token_list: Vec<Vec<u8>>,
        ) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;
            ensure!(token_list.len() > 0, Error::<T>::PreCheckTokenListNotEmpty);
            <ConfPreCheckTokenList<T>>::put(token_list);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn update_pre_check_session_multi(
            origin: OriginFor<T>,
            multi: T::BlockNumber,
        ) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;
            <ConfPreCheckSessionMulti<T>>::put(multi);
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn update_pre_check_allowable_offset(
            origin: OriginFor<T>,
            offset: Percent,
        ) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;
            <ConfPreCheckAllowableOffset<T>>::put(offset);
            Ok(().into())
        }
    }

    /// Events for the pallet.
    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config>
    where
        sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
        u64: From<<T as frame_system::Config>::BlockNumber>,
    {
        // (price_key, price_val, fraction len)
        NewPrice(Vec<(Vec<u8>, u64, FractionLength)>, Vec<PricePayloadSubJumpBlock>, T::AccountId),

        // The report request was closed when the price was submitted
        PurchasedRequestWorkHasEnded(Vec<u8>, T::AccountId),

        NewPurchasedPrice(T::BlockNumber, Vec<PricePayloadSubPrice>, T::AccountId),
        // purchased_id
        NewPurchasedRequest(Vec<u8>, PurchasedRequestData<T>, BalanceOf<T>),
        // NewPurchasedRequest(Vec<u8>, PurchasedRequestData<T>),
        // purchased_id , vec
        PurchasedAvgPrice(Vec<u8>, Vec<Option<(Vec<u8>, PurchasedAvgPriceData, Vec<T::AccountId>)>>),
        UpdatePurchasedDefaultSetting(PurchasedDefaultData),
        UpdateOcwControlSetting(OcwControlData),
        // Average price update.
        RevokePriceRequest(Vec<u8>),
        AddPriceRequest(Vec<u8>, Vec<u8>, u32, FractionLength),
        UpdatePriceRequest(Vec<u8>, Vec<u8>, u32, FractionLength),
        //
        PricePoolDepthUpdate(u32),
        //
        PriceAllowableOffsetUpdate(u8),
        InsufficientCountOfValidators,
        ProblemWithRefund,
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T>
    where
        sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
        u64: From<<T as frame_system::Config>::BlockNumber>,
    {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            if let Call::submit_price_unsigned_with_signed_payload(ref payload, ref signature) = call {
                log::debug!(
                    "🚅 Validate price payload data, on block: {:?}/{:?}, author: {:?} ",
                    payload.block_number.clone(),
                    <system::Pallet<T>>::block_number(),
                    payload.public.clone()
                );

                let sign_accoount = <T as SigningTypes>::Public::into_account(payload.public.clone());
                if ! Self::is_validator_member(sign_accoount.clone().into()) {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Payload public id is no longer in the members. `InvalidTransaction` on price "
                    );
                    // log::warn!(
                    //     "❗Author are not validator. disable refuse account: {:?}", sign_accoount.clone()
                    // );
                    return InvalidTransaction::BadProof.into();
                }

                let signature_valid =
                    SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());

                // let signature_valid =
                //     SignedPayload::<T>::verify::<T::AuthorityAres>(payload, signature.clone());

                if !signature_valid {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Signature invalid. `InvalidTransaction` on price"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                Self::validate_transaction_parameters_of_ares(
                    &payload.block_number,
                    payload.price.to_vec(),
                )
            } else if let Call::submit_purchased_price_unsigned_with_signed_payload(ref payload, ref signature) = call {
                // submit_purchased_price_unsigned_with_signed_payload
                log::debug!(
                    "🚅 Validate purchased price payload data, on block: {:?} ",
                    <system::Pallet<T>>::block_number()
                );

                if ! Self::is_validator_member(<T as SigningTypes>::Public::into_account(payload.public.clone()).into()) {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Payload public id is no longer in the members. `InvalidTransaction` on purchased price"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                let signature_valid =
                    SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());

                // let signature_valid =
                //     SignedPayload::<T>::verify::<T::AuthorityAres>(payload, signature.clone());

                if !signature_valid {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Signature invalid. `InvalidTransaction` on purchased price"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                let priority_num: u64 = T::UnsignedPriority::get();

                // let perfix_v8 = Self::make_transaction_tag_prefix_with_public_account(
                //     "ares-oracle::validate_transaction_parameters_of_purchased_price".as_bytes().to_vec(),
                //     payload.public.clone()
                // );
                // let perfix_str: &'static str = sp_std::str::from_utf8("ares-oracle::validate_transaction_parameters_of_purchased_price").unwrap();
                ValidTransaction::with_tag_prefix("ares-oracle::validate_transaction_parameters_of_purchased_price")
                    .priority(priority_num.saturating_add(1))
                    .and_provides(payload.public.clone())
                    // .and_provides(&payload.block_number)
                    .longevity(5)
                    .propagate(true)
                    .build()

            } else if let Call::submit_forced_clear_purchased_price_payload_signed(ref payload, ref signature) = call  {
                // submit_forced_clear_purchased_price_payload_signed
                log::debug!(
                    "🚅 Validate forced clear purchased price payload data, on block: {:?} ",
                    <system::Pallet<T>>::block_number()
                );

                if ! Self::is_validator_member(<T as SigningTypes>::Public::into_account(payload.public.clone()).into()) {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Payload public id is no longer in the members. `InvalidTransaction` on force clear purchased"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                let signature_valid =
                    SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());

                // let signature_valid =
                //     SignedPayload::<T>::verify::<T::AuthorityAres>(payload, signature.clone());

                if !signature_valid {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Signature invalid. `InvalidTransaction` on force clear purchased"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                let priority_num: u64 = T::UnsignedPriority::get();
                ValidTransaction::with_tag_prefix("ares-oracle::validate_transaction_parameters_of_force_clear_purchased")
                    .priority(priority_num.saturating_add(2))
                    .and_provides(&payload.block_number) // next_unsigned_at
                    .longevity(5)
                    .propagate(true)
                    .build()
            } else if let Call::submit_create_pre_check_task(ref payload, ref signature) = call {
                log::debug!(
                    "🚅 Validate submit_create_pre_check_task, on block: {:?}/{:?}, author: {:?} ",
                    payload.block_number.clone(),
                    <system::Pallet<T>>::block_number(),
                    payload.public.clone()
                );

                let sign_accoount = <T as SigningTypes>::Public::into_account(payload.public.clone());
                if ! Self::is_validator_member(sign_accoount.clone().into()) {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Payload public id is no longer in the members. `InvalidTransaction` on price "
                    );
                    // log::warn!(
                    //     "❗Author are not validator. disable refuse account: {:?}", sign_accoount.clone()
                    // );
                    return InvalidTransaction::BadProof.into();
                }

                let signature_valid =
                    SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());

                if !signature_valid {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Signature invalid. `InvalidTransaction` on price"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                ValidTransaction::with_tag_prefix("ares-oracle::submit_create_pre_check_task")
                    .priority(T::UnsignedPriority::get())
                    .and_provides(&payload.block_number) // next_unsigned_at
                    .longevity(5)
                    .propagate(true)
                    .build()
            } else if let Call::submit_offchain_pre_check_result(ref payload, ref signature) = call {
                log::debug!(
                    "🚅 Validate submit_offchain_pre_check_result, on block: {:?}/{:?}, stash: {:?}, auth: {:?}, pub: {:?}",
                    payload.block_number.clone(),
                    <system::Pallet<T>>::block_number(),
                    payload.stash.clone(),
                    payload.auth.clone(),
                    payload.public.clone()
                );

                log::debug!(" ------ debug . on RUN A1");
                // check stash status
                let mut auth_list = Vec::new();
                auth_list.push(payload.auth.clone());
                if let Some((s_stash, _, _)) = Self::get_pre_task_by_authority_set(auth_list) {
                    if &s_stash != &payload.stash {
                        log::error!(
                            target: "pallet::ocw::validate_unsigned - submit_offchain_pre_check_result",
                            "⛔️ Stash account is inconsistent!"
                        );
                        return InvalidTransaction::BadProof.into();
                    }
                }else{
                    log::error!(
                        target: "pallet::ocw::validate_unsigned - submit_offchain_pre_check_result",
                        "⛔️ Could not find the pre-check task!"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                log::debug!(" ------ debug . on RUN A2");
                //
                let signature_valid =
                    SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());

                if !signature_valid {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned - submit_offchain_pre_check_result",
                        "⛔️ Signature invalid. `InvalidTransaction` on price"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                log::debug!(" ------ debug . on RUN A3");
                //
                ValidTransaction::with_tag_prefix("ares-oracle::submit_offchain_pre_check_result")
                    .priority(T::UnsignedPriority::get())
                    .and_provides(&payload.block_number) // next_unsigned_at
                    .longevity(5)
                    .propagate(true)
                    .build()
            } else {
                //
                InvalidTransaction::Call.into()
            }
        }
    }

    #[pallet::storage]
    #[pallet::getter(fn purchased_default_setting)]
    pub(super) type PurchasedDefaultSetting<T: Config> = StorageValue<
        _,
        PurchasedDefaultData,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn ocw_control_setting)]
    pub(super) type OcwControlSetting<T: Config> = StorageValue<
        _,
        OcwControlData,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn purchased_request_pool)]
    pub(super) type PurchasedRequestPool<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>, // purchased_id
        PurchasedRequestData<T>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn purchased_price_pool)]
    pub(super) type PurchasedPricePool<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        Vec<u8>, // purchased_id,
        Blake2_128Concat,
        Vec<u8>, // price_key,,
        Vec<AresPriceData<T::AccountId, T::BlockNumber>>,
        // Vec<AresPrice<T>>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn purchased_avg_price)]
    pub(super) type PurchasedAvgPrice<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        Vec<u8>, // purchased_id,
        Blake2_128Concat,
        Vec<u8>, // price_key,,
        PurchasedAvgPriceData,
        // Vec<AresPrice<T>>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn purchased_avg_trace)]
    pub(super) type PurchasedAvgTrace<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>, // pricpurchased_ide_key
        (T::BlockNumber),
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn purchased_order_pool)]
    pub(super) type PurchasedOrderPool<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        Vec<u8>, // purchased_id,
        Blake2_128Concat,
        T::AccountId,
        T::BlockNumber,
        // Vec<AresPrice<T>>,
        ValueQuery,
    >;


    #[pallet::storage]
    #[pallet::getter(fn last_price_author)]
    pub(super) type LastPriceAuthor<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>, // price_key
        T::AccountId,
        ValueQuery,
    >;

    /// The lookup table for names.
    #[pallet::storage]
    #[pallet::getter(fn ares_prices)]
    pub(super) type AresPrice<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,
        Vec<AresPriceData<T::AccountId, T::BlockNumber>>,
        ValueQuery,
    >;

    /// The lookup table for names.
    #[pallet::storage]
    #[pallet::getter(fn ares_abnormal_prices)]
    pub(super) type AresAbnormalPrice<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,
        // price, account, bolcknumber, FractionLength, JsonNumberValue
        // Vec<(
        //     u64,
        //     T::AccountId,
        //     T::BlockNumber,
        //     FractionLength,
        //     JsonNumberValue,
        // )>,
        Vec<AresPriceData<T::AccountId, T::BlockNumber>>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn ares_avg_prices)]
    pub(super) type AresAvgPrice<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,
        // price
        (u64, FractionLength),
        ValueQuery,
    >;

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
            Vec<u8>, // price key
            Vec<u8>, // price token
            u32,     // parse version number.
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
    pub(super) type JumpBlockNumber<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, u64>;


    // ---

    // // PerCheckResult
    // #[pallet::storage]
    // #[pallet::getter(fn per_check_result)]
    // pub(super) type PerCheckResult<T: Config> = StorageDoubleMap<
    //     _,
    //     Blake2_128Concat,
    //     T::AccountId,
    //     Blake2_128Concat,
    //     T::BlockNumber,
    //     PreCheckStatus,
    //     ValueQuery,
    // >;

    // FinalPerCheckResult
    #[pallet::storage]
    #[pallet::getter(fn final_per_check_result)]
    pub(super) type FinalPerCheckResult<T: Config> = StorageMap<_,
        Blake2_128Concat,
        T::AccountId,
        Option<(T::BlockNumber, PreCheckStatus)>
    >;

    // PreCheckTaskList
    #[pallet::storage]
    #[pallet::getter(fn per_check_task_list)]
    pub(super) type PreCheckTaskList<T: Config> = StorageValue<_,
        Vec<(T::AccountId, T::AuthorityAres, T::BlockNumber)>,
        ValueQuery
    >;


    #[pallet::storage]
    #[pallet::getter(fn symbol_fraction)]
    pub(super) type SymbolFraction<T: Config> = StorageMap<_, Blake2_128Concat, Vec<u8>, FractionLength>;


    // pre_check_session_multi A parameter used to determine the pre-verification
    // task a few session cycles before the validator election, the type is BlockNumber.
    #[pallet::storage]
    #[pallet::getter(fn conf_pre_check_session_multi)]
    pub(super) type ConfPreCheckSessionMulti<T: Config> = StorageValue<_,
        T::BlockNumber,
        ValueQuery
    >;

    // pre_check_token_list is used to check the token_list with the average value on the chain.
    #[pallet::storage]
    #[pallet::getter(fn conf_pre_check_token_list)]
    pub(super) type ConfPreCheckTokenList<T: Config> = StorageValue<_,
        Vec<Vec<u8>>, // Vec<price_key>
        ValueQuery
    >;

    // pre_check_allowable_offset is maximum allowable deviation when comparing means.
    #[pallet::storage]
    #[pallet::getter(fn conf_pre_check_allowable_offset)]
    pub(super) type ConfPreCheckAllowableOffset<T: Config> = StorageValue<_,
        Percent, //
        ValueQuery
    >;

    // ---

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config>
    where
        AccountId32: From<<T as frame_system::Config>::AccountId>,
        u64: From<<T as frame_system::Config>::BlockNumber>,
        // <T as frame_system::offchain::SigningTypes>::Public: From<<T as pallet::Config>::AuthorityAres>,
    {
        pub _phantom: sp_std::marker::PhantomData<T>,
        pub request_base: Vec<u8>,
        pub price_allowable_offset: u8,
        pub price_pool_depth: u32,
        pub price_requests: Vec<(Vec<u8>, Vec<u8>, u32, FractionLength, RequestInterval)>,
        // pub pre_check_session_multi: T::BlockNumber,
        // pub pre_check_token_list: Vec<Vec<u8>>,
        // pub pre_check_allowable_offset: Percent,
    }



    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T>
    where
        AccountId32: From<<T as frame_system::Config>::AccountId>,
        u64: From<<T as frame_system::Config>::BlockNumber>,
        // <T as frame_system::offchain::SigningTypes>::Public: From<<T as pallet::Config>::AuthorityAres>,
    {
        fn default() -> Self {
            GenesisConfig {
                _phantom: Default::default(),
                request_base: Vec::new(),
                price_allowable_offset: 10u8,
                price_pool_depth: 10u32,
                price_requests: Vec::new(),
                // pre_check_session_multi: 2u32.into(),
                // pre_check_token_list: Vec::new(),
                // pre_check_allowable_offset: Percent::from_percent(10),
            }
        }
    }

    // #[pallet::genesis_build]
    // impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
    //     fn build(&self) {
    //         let finance_account = <Pallet<T>>::account_id();
    //         if T::Currency::total_balance(&finance_account).is_zero() {
    //             T::Currency::deposit_creating(&finance_account, T::Currency::minimum_balance());
    //             // Self::deposit_event(Event::<T>::OcwFinanceDepositCreating(T::Currency::minimum_balance()));
    //         }
    //     }
    // }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T>
    where
        AccountId32: From<<T as frame_system::Config>::AccountId>,
        u64: From<<T as frame_system::Config>::BlockNumber>,
        // <T as frame_system::offchain::SigningTypes>::Public: From<<T as pallet::Config>::AuthorityAres>,
    {
        fn build(&self) {
            if !self.price_requests.is_empty() {
                PricesRequests::<T>::put(&self.price_requests);
                self.price_requests
                    .iter()
                    .for_each(|(symbol, _, _, fractionLength, _)| {
                        SymbolFraction::<T>::insert(symbol, fractionLength);
                    })
            }
            if self.price_pool_depth > 0 {
                PricePoolDepth::<T>::put(&self.price_pool_depth);
            }
            if self.request_base.len() > 0 {
                RequestBaseOnchain::<T>::put(&self.request_base);
            }
            PriceAllowableOffset::<T>::put(&self.price_allowable_offset);
            // For new vesrion.
            ConfPreCheckAllowableOffset::<T>::put(Percent::from_percent(10));
            let session_multi: T::BlockNumber= 2u32.into();
            ConfPreCheckSessionMulti::<T>::put(session_multi);
            let mut token_list = Vec::new();
            token_list.push("btc-usdt".as_bytes().to_vec());
            token_list.push("eth-usdt".as_bytes().to_vec());
            token_list.push("dot-usdt".as_bytes().to_vec());
            ConfPreCheckTokenList::<T>::put(token_list);
        }
    }
}

pub mod types;
pub mod traits;
pub mod crypto2;

use types::*;
use hex;
use sp_runtime::traits::UniqueSaturatedInto;
use frame_support::pallet_prelude::StorageMap;
use oracle_finance::types::BalanceOf;
use oracle_finance::traits::{IForReporter, IForPrice};
use crate::traits::*;
use frame_support::weights::Weight;
use frame_support::sp_runtime::{Percent, DigestItem};
use ares_oracle_provider_support::{IAresOraclePreCheck, JsonNumberValue, PreCheckTaskConfig, PreCheckStruct, PreCheckStatus};



impl<T: Config> Pallet<T>
where
    sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
    u64: From<<T as frame_system::Config>::BlockNumber>,
{
    fn are_block_author_and_sotre_key_the_same(block_author: T::AccountId) -> bool {
        // let mut is_same = !T::NeedVerifierCheck::get(); // Self::get_default_author_save_bool();

        let mut is_same = !<OcwControlSetting<T>>::get().need_verifier_check;
        if is_same {
            log::warn!(
                target: "pallet::are_block_author_and_sotre_key_the_same",
                "❗❗❗ verifier_check is disable, current status is debug."
            );
        }
        let worker_ownerid_list = T::AuthorityAres::all();
        for ownerid in worker_ownerid_list.iter() {
            let mut a = [0u8; 32];
            a[..].copy_from_slice(&ownerid.to_raw_vec());
            // extract AccountId32 from store keys
            let owner_account_id32 = AccountId32::new(a);
            // debug end.
            if owner_account_id32 == block_author.clone().into() {
                log::info!("🚅 found mathing block author {:?}", &block_author);
                is_same = true;
            }
        }
        is_same
    }

    /// Obtain ares price and submit it.
    fn ares_price_worker(
        block_number: T::BlockNumber,
        account_id: T::AccountId,
    ) -> Result<(), &'static str>
    where
        <T as frame_system::offchain::SigningTypes>::Public:
            From<sp_application_crypto::sr25519::Public>,
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
    fn ares_purchased_worker(
        block_number: T::BlockNumber,
        account_id: T::AccountId,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public:
            From<sp_application_crypto::sr25519::Public>,
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
    fn ares_purchased_checker(
        block_number: T::BlockNumber,
        account_id: T::AccountId,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public:
            From<sp_application_crypto::sr25519::Public>,
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
    fn get_raw_price_source_list() -> Vec<(Vec<u8>, Vec<u8>, u32, FractionLength, RequestInterval)>
    {
        let result: Vec<(Vec<u8>, Vec<u8>, u32, FractionLength, RequestInterval)> =
            <PricesRequests<T>>::get()
                .into_iter()
                .map(
                    |(price_key, request_url, parse_version, fraction_length, request_interval)| {
                        (
                            price_key,
                            // sp_std::str::from_utf8(&request_url).unwrap().clone(),
                            request_url,
                            parse_version,
                            fraction_length,
                            request_interval,
                        )
                    },
                )
                .collect();
        result
    }

    // Get request domain, include TCP protocol, example: http://www.xxxx.com
    fn get_local_storage_request_domain() -> Vec<u8> {
        let request_base_onchain = RequestBaseOnchain::<T>::get();
        if request_base_onchain.len() > 0 {
            return request_base_onchain;
        }

        let mut storage_request_base =
            StorageValueRef::persistent(LOCAL_STORAGE_PRICE_REQUEST_DOMAIN);

        if let Some(request_base) = storage_request_base
            .get::<Vec<u8>>()
            .unwrap_or(Some(Vec::new()))
        {
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

    fn make_local_storage_request_uri_by_str(sub_path: &str) -> Vec<u8> {
        Self::make_local_storage_request_uri_by_vec_u8(sub_path.as_bytes().to_vec())
    }
    fn make_local_storage_request_uri_by_vec_u8(sub_path: Vec<u8>) -> Vec<u8> {
        let domain = Self::get_local_storage_request_domain(); //.as_bytes().to_vec();
        [domain, sub_path].concat()
    }

    fn get_block_author() -> Option<T::AccountId> {
        let digest = <frame_system::Pallet<T>>::digest();
        let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
        <T as pallet::Config>::FindAuthor::find_author(pre_runtime_digests)
    }

    // extract LocalPriceRequestStorage struct from json str
    fn extract_local_price_storage(price_json_str: &str) -> Option<LocalPriceRequestStorage> {
        serde_json::from_str(price_json_str).ok()
    }

    // // Get the number of cycles required to loop the array lenght.
    // // If round_num = 0 returns the maximum value of u8
    // fn get_number_of_cycles(vec_len: u8, round_num: u8) -> u8 {
    //     if round_num == 0 {
    //         return u8::MAX;
    //     }
    //     let mut round_offset = 0u8;
    //     if vec_len % round_num != 0 {
    //         round_offset = 1u8;
    //     }
    //     vec_len / round_num + round_offset
    // }

    fn save_forced_clear_purchased_price_payload_signed(
        block_number: T::BlockNumber,
        account_id: T::AccountId,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public:
            From<sp_application_crypto::sr25519::Public>,
    {
        let force_request_list = Self::get_expired_purchased_transactions();

        log::debug!("🚅 Force request list length: {:?} .", &force_request_list.len());
        if force_request_list.len() > 0 {
            // send force clear transaction
            // -- Sign using any account
            // let mut sign_public_keys: Vec<<T as SigningTypes>::Public> = Vec::new();
            // let encode_data: Vec<u8> = account_id.encode();
            // assert!(32 == encode_data.len());
            // let raw_data = encode_data.try_into();
            // let raw_data = raw_data.unwrap();
            // // <T as SigningTypes>::Public::
            // // let new_account = T::AuthorityAres::unchecked_from(raw_data);
            // // <T as SigningTypes>::Public::unchecked_from(raw_data);
            // let new_account = sp_core::sr25519::Public::from_raw(raw_data);
            // sign_public_keys.push(new_account.into());

            let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());

            // Singer
            let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
            // let (_, result) = Signer::<T, T::AuthorityAres>::any_account()
                .with_filter(sign_public_keys)
                .send_unsigned_transaction(
                    |account| PurchasedForceCleanPayload {
                        block_number,
                        purchase_id_list: force_request_list.clone(),
                        public: account.public.clone(),
                    },
                    |payload, signature| {
                        Call::submit_forced_clear_purchased_price_payload_signed(payload, signature)
                    },
                )
                .ok_or(
                    "❗ No local accounts accounts available, `ares` StoreKey needs to be set.",
                )?;
            result.map_err(|()| "⛔ Unable to submit transaction")?;
        }

        Ok(())
    }


    fn handler_get_sign_public_keys(account_id: T::AccountId) -> Vec<<T as SigningTypes>::Public>
        where <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
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
        account_id: T::AccountId,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public:
            From<sp_application_crypto::sr25519::Public>,
    {
        let mut price_list = Vec::new();

        // // Get raw request.
        // let format_arr = Self::make_bulk_price_format_data(block_number);
        // // Filter jump block info
        // let (format_arr, jump_block) =
        //     Self::filter_jump_block_data(format_arr.clone(), account_id.clone(), block_number);

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

        let fetch_http_reesult = Self::fetch_bulk_price_with_http(purchased_key.clone().raw_source_keys)
            .ok();


        let price_result = fetch_http_reesult;
        if price_result.is_none() {
            log::error!(
                target: "pallet::ocw::save_fetch_purchased_price_and_send_payload_signed",
                "⛔ Ocw network error."
            );
            return Ok(());
        }

        for (price_key, price_option, fraction_length, json_number_value) in price_result.unwrap() {
            if price_option.is_some() {
                // record price to vec!
                price_list.push(PricePayloadSubPrice(
                    price_key,
                    price_option.unwrap(),
                    fraction_length,
                    JsonNumberValue::new(json_number_value),
                ));
            }
        }

        log::debug!("🚅 fetch purchased price count: {:?}", price_list.len());

        if price_list.len() > 0  {
            // -- Sign using any account
            // let mut sign_public_keys: Vec<<T as SigningTypes>::Public> = Vec::new();
            // let encode_data: Vec<u8> = account_id.encode();
            // assert!(32 == encode_data.len());
            // let raw_data = encode_data.try_into();
            // let raw_data = raw_data.unwrap();
            // // <T as SigningTypes>::Public::
            // let new_account = T::AuthorityAres::unchecked_from(raw_data);
            // // <T as SigningTypes>::Public::unchecked_from(raw_data);
            // let new_account = sp_core::sr25519::Public::from_raw(raw_data);
            // sign_public_keys.push(new_account.into());

            let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
            // Singer
            let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
                .with_filter(sign_public_keys)
                .send_unsigned_transaction(
                    |account| PurchasedPricePayload {
                        price: price_list.clone(),
                        block_number,
                        public: account.public.clone(),
                        purchase_id: purchased_key.purchase_id.clone(),
                    },
                    |payload, signature| {
                        Call::submit_purchased_price_unsigned_with_signed_payload(payload, signature)
                    },
                )
                .ok_or(
                    "❗ No local accounts accounts available, `ares` StoreKey needs to be set.",
                )?;
            result.map_err(|()| "⛔ Unable to submit transaction")?;
        }
        Ok(())
    }


    fn save_offchain_pre_check_result(
        stash_id: T::AccountId,
        auth_id: T::AuthorityAres,
        block_number: T::BlockNumber,
        per_check_list: Vec<PreCheckStruct>) -> Result<(), &'static str>  where
            <T as frame_system::offchain::SigningTypes>::Public:
            From<sp_application_crypto::sr25519::Public>,
    {
        // let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());

        let mut tmp_raw = [0u8; 32];
        tmp_raw[..].copy_from_slice(&auth_id.to_raw_vec());

        let mut sign_public_keys:Vec<<T as SigningTypes>::Public> = Vec::new();
        let new_account = sp_core::sr25519::Public::from_raw(tmp_raw);
        // let new_account = auth_id.Public
        sign_public_keys.push(new_account.into());

        // Singer
        let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
            .with_filter(sign_public_keys)
            .send_unsigned_transaction(
                |account| PreCheckResultPayload {
                    stash: stash_id.clone(),
                    auth: auth_id.clone(),
                    block_number,
                    per_check_list: per_check_list.clone(),
                    public: account.public.clone(),
                },
                |payload, signature| {
                    Call::submit_offchain_pre_check_result(payload, signature)
                },
            )
            .ok_or(
                "❗ No local accounts accounts available, `ares` StoreKey needs to be set.",
            )?;
        result.map_err(|()| "⛔ Unable to submit transaction")?;
        Ok(())
    }

    fn save_create_pre_check_task(
        account_id: T::AccountId,
        stash_id: T::AccountId,
        auth_id: T::AuthorityAres,
        block_number: T::BlockNumber) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public:
            From<sp_application_crypto::sr25519::Public>,
    {
        let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
        // Singer
        let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
            .with_filter(sign_public_keys)
            .send_unsigned_transaction(
                |account| PerCheckPayload {
                    stash: stash_id.clone(),
                    auth: auth_id.clone(),
                    block_number,
                    public: account.public.clone(),
                },
                |payload, signature| {
                    Call::submit_create_pre_check_task(payload, signature)
                },
            )
            .ok_or(
                "❗ No local accounts accounts available, `ares` StoreKey needs to be set.",
            )?;
        result.map_err(|()| "⛔ Unable to submit transaction")?;
        Ok(())
    }

    fn save_fetch_ares_price_and_send_payload_signed(
        block_number: T::BlockNumber,
        account_id: T::AccountId,
    ) -> Result<(), &'static str>
    where
        <T as frame_system::offchain::SigningTypes>::Public:
            From<sp_application_crypto::sr25519::Public>,
    {
        let mut price_list = Vec::new();

        // Get raw request.
        let format_arr = Self::make_bulk_price_format_data(block_number);
        // Filter jump block info
        let (format_arr, jump_block) =
            Self::filter_jump_block_data(format_arr.clone(), account_id.clone(), block_number);

        let price_result =
            Self::fetch_bulk_price_with_http(format_arr)
                .ok();

        if price_result.is_none() {
            log::error!(
                target: "pallet::ocw::save_fetch_purchased_price_and_send_payload_signed",
                "⛔ Ocw network error."
            );
            return Ok(());
        }

        let price_result = price_result.unwrap();

        for (price_key, price_option, fraction_length, json_number_value) in price_result {
            if price_option.is_some() {
                // record price to vec!
                price_list.push(PricePayloadSubPrice(
                    price_key,
                    price_option.unwrap(),
                    fraction_length,
                    JsonNumberValue::new(json_number_value),
                ));
            }
        }

        log::debug!("🚅 fetch price count: {:?}, jump block count: {:?}", price_list.len(), jump_block.len());

        if price_list.len() > 0 || jump_block.len() > 0 {
            // -- Sign using any account
            // let mut sign_public_keys: Vec<<T as SigningTypes>::Public> = Vec::new();
            // let encode_data: Vec<u8> = account_id.encode();
            // assert!(32 == encode_data.len());
            // let raw_data = encode_data.try_into();
            // let raw_data = raw_data.unwrap();
            // // <T as SigningTypes>::Public::
            // // let new_account = T::AuthorityAres::unchecked_from(raw_data);
            // // <T as SigningTypes>::Public::unchecked_from(raw_data);
            // let new_account = sp_core::sr25519::Public::from_raw(raw_data);
            // sign_public_keys.push(new_account.into());

            let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());

            // Singer
            let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
            // let (_, result) = Signer::<T, T::AuthorityAres>::any_account()
                .with_filter(sign_public_keys)
                .send_unsigned_transaction(
                    |account| PricePayload {
                        price: price_list.clone(),
                        jump_block: jump_block.clone(),
                        block_number,
                        public: account.public.clone(),
                    },
                    |payload, signature| {
                        Call::submit_price_unsigned_with_signed_payload(payload, signature)
                    },
                )
                .ok_or(
                    "❗ No local accounts accounts available, `ares` StoreKey needs to be set.",
                )?;
            result.map_err(|()| "⛔ Unable to submit transaction")?;
        }
        Ok(())
    }

    //
    fn make_bulk_price_request_url(format: Vec<(Vec<u8>, Vec<u8>, FractionLength)>) -> (Vec<u8>, Vec<u8>) {
        // "http://141.164.58.241:5566/api/getBulkPrices".as_bytes().to_vec()
        // let raw_request_url = Self::make_local_storage_request_uri_by_vec_u8(
        //     "/api/getBulkPrices".as_bytes().to_vec(),
        // );
        let raw_request_url = Self::make_local_storage_request_uri_by_vec_u8(
            "/api/getBulkCurrencyPrices?currency=usdt".as_bytes().to_vec(),
        );

        let mut request_url = Vec::new();
        for (_, extract_key, _) in format {
            if request_url.len() == 0 {
                request_url = [
                    raw_request_url.clone(),
                    "&symbol=".as_bytes().to_vec(),
                    extract_key,
                ]
                .concat();
            } else {
                request_url = [request_url, "_".as_bytes().to_vec(), extract_key].concat();
            }
        }
        (request_url, "usdt".as_bytes().to_vec())
    }

    // Use to filter out those format_data of price that need to jump block.
    fn filter_jump_block_data(
        fomat_data: Vec<(Vec<u8>, Vec<u8>, FractionLength, RequestInterval)>,
        account: T::AccountId,
        block_number: T::BlockNumber,
    ) -> (
        Vec<(Vec<u8>, Vec<u8>, FractionLength)>,
        Vec<PricePayloadSubJumpBlock>,
    ) {
        // isNeedUpdateJumpBlock
        let mut new_format_data = Vec::new();
        let mut jump_format_data = Vec::new();

        fomat_data.iter().any(
            |(price_key, prase_key, fraction_length, request_interval)| {
                if Self::is_need_update_jump_block(price_key.clone(), account.clone()) {
                    jump_format_data.push(PricePayloadSubJumpBlock(
                        price_key.to_vec(),
                        *request_interval,
                    ));
                } else {
                    new_format_data.push((
                        price_key.to_vec(),
                        prase_key.to_vec(),
                        *fraction_length,
                    ));
                }
                false
            },
        );

        (new_format_data, jump_format_data)
    }

    // Judge the author who submitted the price last time, and return true if it is consistent with this time.
    fn is_need_update_jump_block(price_key: Vec<u8>, account: T::AccountId) -> bool {
        if !Self::is_aura() || 1 == T::AuthorityCount::get_validators_count(){
            return false;
        }
        match Self::get_last_price_author(price_key) {
            None => false,
            Some(x) => x == account,
        }
    }

    fn keystore_validator_member() -> Option<T::AccountId>
        where <T as frame_system::Config>::AccountId: From<sp_application_crypto::sr25519::Public>
    {
        let local_keys: Vec<<T as Config>::AuthorityAres> = T::AuthorityAres::all();
        if let Some(keystore_account) = local_keys.get(0) {
            let mut a = [0u8; 32];
            a[..].copy_from_slice(&keystore_account.to_raw_vec());
            // let local_account32 = AccountId32::new(a);
            // // local_account32.
            // let local_account: T::AccountId = local_account32.into();
            let new_account = sp_core::sr25519::Public::from_raw(a);
            let local_account: T::AccountId = new_account.into();
            if Self::is_validator_member(local_account.clone().into()) {
                return Some(local_account);
            }
        }
        None
    }

    fn is_validator_member(validator: T::ValidatorAuthority) -> bool {
        // let mut find_validator = !T::NeedVerifierCheck::get();
        let mut find_validator = !<OcwControlSetting<T>>::get().need_verifier_check;
        if false == find_validator {
            // check exists
            // let encode_data: Vec<u8> = payload.public.encode();
            // let validator_authority: T::ValidatorAuthority =
            //     <T as SigningTypes>::Public::into_account(payload.public.clone()).into();
            find_validator = T::VMember::is_member(&validator);
        }
        find_validator
    }

    // Store the list of authors in the price list.
    fn update_last_price_list_for_author(price_list: Vec<Vec<u8>>, author: T::AccountId) {
        price_list.iter().any(|price_key| {
            <LastPriceAuthor<T>>::insert(price_key.to_vec(), author.clone());
            false
        });
    }

    // Get
    fn get_last_price_author(price_key: Vec<u8>) -> Option<T::AccountId> {
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
        let a = 1;
        is_aura
    }

    // fn make_transaction_tag_prefix_with_public_account(mut perfix: Vec<u8>, account: <T as SigningTypes>::Public)-> Vec<u8> {
    //     let mut account_u8 = account.encode();
    //     perfix.append(&mut account_u8);
    //     perfix
    // }

    // Make bulk request format array.
    fn make_bulk_price_format_data(
        block_number: T::BlockNumber,
    ) -> Vec<(Vec<u8>, Vec<u8>, FractionLength, RequestInterval)> {
        let mut format = Vec::new();

        // price_key, request_url, parse_version, fraction_length
        let source_list = Self::get_raw_price_source_list();

        // In the new version, it is more important to control the request interval here.
        for (price_key, extract_key, parse_version, fraction_length, request_interval) in
            source_list
        {
            if 2 == parse_version {
                let mut round_number: u64 = block_number.into();
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
        format_arr: Vec<(Vec<u8>, Vec<u8>, u32)>,
    ) -> Result<
        Vec<(Vec<u8>, Option<u64>, FractionLength, NumberValue)>,
        http::Error,
    > {
        // make request url
        let (request_url, base_coin) = Self::make_bulk_price_request_url(format_arr.clone());
        let request_url = sp_std::str::from_utf8(&request_url).unwrap();

        // request and return http body.
        if "" == request_url {
            log::warn!(target: "pallet::ocw::fetch_bulk_price_with_http", "❗ Ares http requests cannot be empty.");
            return Ok(Vec::new());
        }
        log::info!(
            "🚅 Batch price request address: {:?}",
            request_url
        );
        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(4_000));
        let request = http::Request::get(request_url.clone());
        let pending = request
            .deadline(deadline)
            .send()
            .map_err(|_| http::Error::IoError)?;
        let response = pending.try_wait(deadline).map_err(|e| {
            log::warn!(
                target: "pallet::ocw::fetch_bulk_price_with_http",
                "❗ The network cannot connect. http::Error::DeadlineReached error = {:?}",
                e
            );
            http::Error::DeadlineReached
        })??;
        if response.code != 200 {
            log::warn!(
                target: "pallet::ocw::fetch_bulk_price_with_http",
                "❗ Unexpected http status code: {}",
                response.code
            );
            return Err(http::Error::Unknown);
        }
        let body = response.body().collect::<Vec<u8>>();
        // Create a str slice from the body.
        let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
            log::warn!(
                target: "pallet::ocw::fetch_bulk_price_with_http",
                "❗ Extracting body error, No UTF8 body!"
            );
            http::Error::Unknown
        })?;
        Ok(Self::bulk_parse_price_of_ares(body_str, base_coin, format_arr))
    }

    // /// Fetch current price and return the result in cents.
    // fn fetch_price_body_with_http(
    //     _price_key: Vec<u8>,
    //     request_url: &str,
    //     version_num: u32,
    //     fraction_length: FractionLength,
    // ) -> Result<u64, http::Error> {
    //     if "" == request_url {
    //         log::warn!("ERROR:: Cannot match area pricer url. ");
    //         return Err(http::Error::Unknown);
    //     }
    //
    //     log::info!("Go to fetch_price_of_ares on http.");
    //
    //     let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(4_000));
    //     let request = http::Request::get(request_url.clone());
    //     let pending = request
    //         .deadline(deadline)
    //         .send()
    //         .map_err(|_| http::Error::IoError)?;
    //     let response = pending.try_wait(deadline).map_err(|e| {
    //         log::warn!(
    //             "ERROR:: The network cannot connect. http::Error::DeadlineReached == {:?} ",
    //             e
    //         );
    //         http::Error::DeadlineReached
    //     })??;
    //     log::info!("The http server has returned a message.");
    //     // Let's check the status code before we proceed to reading the response.
    //     if response.code != 200 {
    //         log::warn!("ERROR:: Unexpected status code: {}", response.code);
    //         return Err(http::Error::Unknown);
    //     }
    //     let body = response.body().collect::<Vec<u8>>();
    //     // Create a str slice from the body.
    //     let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
    //         log::warn!("Error:: Extracting body, No UTF8 body");
    //         http::Error::Unknown
    //     })?;
    //
    //     log::info!("Parse ares json format.");
    //     match version_num {
    //         1 => {
    //             let price = match Self::parse_price_of_ares(body_str, fraction_length) {
    //                 Some(price) => Ok(price),
    //                 None => {
    //                     log::warn!("Unable to extract price from the response: {:?}", body_str);
    //                     Err(http::Error::Unknown)
    //                 }
    //             }?;
    //             log::info!("Get the price {:?} provided by Ares", price);
    //             Ok(price)
    //         }
    //         _ => Err(http::Error::Unknown),
    //     }
    // }

    // handler for bulk_parse_price_of_ares
    fn extract_bulk_price_by_json_value(
        json_val: JsonValue,
        find_key: &str,
        param_length: FractionLength,
    ) -> Option<(u64, NumberValue)> {
        assert!(
            param_length <= 6,
            "Fraction length must be less than or equal to 6"
        );

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
                    Some((_, sub_val)) => {
                        match sub_val {
                            JsonValue::Object(price_obj) => {
                                let (_, price) = price_obj
                                    .into_iter()
                                    .find(|(k, _)| {
                                        // println!("find(k) B = {:?}", k);
                                        // let tmp_k = k.iter().copied();
                                        k.iter().copied().eq("price".chars())
                                    })
                                    .unwrap();
                                // println!("price = {:?}", price);
                                match price {
                                    JsonValue::Number(number) => number,
                                    _ => return None,
                                }
                            }
                            _ => {
                                return None;
                            }
                        }
                    }
                }
            }
            _ => {
                return None;
            }
        };

        // Make u64 with fraction length
        // let result_price = Self::format_price_fraction_to_u64(price_value.clone(), param_length);
        // log::info!(" TO=DEBUG:: price::");
        let result_price = JsonNumberValue::new(price_value.clone()).toPrice(param_length);

        // A price of 0 means that the correct result of the data is not obtained.
        if result_price == 0 {
            return None;
        }
        Some((result_price, price_value))
    }

    /// format: ()
    fn bulk_parse_price_of_ares(
        price_str: &str,
        base_coin: Vec<u8>,
        format: Vec<(Vec<u8>, Vec<u8>, FractionLength)>,
    ) -> Vec<(Vec<u8>, Option<u64>, FractionLength, NumberValue)> {
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
                        // println!("v_data = {:?}", v_data);

                        for (price_key, extract_key, fraction_length) in format {
                            let new_extract_key = [extract_key, base_coin.clone()].concat();
                            let extract_key = sp_std::str::from_utf8(&new_extract_key).unwrap();
                            let extract_price_grp = Self::extract_bulk_price_by_json_value(
                                v_data.clone(),
                                extract_key,
                                fraction_length,
                            );
                            // println!(" extract_price = {:?}", extract_price);
                            if extract_price_grp.is_some() {
                                let (extract_price, json_number_value) = extract_price_grp.unwrap();
                                // TODO::need recheck why use Some(extract_price)
                                result_vec.push((
                                    price_key,
                                    Some(extract_price),
                                    fraction_length,
                                    json_number_value,
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
        assert!(
            param_length <= 6,
            "Fraction length must be less than or equal to 6"
        );

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
        Some(JsonNumberValue::new(price).toPrice(param_length))
    }

    // This function will to be remove. replace it JsonNumberValue toPrice()
    // fn format_price_fraction_to_u64 ( json_number: NumberValue, param_length: FractionLength) -> u64 {
    //     let mut price_fraction = json_number.fraction ;
    //     if price_fraction < 10u64.pow(param_length) {
    //         price_fraction *= 10u64.pow(param_length.checked_sub(json_number.fraction_length).unwrap_or(0));
    //     }
    //     let exp = json_number.fraction_length.checked_sub(param_length).unwrap_or(0);
    //     json_number.integer as u64 * (10u64.pow(param_length)) + (price_fraction / 10_u64.pow(exp))
    // }

    // Get price pool size
    fn get_price_pool_depth() -> u32 {
        // T::PriceVecMaxSize::get()
        <PricePoolDepth<T>>::get()
    }

    // Calculate the purchased amount, related to the amount of data
    // TODO:: will be replace by owc-finance.
    fn calculate_purchased_amount(unit_price: u64, request_keys: &Vec<Vec<u8>>) -> u64 {
        unit_price.saturating_mul(request_keys.len() as u64)
    }

    fn pay_to_purchase(who: T::AccountId, offer: u64) -> Result<(), Error<T>> {
        // TODO:: to be implement.
        Ok(().into())
    }


    fn filter_raw_price_source_list(request_data: Vec<Vec<u8>>)
        -> Vec<(Vec<u8>, Vec<u8>, FractionLength, RequestInterval)>
    {
        let source_list = Self::get_raw_price_source_list();
        let mut result = Vec::new();
        source_list.iter().any(|(price_key, parse_key, _version, fraction, interval)|{
            if request_data.iter().any(|x| x == price_key) {
                result.push((price_key.to_vec(), parse_key.to_vec(), *fraction, *interval));
            }
            false
        });
        result
    }

    fn purchased_storage_clean(p_id: Vec<u8>) {
        <PurchasedPricePool<T>>::remove_prefix(p_id.to_vec(), None);
        <PurchasedRequestPool<T>>::remove(p_id.to_vec());
        <PurchasedOrderPool<T>>::remove_prefix(p_id.to_vec(), None);
    }

    //
    fn fetch_purchased_request_keys(who: T::AccountId)
        -> Option<PurchasedSourceRawKeys> // Vec<(Vec<u8>, Vec<u8>, FractionLength, RequestInterval)>
    {
        let mut raw_source_keys = Vec::new();
        let mut raw_purchase_id: Vec<u8>= Vec::new();
        // Iter
        <PurchasedRequestPool<T>>::iter().any(|(mut purchase_id, request_data)| {
            if false == <PurchasedOrderPool<T>>::contains_key(purchase_id.clone(), who.clone()) {
                raw_purchase_id = purchase_id.clone();
                raw_source_keys = Self::filter_raw_price_source_list(request_data.request_keys)
                    .iter().map(|(price_key,parse_key,fraction_len,_)| {
                    (price_key.clone(), parse_key.clone(), *fraction_len)
                }).collect();
                return true;
            }
            false
        });

        if raw_source_keys.len() == 0 {
            return None;
        }

        Some(PurchasedSourceRawKeys { purchase_id: raw_purchase_id, raw_source_keys})
    }

    fn make_purchase_price_id(who: T::AccountId, add_up: u8) -> Vec<u8> {

        // check add up
        if add_up == u8::MAX {
            panic!("⛔ Add up number too large.");
        }

        let account32: AccountId32 = who.clone().into();
        let account_u8_32 = <AccountId32 as AsRef<[u8;32]>>::as_ref(&account32);
        let mut account_vec: Vec<u8> = account_u8_32.to_vec();

        // Get block number to u64
        let current_block_num:T::BlockNumber= <system::Pallet<T>>::block_number() ;
        let current_blocknumber: u64 = current_block_num.unique_saturated_into();
        let mut current_bn_vec: Vec<u8> = current_blocknumber.encode();
        account_vec.append(&mut current_bn_vec);
        // Get add up number
        let mut add_u8_vec: Vec<u8> = add_up.encode();
        account_vec.append(&mut add_u8_vec);

        // Check id exists.
        if <PurchasedRequestPool<T>>::contains_key(&account_vec) {
            return Self::make_purchase_price_id(who, add_up.saturating_add(1));
        }

        account_vec

        // let b = sp_core::sr25519::Public::from_raw(*a);
        // b.to_vec();
        // let str = sp_std::str::from_utf8(&b);
        // let purchase_id = HexDisplay::from(&account_vec.clone());
        // // log::info!("Hex = {:?}", &purchase_id);
        // println!("ID = {:?}",purchase_id.);
        // Vec::new()
    }

    // submit price on chain.
    fn ask_price (
        who: T::AccountId,
        offer: BalanceOf<T>,
        submit_threshold: u8,
        max_duration: u64,
        purchase_id: Vec<u8>,
        request_keys: Vec<Vec<u8>>,
    ) -> Result<Vec<u8>, Error<T>> {

        // Judge submit_threshold range is (0,100]
        if 0 >= submit_threshold || 100 < submit_threshold {
            return Err(Error::<T>::SubmitThresholdRangeError);
        }

        if 0 >= max_duration {
            return Err(Error::<T>::DruationNumberNotBeZero);
        }

        // make purchase price id
        // let purchase_id = Self::make_purchase_price_id(who.clone(), 0);

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

        Self::deposit_event(Event::NewPurchasedRequest(purchase_id.clone(), request_data, offer));
        // Self::deposit_event(Event::NewPurchasedRequest(purchase_id.clone(), request_data));

        Ok(purchase_id)
    }

    // add price on chain
    fn add_price(
        who: T::AccountId,
        price: u64,
        price_key: Vec<u8>,
        fraction_length: FractionLength,
        json_number_value: JsonNumberValue,
        max_len: u32,
    ) {
        // println!("add_price == {:?}", price);
        let key_str = price_key;
        let current_block = <system::Pallet<T>>::block_number();
        // 1. Check key exists
        if <AresPrice<T>>::contains_key(key_str.clone()) {
            // get and reset .
            let mut old_price = <AresPrice<T>>::get(key_str.clone());
            let mut is_fraction_changed = false;
            // check fraction length inconsistent.
            // for (index, (_, _, _, check_fraction, old_json_number_val)) in
            for (index, price_data) in
                old_price.clone().iter().enumerate()
            {
                if &price_data.fraction_len != &fraction_length {
                    // TODO:: Instead new funciton. !
                    // old_price.clear();
                    is_fraction_changed = true;
                    break;
                }
            }

            //
            let mut new_price = Vec::new();
            let MAX_LEN: usize = max_len.clone() as usize;

            for (index, value) in old_price.iter().enumerate() {
                if old_price.len() >= MAX_LEN && 0 == index {
                    continue;
                }

                let mut old_value = (*value).clone();
                if is_fraction_changed {
                    old_value = (*value).clone();
                    old_value.price = old_value.raw_number.toPrice(fraction_length.clone());
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
            });

            <AresPrice<T>>::insert(key_str.clone(), new_price);
        } else {
            // push a new value.
            let mut new_price: Vec<AresPriceData<T::AccountId, T::BlockNumber>> = Vec::new();
            // new_price.push((
            //     price.clone(),
            //     who.clone(),
            //     current_block,
            //     fraction_length,
            //     json_number_value,
            // ));
            new_price.push(AresPriceData{
                price: price.clone(),
                account_id: who.clone(),
                create_bn: current_block,
                fraction_len: fraction_length,
                raw_number: json_number_value,
            });
            <AresPrice<T>>::insert(key_str.clone(), new_price);
        }

        Self::check_and_update_avg_price_storage(key_str.clone(), max_len);

        // Check if the price request exists, if not, clear storage data.
        let price_request_list = <PricesRequests<T>>::get();
        let has_key = price_request_list
            .iter()
            .any(|(any_price, _, _, _, _)| any_price == &key_str);
        if !has_key {
            Self::clear_price_storage_data(key_str);
        }
    }

    fn check_and_update_avg_price_storage(key_str: Vec<u8>, max_len: u32) {
        let ares_price_list_len = <AresPrice<T>>::get(key_str.clone()).len();
        if ares_price_list_len >= max_len as usize && ares_price_list_len > 0 {
            // println!("update_avg_price_storage price count = {:?}", ares_price_list_len);
            Self::update_avg_price_storage(key_str.clone(), max_len);
        }
    }

    // TODO:: remove max_len ?
    fn update_avg_price_storage(key_str: Vec<u8>, max_len: u32) {
        let prices_info = <AresPrice<T>>::get(key_str.clone());
        let average_price_result =
            Self::average_price(prices_info, T::CalculationKind::get());

        if let Some((average, fraction_length)) = average_price_result {
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
                        <AresAbnormalPrice<T>>::append(key_str.clone(), check_price);
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
                    // TODO:: Refactoring recursion like handler_purchase_avg_price_storage
                    <AresPrice<T>>::insert(key_str.clone(), price_list_of_pool);
                    return Self::update_avg_price_storage(key_str.clone(), max_len.clone());
                }

                // Update avg price
                <AresAvgPrice<T>>::insert(key_str.clone(), (average, fraction_length));
                // Clear price pool.
                <AresPrice<T>>::remove(key_str.clone());
            }
        }
    }

    // to determine whether the submit price period has expired but there is still no submit.
    // This function returns a list of purchased_id.
    fn get_expired_purchased_transactions() -> Vec<Vec<u8>> {
        let a : PurchasedRequestData<T> ;
        let mut purchased_id_list: Vec<Vec<u8>> = Vec::new();
        let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();

        <PurchasedRequestPool<T>>::iter().any(|(p_id ,p_d  )|
            {
                if current_block >= p_d.max_duration {

                    purchased_id_list.push(p_id.to_vec());
                }
                false
            }
        );
        purchased_id_list
    }

    fn handler_purchase_avg_price_storage(purchase_id: Vec<u8>, price_key: Vec<u8>, mut prices_info: Vec<AresPriceData<T::AccountId, T::BlockNumber>>, reached_type: u8)
        -> Option<(Vec<u8>, PurchasedAvgPriceData, Vec<T::AccountId>)>
    {

        let (average, fraction_length) =
            Self::average_price(prices_info.clone(), T::CalculationKind::get())
                .expect("The average is not empty.");

        // TODO:: Combine duplicate codes
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
                    // TODO:: need update struct of AresAbnormalPrice , add the comparison value of the current deviation
                    <AresAbnormalPrice<T>>::append(price_key.clone(), check_price);
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
                // <AresPrice<T>>::insert(key_str.clone(), price_list_of_pool);
                // <PurchasedPricePool<T>>::insert(purchase_id.clone(), price_key.clone(), prices_info);
                return Self::handler_purchase_avg_price_storage(purchase_id.clone(), price_key.clone(), prices_info.clone(), reached_type);
            }

            // let current_block = <system::Pallet<T>>::block_number() as u64;
            let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();

            // get valid request price accounts
            let valid_request_account_id_list: Vec<T::AccountId> = prices_info.iter_mut().map(
                |x| {
                    x.account_id.clone()
                }
            ).collect();

            let avg_price_data = PurchasedAvgPriceData{
                create_bn: current_block,
                reached_type,
                price_data: (average.clone(), fraction_length.clone()),
            };
            // Update avg price (average, fraction_length)
            <PurchasedAvgPrice<T>>::insert(purchase_id.clone(),
                                           price_key.clone(),
                                           avg_price_data.clone(),
                                            );
            <PurchasedAvgTrace<T>>::insert(purchase_id.clone(),(<system::Pallet<T>>::block_number()));

            // Clear price pool.
            // <PurchasedPricePool<T>>::remove(purchase_id.clone(), price_key.clone());
            // // Clear purchased request.
            // <PurchasedRequestPool<T>>::remove(purchase_id.clone());

            return Some((price_key.clone(), avg_price_data, valid_request_account_id_list));
        }
        None
    }

    fn update_purchase_avg_price_storage(purchase_id: Vec<u8>, reached_type: u8) {
        // Get purchase price pool
        let price_key_list = <PurchasedPricePool<T>>::iter_key_prefix(purchase_id.clone()).collect::<Vec<_>>();

        //
        let mut event_result_list = Vec::new();
        //
        price_key_list.iter().any(|x| {
            let prices_info = <PurchasedPricePool<T>>::get(purchase_id.clone(), x.to_vec());
            let result = Self::handler_purchase_avg_price_storage(purchase_id.clone(),x.to_vec(), prices_info, reached_type);
            event_result_list.push(result);
            false
        });

        Self::deposit_event(Event::PurchasedAvgPrice(
            purchase_id.clone(),
            event_result_list,
        ));

        let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();
        Self::check_and_clear_expired_purchased_average_price_storage(purchase_id, current_block);
    }

    fn check_and_clear_expired_purchased_average_price_storage (purchase_id: Vec<u8>, current_block_num: u64) -> bool {
        if !<PurchasedAvgTrace<T>>::contains_key(&purchase_id) {
            return false;
        }

        // if let (avg_trace) = <PurchasedAvgTrace<T>>::get(purchase_id.clone()) {
        //     let avg_trace_num: u64 = avg_trace.unique_saturated_into();
        //     let purchase_setting = <PurchasedDefaultSetting<T>>::get();
        //     let comp_blocknum = purchase_setting.avg_keep_duration.saturating_add(avg_trace_num);
        //     if current_block_num > comp_blocknum {
        //         <PurchasedAvgPrice<T>>::remove_prefix(purchase_id.clone(), None);
        //         <PurchasedAvgTrace<T>>::remove(purchase_id.clone());
        //         return true;
        //     }
        // }

        let (avg_trace) = <PurchasedAvgTrace<T>>::get(purchase_id.clone()) ;
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

    // fn get_block_author() -> Option<<<T as pallet::Config>::ValidatorSet as frame_support::traits::ValidatorSet<<T as frame_system::Config>::AccountId>>::ValidatorId>
    // {
    //     let digest = <frame_system::Pallet<T>>::digest();
    //     let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
    //     let author = if let Some(author) = <T as pallet::Config>::FindAuthor::find_author(pre_runtime_digests) {
    //         author
    //     } else {
    //         log::info!("!!!!!!!!!!!!!! find author bad");
    //         Default::default()
    //     };
    //     log::info!("author index == {:?}", author);
    //
    //     let validators = T::VMember
    //     let author_index = author as usize;
    //     if validators.len() <= author_index {
    //         return None;
    //     }
    //
    //     Some(validators[author as usize])
    //
    //     // let encode_data = validators[author as usize];
    //     //
    //     // Some(encode_data.into())
    //     // let author_info = &validators[author as usize];
    //     // log::info!("author info == {:?}", author_info);
    // }

    //
    fn clear_price_storage_data(price_key: Vec<u8>) {
        <AresPrice<T>>::remove(&price_key);
        <AresAvgPrice<T>>::remove(&price_key);
    }

    /// Calculate current average price. // fraction_length: FractionLength
    fn average_price(prices_info: Vec<AresPriceData<T::AccountId, T::BlockNumber>>, kind: u8) -> Option<(u64, FractionLength)> {

        let mut fraction_length_of_pool: FractionLength = 0;
        // Check and get fraction_length.
        prices_info
            .clone()
            .into_iter()
            .any(|tmp_price_data| {
                if 0 == fraction_length_of_pool {
                    fraction_length_of_pool = tmp_price_data.fraction_len;
                }
                if fraction_length_of_pool != tmp_price_data.fraction_len {
                    panic!("Inconsistent of fraction lenght.")
                }
                false
            });

        // let prices: Vec<u64> = prices_info
        //     .into_iter()
        //     .map(|(price, _, _, _, _)| price)
        //     .collect();
        let prices: Vec<u64> = prices_info
            .into_iter()
            .map(|price_data| price_data.price)
            .collect();

        if prices.is_empty() {
            return None;
        }

        match Self::calculation_average_price(prices, kind) {
            Some(price_value) => {
                return Some((price_value, fraction_length_of_pool));
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
            Some(x) => jump_block.saturating_sub(1),
        };
        (interval, new_jump_block)
    }

    // Get validator count number
    fn handler_validator_count() -> u64 {
        T::AuthorityCount::get_validators_count()
    }


    fn add_purchased_price(purchase_id: Vec<u8>, account_id: T::AccountId, block_number: T::BlockNumber, price_list : Vec<PricePayloadSubPrice>) {

        if <PurchasedOrderPool<T>>::contains_key(
            purchase_id.clone(),
            account_id.clone(),
        ) {
            return ();
        }

        let current_block = <system::Pallet<T>>::block_number();

        price_list.iter().any(|PricePayloadSubPrice(a,b,c,d)|{
            let mut price_data_vec = <PurchasedPricePool<T>>::get(
                purchase_id.clone(),
                a.to_vec(),
            );
            price_data_vec.push(
                AresPriceData {
                    price: *b,
                    account_id: account_id.clone(),
                    create_bn: current_block,
                    fraction_len: *c,
                    raw_number: d.clone(),
                }
            );
            //
            <PurchasedPricePool<T>>::insert(
                purchase_id.clone(),
                a.to_vec(),
                price_data_vec,
            );
            false
        });

        <PurchasedOrderPool<T>>::insert(
            purchase_id.clone(),
            account_id.clone(),
            current_block.clone(),
        );

        Self::deposit_event(Event::NewPurchasedPrice(
            block_number.clone(),
            price_list.clone(),
            account_id.clone(),
        ));

    }

    // Judge whether the predetermined conditions of the validator of the current
    // purchased_id meet the requirements, and return true if it is
    fn is_validator_purchased_threshold_up_on(purchased_id: Vec<u8>) -> bool {
        if false == <PurchasedRequestPool<T>>::contains_key(purchased_id.clone()) {
            return false;
        }

        let reporter_count = <PurchasedOrderPool<T>>::iter_prefix_values(purchased_id.clone()).count();
        if 0 == reporter_count {
            return false;
        }

        let validator_count = T::AuthorityCount::get_validators_count();

        let reporter_count = reporter_count as u64;

        let div_val =(reporter_count * 100) / (validator_count ) ;


        let purchased_request = <PurchasedRequestPool<T>>::get(purchased_id.clone()) ;
        let submit_threshold = purchased_request.submit_threshold as u64;

        div_val >= submit_threshold
    }

    fn is_all_validator_submitted_price (purchased_id: Vec<u8>) -> bool {
        if false == <PurchasedRequestPool<T>>::contains_key(purchased_id.clone()) {
            return false;
        }

        let reporter_count = <PurchasedOrderPool<T>>::iter_prefix_values(purchased_id.clone()).count();
        if 0 == reporter_count {
            return false;
        }

        let validator_count = T::AuthorityCount::get_validators_count();

        (reporter_count as u64) >= validator_count
    }

    // Split purchased_request by the commas.
    fn extract_purchased_request(request: Vec<u8>) -> Vec<Vec<u8>> {
        // convert to str
        let purchased_str = sp_std::str::from_utf8(request.as_slice());
        if purchased_str.is_err() {
            return Vec::new();
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
                    result.push(x.as_bytes().to_vec());
                }
            }
        }
        result
    }

    // Increase jump block number, PARAM:: price_key, interval
    fn increase_jump_block_number(price_key: Vec<u8>, interval: u64) -> (u64, u64) {
        let (old_interval, jump_number) = Self::handle_calculate_jump_block((
            interval,
            Self::get_jump_block_number(price_key.clone()),
        ));
        <JumpBlockNumber<T>>::insert(&price_key, jump_number.clone());
        (old_interval, jump_number)
    }

    // Get jump block number
    fn get_jump_block_number(price_key: Vec<u8>) -> u64 {
        <JumpBlockNumber<T>>::get(&price_key).unwrap_or(0)
    }

    // delete a jump block info.
    fn remove_jump_block_number(price_key: Vec<u8>) {
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
            return Some(
                prices.iter().fold(0_u64, |a, b| a.saturating_add(*b)) / prices.len() as u64,
            );
        }
        None
    }

    fn validate_transaction_parameters_of_ares(
        block_number: &T::BlockNumber,
        // _price_list: Vec<(Vec<u8>, u64, FractionLength, JsonNumberValue)>,
        _price_list: Vec<PricePayloadSubPrice>,
    ) -> TransactionValidity {
        // // Now let's check if the transaction has any chance to succeed.
        // let next_unsigned_at = <NextUnsignedAt<T>>::get();
        // if &next_unsigned_at > block_number {
        //     return InvalidTransaction::Stale.into();
        // }

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

    fn update_reporter_point(purchase_id: Vec<u8>)->Result<(), Error<T>> {
        // transfer balance to pallet account.
        if T::OcwFinanceHandler::pay_to_ask(purchase_id.clone()).is_err() {
            return Err(Error::<T>::PayToPurchaseFeeFailed)
        }

        let request_mission: PurchasedRequestData<T> = <PurchasedRequestPool<T>>::get(purchase_id.clone());
        // update report work point
        <PurchasedOrderPool<T>>::iter_prefix(purchase_id.clone()).into_iter()
            .any(|(acc, _)| {
                T::OcwFinanceHandler::record_submit_point(acc,
                                                          purchase_id.clone(),
                                                          request_mission.create_bn,
                                                          request_mission.request_keys.len() as u32);
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


impl<T: Config> SymbolInfo for Pallet<T>
    where
        sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
        u64: From<<T as frame_system::Config>::BlockNumber>,
{
    fn price(symbol: &Vec<u8>) -> Result<(u64, FractionLength), ()> {
        AresAvgPrice::<T>::try_get(symbol)
    }

    fn fraction(symbol: &Vec<u8>) -> Option<FractionLength> {
        SymbolFraction::<T>::get(symbol)
    }
}

impl <T: Config> IAresOraclePreCheck<T::AccountId, T::AuthorityAres, T::BlockNumber> for Pallet<T>
    where sp_runtime::AccountId32: From<T::AccountId>,
      u64: From<T::BlockNumber>,
{
    //
    fn has_pre_check_task(stash: T::AccountId) -> bool {
        if let Some((_, check_status)) = Self::get_pre_check_status(stash.clone()) {
            match check_status {
                PreCheckStatus::Review => { return true ;}
                PreCheckStatus::Pass => { return true; }
                PreCheckStatus::Prohibit => { return true; }
            }
        }
        if <PreCheckTaskList<T>>::get().len() < 0 { return false; }
        let task_list = <PreCheckTaskList<T>>::get();
        task_list.iter().any(|(storage_stash, _, _,)|{
            &stash == storage_stash
        })
    }

    //
    fn get_pre_task_by_authority_set(auth_list: Vec<T::AuthorityAres>) -> Option<(T::AccountId, T::AuthorityAres, T::BlockNumber)> {
        let task_list = <PreCheckTaskList<T>>::get();
        for (stash, auth, bn ) in task_list {
            if auth_list.iter().any(|x|{
                x == &auth
            }) {
                // Check status
                match Self::get_pre_check_status(stash.clone()) {
                    None => {}
                    Some((_, pre_status)) => {
                        match pre_status {
                            PreCheckStatus::Review => {
                                return Some((stash, auth, bn));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    }

    //
    fn check_and_clean_obsolete_task(maximum_due: T::BlockNumber) -> Weight {
        let mut old_task_list = <PreCheckTaskList<T>>::get();
        if old_task_list.len() > 0 {
            let current_block_num:T::BlockNumber= <system::Pallet<T>>::block_number() ;
            let old_count = old_task_list.len();
            old_task_list.retain(|(_, _, bn)|{
                    let duration_bn = current_block_num - *bn;
                    duration_bn <= maximum_due
            });
            if old_count > old_task_list.len() {
                <PreCheckTaskList<T>>::put(old_task_list);
            }
        }
        0
    }

    // Obtain a set of price data according to the task configuration structure.
    fn take_price_for_per_check(check_config: PreCheckTaskConfig) -> Vec<PreCheckStruct> {

        let mut raw_price_source_list = Self::get_raw_price_source_list();
        raw_price_source_list.retain(|x| {
                check_config.check_token_list.clone()
                    .iter()
                    .any(|check_price_key| {
                        // println!("check_price_key == &x.0, {:?} == {:?}", sp_std::str::from_utf8(check_price_key), sp_std::str::from_utf8(&x.0)) ;
                        check_price_key == &x.0
                    })
        });

        let format_data: Vec<(Vec<u8>, Vec<u8>, FractionLength)> = raw_price_source_list.into_iter().map(|(price_key, parse_key, _version, fraction_len, _request_interval)|{
            (price_key, parse_key, fraction_len)
        }).collect();

        // // check_config
        let reponse_result= Self::fetch_bulk_price_with_http(format_data);
        let reponse_result = reponse_result.unwrap_or(Vec::new());

        reponse_result.into_iter().map(|(price_key,parse_key,fraction_len, number_val )|{
            let number_val = JsonNumberValue::new(number_val);
            PreCheckStruct {
                price_key,
                number_val,
                max_offset: check_config.allowable_offset.clone()
            }
        }).collect()
    }

    // Record the per check results and add them to the storage structure.
    fn save_pre_check_result(stash: T::AccountId, bn: T::BlockNumber, per_check_list: Vec<PreCheckStruct> ) {
        // get avg price.
        let check_result = per_check_list.iter().all(|checked_struct| {
            // Check price key exists.
            if !<AresAvgPrice<T>>::contains_key(&checked_struct.price_key) {
                return false;
            }
            // Get avg price struct.
            let (avg_price_val, avg_fraction_len) = <AresAvgPrice<T>>::get(&checked_struct.price_key);

            let max_price = checked_struct.number_val.toPrice(avg_fraction_len).max(avg_price_val);
            let min_price = checked_struct.number_val.toPrice(avg_fraction_len).min(avg_price_val);
            // println!("max_price={}, min_price={}", max_price, min_price);
            let diff = max_price - min_price;
            // println!("diff <= checked_struct.max_offset * avg_price_val = {} <= {}", diff, checked_struct.max_offset * avg_price_val);
            diff <= checked_struct.max_offset * avg_price_val
        });

        let mut per_checkstatus = PreCheckStatus::Prohibit;
        if check_result {
            per_checkstatus = PreCheckStatus::Pass;
        }

        log::debug!(" ------ debug . on RUN A5");
        <FinalPerCheckResult<T>>::insert(stash.clone(), Some((bn, per_checkstatus)));
        let mut task_list = <PreCheckTaskList<T>>::get();
        task_list.retain(|(old_acc, _, _)|{ &stash != old_acc }) ;
        <PreCheckTaskList<T>>::put(task_list);

        log::debug!(" ------ debug . on RUN A6");
    }

    //
    fn get_pre_check_status(stash: T::AccountId) -> Option<(T::BlockNumber, PreCheckStatus)> {
        <FinalPerCheckResult<T>>::get(stash).unwrap_or(None)
    }

    //
    fn create_pre_check_task(stash: T::AccountId, auth: T::AuthorityAres, bn: T::BlockNumber) -> bool {
        let mut task_list = <PreCheckTaskList<T>>::get();
        let exists = task_list.iter().any(|(old_acc, _, _)|{ &stash == old_acc }) ;
        if exists {
            return false;
        }
        task_list.push((stash.clone(), auth, bn));
        <PreCheckTaskList<T>>::put(task_list);
        <FinalPerCheckResult<T>>::insert(stash.clone(), Some((bn, PreCheckStatus::Review)));
        true
    }
}


