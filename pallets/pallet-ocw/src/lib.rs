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

use frame_support::sp_std::str::FromStr;
use frame_support::traits::{FindAuthor, Get, ValidatorSet};
use serde::{Deserialize, Deserializer};
use sp_std::{prelude::*, str};

#[cfg(test)]
mod tests;

pub mod aura_handler;

/// The keys can be inserted manually via RPC (see `author_insertKey`).
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"ares"); // sp_application_crypto::key_types::BABE ; //
pub const LOCAL_STORAGE_PRICE_REQUEST_MAKE_POOL: &[u8] = b"are-ocw::make_price_request_pool";
pub const LOCAL_STORAGE_PRICE_REQUEST_LIST: &[u8] = b"are-ocw::price_request_list";
pub const LOCAL_STORAGE_PRICE_REQUEST_DOMAIN: &[u8] = b"are-ocw::price_request_domain";
pub const CALCULATION_KIND_AVERAGE: u8 = 1;
pub const CALCULATION_KIND_MEDIAN: u8 = 2;

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
use frame_support::sp_runtime::app_crypto::{Public, TryFrom};
use frame_support::sp_runtime::sp_std::convert::TryInto;
use frame_support::sp_runtime::traits::AccountIdConversion;
use frame_system::offchain::{SendUnsignedTransaction, Signer};
use lite_json::NumberValue;
pub use pallet::*;
use sp_application_crypto::sp_core::crypto::UncheckedFrom;
use sp_consensus_aura::AURA_ENGINE_ID;
use sp_runtime::offchain::storage::StorageValueRef;
use sp_runtime::offchain::storage_lock::{BlockAndTime, StorageLock};

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_support::sp_runtime::traits::{IdentifyAccount, IsMember};
    use frame_support::sp_std::convert::TryInto;
    use frame_system::pallet_prelude::*;
    use sp_core::crypto::UncheckedFrom;

    #[pallet::error]
    pub enum Error<T> {
        //
        SubmitThresholdRangeError,
        DruationNumberNotBeZero,
        UnknownAresPriceVersionNum,
    }

    /// This pallet's configuration trait
    #[pallet::config]
    pub trait Config: CreateSignedTransaction<Call<Self>> + frame_system::Config
    where
        sp_runtime::AccountId32: From<<Self as frame_system::Config>::AccountId>,
        u64: From<<Self as frame_system::Config>::BlockNumber>,
    {
        /// The identifier type for an offchain worker.
        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;

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

        #[pallet::constant]
        type NeedVerifierCheck: Get<bool>;

        // Used to confirm RequestPropose.
        type RequestOrigin: EnsureOrigin<Self::Origin>;

        #[pallet::constant]
        type FractionLengthNum: Get<u32>;

        #[pallet::constant]
        type CalculationKind: Get<u8>;

        type AuthorityCount: ValidatorCount;
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
        <T as frame_system::offchain::SigningTypes>::Public:
            From<sp_application_crypto::sr25519::Public>,
    {
        /// You can use `Local Storage` API to coordinate runs of the worker.
        fn offchain_worker(block_number: T::BlockNumber) {
            let block_author = Self::get_block_author();
            log::info!("❗❗❗ LINDEBUG:: author count = {:?}", T::AuthorityCount::get_validators_count());

            match block_author {
                None => {
                    log::warn!(target: "pallet::ocw::offchain_worker", "❗ Not found author.");
                }
                Some(author) => {
                    log::info!("❗❗❗ LINDEBUG , {:?}", HexDisplay::from(&Self::make_purchase_price_id(author.clone(), 0)));
                    log::info!("🚅 ❗ ⛔ Ocw offchain start {:?} ", &author);
                    // if Self::are_block_author_and_sotre_key_the_same(<pallet_authorship::Pallet<T>>::author()) {
                    if Self::are_block_author_and_sotre_key_the_same(author.clone()) {
                        // Try to get ares price.
                        match Self::ares_price_worker(block_number, author) {
                            Ok(v) => log::info!("🚅 Ares OCW price acquisition completed."),
                            Err(e) => log::warn!(
                                target: "pallet::ocw::offchain_worker",
                                "❗ Ares price has a problem : {:?}",
                                e
                            ),
                        }
                    }
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
        #[pallet::weight(0)]
        pub fn submit_price_unsigned_with_signed_payload(
            origin: OriginFor<T>,
            price_payload: PricePayload<T::Public, T::BlockNumber>,
            _signature: T::Signature,
        ) -> DispatchResultWithPostInfo {
            // This ensures that the function can only be called via unsigned transaction.
            ensure_none(origin)?;

            log::info!(
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

            log::info!(
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

            log::info!(
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
        pub fn revoke_request_propose(origin: OriginFor<T>, price_key: Vec<u8>) -> DispatchResult {
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
        pub fn request_propose(
            origin: OriginFor<T>,
            price_key: Vec<u8>,
            request_url: Vec<u8>,
            parse_version: u32,
            fraction_num: FractionLength,
            request_interval: RequestInterval,
        ) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;

            <PricesRequests<T>>::mutate(|prices_request| {
                let mut find_old = false;
                for (
                    index,
                    (old_price_key, _old_request_url, _old_parse_version, old_fraction_count, _),
                ) in prices_request.clone().into_iter().enumerate()
                {
                    if &price_key == &old_price_key {
                        if &"".as_bytes().to_vec() != &request_url {
                            // add input value
                            prices_request.push((
                                price_key.clone(),
                                request_url.clone(),
                                parse_version,
                                fraction_num.clone(),
                                request_interval.clone(),
                            ));
                            Self::deposit_event(Event::UpdatePriceRequest(
                                price_key.clone(),
                                request_url.clone(),
                                parse_version.clone(),
                                fraction_num.clone(),
                            ));
                        }

                        // remove old one
                        prices_request.remove(index);

                        // check fraction number on change
                        if &old_fraction_count != &fraction_num
                            || &"".as_bytes().to_vec() == &request_url
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
                        request_url.clone(),
                        parse_version.clone(),
                        fraction_num.clone(),
                        request_interval.clone(),
                    ));
                    Self::deposit_event(Event::AddPriceRequest(
                        price_key,
                        request_url,
                        parse_version,
                        fraction_num,
                    ));
                }
            });

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn allowable_offset_propose(origin: OriginFor<T>, offset: u8) -> DispatchResult {
            T::RequestOrigin::ensure_origin(origin)?;
            <PriceAllowableOffset<T>>::put(offset);
            Self::deposit_event(Event::PriceAllowableOffsetUpdate(offset));

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn pool_depth_propose(origin: OriginFor<T>, depth: u32) -> DispatchResult {
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
        // Average price update.
        RevokePriceRequest(Vec<u8>),
        AddPriceRequest(Vec<u8>, Vec<u8>, u32, FractionLength),
        UpdatePriceRequest(Vec<u8>, Vec<u8>, u32, FractionLength),
        //
        PricePoolDepthUpdate(u32),
        //
        PriceAllowableOffsetUpdate(u8),
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T>
    where
        sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
        u64: From<<T as frame_system::Config>::BlockNumber>,
    {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            if let Call::submit_price_unsigned_with_signed_payload(ref payload, ref signature) =
                call
            {
                log::info!(
                    "🚅 Validate price payload data, on block: {:?} ",
                    <system::Pallet<T>>::block_number()
                );
                let mut find_validator = !T::NeedVerifierCheck::get();
                if false == find_validator {
                    // check exists
                    // let encode_data: Vec<u8> = payload.public.encode();
                    let validator_authority: T::ValidatorAuthority =
                        <T as SigningTypes>::Public::into_account(payload.public.clone()).into();
                    find_validator = T::VMember::is_member(&validator_authority);
                }

                if !find_validator {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Payload public id is no longer in the members. `InvalidTransaction`"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                let signature_valid =
                    SignedPayload::<T>::verify::<T::AuthorityId>(payload, signature.clone());

                if !signature_valid {
                    log::error!(
                        target: "pallet::ocw::validate_unsigned",
                        "⛔️ Payload public id is no longer in the members. `InvalidTransaction`"
                    );
                    return InvalidTransaction::BadProof.into();
                }

                Self::validate_transaction_parameters_of_ares(
                    &payload.block_number,
                    payload.price.to_vec(),
                )
            } else {
                InvalidTransaction::Call.into()
            }
        }
    }

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
        // price, account, bolcknumber, FractionLength, JsonNumberValue
        // Vec<(
        //     u64,
        //     T::AccountId,
        //     T::BlockNumber,
        //     FractionLength,
        //     JsonNumberValue,
        // )>,
        Vec<AresPriceData<T>>,
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
        Vec<AresPriceData<T>>,
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
            Vec<u8>, // request url
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
            }
        }
    }

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
            }
            if self.price_pool_depth > 0 {
                PricePoolDepth::<T>::put(&self.price_pool_depth);
            }
            if self.request_base.len() > 0 {
                RequestBaseOnchain::<T>::put(&self.request_base);
                // storage write
                // let mut storage_request_base = StorageValueRef::persistent(LOCAL_STORAGE_PRICE_REQUEST_DOMAIN);
                // storage_request_base.set(&self.request_base);
            }
            PriceAllowableOffset::<T>::put(self.price_allowable_offset);
        }
    }
}

pub mod structs;
use structs::*;
use hex;
use sp_runtime::traits::UniqueSaturatedInto;
use sp_core::hexdisplay::HexDisplay;


impl<T: Config> Pallet<T>
where
    sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
    u64: From<<T as frame_system::Config>::BlockNumber>,
{
    fn are_block_author_and_sotre_key_the_same(block_author: T::AccountId) -> bool {
        let mut is_same = !T::NeedVerifierCheck::get(); // Self::get_default_author_save_bool();
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

    // Get the number of cycles required to loop the array lenght.
    // If round_num = 0 returns the maximum value of u8
    fn get_number_of_cycles(vec_len: u8, round_num: u8) -> u8 {
        if round_num == 0 {
            return u8::MAX;
        }
        let mut round_offset = 0u8;
        if vec_len % round_num != 0 {
            round_offset = 1u8;
        }
        vec_len / round_num + round_offset
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

        let (price_result, jump_block) =
            Self::fetch_bulk_price_with_http(block_number, account_id.clone(), 2)
                .ok()
                .unwrap();

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

        log::info!("🚅 fetch price count: {:?}, jump block count: {:?}", price_list.len(), jump_block.len());

        if price_list.len() > 0 || jump_block.len() > 0 {
            // -- Sign using any account
            let mut sign_public_keys: Vec<<T as SigningTypes>::Public> = Vec::new();

            let encode_data: Vec<u8> = account_id.encode();
            assert!(32 == encode_data.len());
            let raw_data = encode_data.try_into();
            let raw_data = raw_data.unwrap();
            // <T as SigningTypes>::Public::
            let new_account = T::AuthorityAres::unchecked_from(raw_data);
            // <T as SigningTypes>::Public::unchecked_from(raw_data);
            let new_account = sp_core::sr25519::Public::from_raw(raw_data);
            sign_public_keys.push(new_account.into());

            // Singer
            let (_, result) = Signer::<T, T::AuthorityId>::any_account()
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
    fn make_bulk_price_request_url(format: Vec<(Vec<u8>, Vec<u8>, FractionLength)>) -> Vec<u8> {
        // "http://141.164.58.241:5566/api/getBulkPrices".as_bytes().to_vec()
        let raw_request_url = Self::make_local_storage_request_uri_by_vec_u8(
            "/api/getBulkPrices".as_bytes().to_vec(),
        );
        let mut request_url = Vec::new();
        for (_, extract_key, _) in format {
            if request_url.len() == 0 {
                request_url = [
                    raw_request_url.clone(),
                    "?symbol=".as_bytes().to_vec(),
                    extract_key,
                ]
                .concat();
            } else {
                request_url = [request_url, "_".as_bytes().to_vec(), extract_key].concat();
            }
        }
        request_url
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
        if !Self::is_aura() {
            return false;
        }
        match Self::get_last_price_author(price_key) {
            None => false,
            Some(x) => x == account,
        }
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
        log::info!("🚅 Ares will be request list count {:?}", format.len());
        format
    }

    //
    fn fetch_bulk_price_with_http(
        block_number: T::BlockNumber,
        account_id: T::AccountId,
        version_num: u8,
    ) -> Result<
        (
            Vec<(Vec<u8>, Option<u64>, FractionLength, NumberValue)>,
            Vec<PricePayloadSubJumpBlock>,
        ),
        http::Error,
    > {
        // Get current available price list.
        let format_arr = Self::make_bulk_price_format_data(block_number);

        // Filter format arr, separate jump block data.
        let (format_arr, jump_arr) =
            Self::filter_jump_block_data(format_arr.clone(), account_id.clone(), block_number);

        // make request url
        let request_url = Self::make_bulk_price_request_url(format_arr.clone());
        let request_url = sp_std::str::from_utf8(&request_url).unwrap();

        // request and return http body.
        if "" == request_url {
            log::warn!(target: "pallet::ocw::fetch_bulk_price_with_http", "❗ Ares http requests cannot be empty.");
            return Ok((Vec::new(), jump_arr));
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
        Ok((
            Self::bulk_parse_price_of_ares(body_str, format_arr),
            jump_arr,
        ))
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
                            let extract_key = sp_std::str::from_utf8(&extract_key).unwrap();
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
    fn calculate_purchased_amount(request_keys: &Vec<Vec<u8>>) -> u64 {
        // TODO:: to be implement.
        100
    }

    fn pay_to_purchase(who: T::AccountId, offer: u64) -> Result<(), Error<T>> {
        // TODO:: to be implement.
        Ok(().into())
    }

    fn make_purchase_price_id(who: T::AccountId, add_up: u8) -> Vec<u8> {
        // TODO:: to be implement.

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
        account_vec

        // Check id exists.
        // <<T
        // TODO::deving

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
        offer: u64,
        submit_threshold: u8,
        max_duration: u64,
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
        let purchase_id = Self::make_purchase_price_id(who, 254);

        // println!("purchase_id = {:?}", purchase_id);
        // check whether purchase_id exists.


        Err(Error::<T>::UnknownAresPriceVersionNum)
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
                    // old_value = match (*value).clone() {
                    //     (
                    //         old_price,
                    //         account_id,
                    //         b_number,
                    //         old_fraction_lenght,
                    //         json_number_value,
                    //     ) => (
                    //         json_number_value.toPrice(fraction_length.clone()),
                    //         account_id,
                    //         b_number,
                    //         fraction_length.clone(),
                    //         json_number_value,
                    //     ),
                    // };
                    old_value = (*value).clone();
                    old_value.price = old_value.raw_number.toPrice(fraction_length.clone());
                    old_value.fraction_len = fraction_length.clone();
                }

                // let new_value = old_value;
                new_price.push(old_value);
            }

            // new_price.push((
            //     price.clone(),
            //     who.clone(),
            //     current_block,
            //     fraction_length,
            //     json_number_value,
            // ));

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
            let mut new_price: Vec<AresPriceData<T>> = Vec::new();
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

        // Get current block number for test.
        // let current_block = <system::Pallet<T>>::block_number();

        // <PricesTrace<T>>::mutate(|prices_trace| {
        //     // let author = <pallet_authorship::Pallet<T>>::author();
        //     let author = Self::get_block_author().unwrap();
        //     let MAX_LEN: usize = max_len.clone() as usize;
        //     let price_trace_len = prices_trace.len();
        //     if price_trace_len < MAX_LEN {
        //         prices_trace.push((price.clone(), author.clone(), who.clone()));
        //     } else {
        //         prices_trace[price_trace_len % MAX_LEN] = (price.clone(), author.clone(), who.clone());
        //     }
        // });

        // // Check price pool deep reaches the maximum value, and if so, calculated the average.
        // if  <AresPrice<T>>::get(key_str.clone()).len() >= max_len as usize {
        //     let average = Self::average_price(key_str.clone(), T::CalculationKind::get())
        //         .expect("The average is not empty.");
        //     log::info!("Calculate current average price average price is: ({},{}) , {:?}", average, fraction_length, &key_str);
        //     // Update avg price
        //     <AresAvgPrice<T>>::insert(key_str.clone(), (average, fraction_length));
        // }else{
        //     <AresAvgPrice<T>>::insert(key_str.clone(), (0, 0));
        // }

        // let ares_price_list_len = <AresPrice<T>>::get(key_str.clone()).len();
        // if ares_price_list_len >= max_len as usize && ares_price_list_len > 0 {
        //     println!("update_avg_price_storage price count = {:?}", ares_price_list_len);
        //     Self::update_avg_price_storage(key_str.clone(), max_len);
        // }

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
        // Check price pool deep reaches the maximum value, and if so, calculated the average.
        // if  <AresPrice<T>>::get(key_str.clone()).len() >= max_len as usize {

        let (average, fraction_length) =
            Self::average_price(key_str.clone(), T::CalculationKind::get())
                .expect("The average is not empty.");

        // log::info!(
        //     "Calculate current average price average price is: ({},{}) , {:?}",
        //     average,
        //     fraction_length,
        //     &key_str
        // );

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
                    price_list_of_pool.remove((*remove_index - remove_count));
                    remove_count += 1;
                    false
                });
                // reset price pool
                <AresPrice<T>>::insert(key_str.clone(), price_list_of_pool);
                return Self::update_avg_price_storage(key_str.clone(), max_len.clone());
            }

            // Update avg price
            <AresAvgPrice<T>>::insert(key_str.clone(), (average, fraction_length));
            // Clear price pool.
            <AresPrice<T>>::remove(key_str.clone());
        }
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
    fn average_price(price_key_str: Vec<u8>, kind: u8) -> Option<(u64, FractionLength)> {
        let prices_info = <AresPrice<T>>::get(price_key_str.clone());
        let mut fraction_length_of_pool: FractionLength = 0;
        // Check and get fraction_length.
        prices_info
            .clone()
            .into_iter()
            // .any(|(_, _, _, tmp_fraction, _)| {
            //     if 0 == fraction_length_of_pool {
            //         fraction_length_of_pool = tmp_fraction;
            //     }
            //     if fraction_length_of_pool != tmp_fraction {
            //         panic!("Inconsistent of fraction lenght.")
            //     }
            //     false
            // });
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

        ValidTransaction::with_tag_prefix("pallet-ocw::validate_transaction_parameters_of_ares")
            .priority(T::UnsignedPriority::get())
            .and_provides(block_number) // next_unsigned_at
            .longevity(5)
            .propagate(true)
            .build()
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
