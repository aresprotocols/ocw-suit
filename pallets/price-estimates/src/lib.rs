#![cfg_attr(not(feature = "std"), no_std)]

mod tests;
pub mod types;

use frame_system::{
    offchain::{
        AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SignedPayload, Signer,
        SigningTypes,
    },
    pallet_prelude::*,
};

#[cfg(feature = "std")]
use frame_support::traits::GenesisBuild;

use types::{
    is_hex_address, AccountParticipateEstimates, ChooseWinnersPayload, EstimatesState,
    EstimatesType, MultiplierOption, SymbolEstimatesConfig,
};

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{
        BalanceStatus, ChangeMembers, Currency, ExistenceRequirement, FindAuthor, Imbalance,
        IsSubType, LockIdentifier, LockableCurrency, NamedReservableCurrency, OnUnbalanced,
        ReservableCurrency, WithdrawReasons,
    },
    weights::{DispatchClass, Weight},
    PalletId, StorageHasher,
};
use log;
pub use pallet::*;
use sp_core::hexdisplay::HexDisplay;
use sp_std::str;
use sp_std::vec;
use sp_std::vec::Vec;

use sp_runtime::{
    offchain::{http, Duration},
    traits::{AccountIdConversion, IdentifyAccount, SaturatedConversion, StaticLookup},
    transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
    AccountId32, FixedU128, Perbill, Permill, Rational128, RuntimeAppPublic, RuntimeDebug,
};

use ares_oracle::{traits::SymbolInfo, types::FractionLength};
use sp_core::sp_std::convert::TryInto;

type BalanceOf<T, I = ()> =
<<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T, I = ()>(PhantomData<(T, I)>);

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config<I: 'static = ()>:
    CreateSignedTransaction<Call<Self, I>> + frame_system::Config
    {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

        /// The Lottery's pallet id
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        #[pallet::constant]
        type MaxEstimatesPerSymbol: Get<u32>;

        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        type Currency: Currency<Self::AccountId>
        + NamedReservableCurrency<Self::AccountId, ReserveIdentifier = [u8; 8]>;

        type Call: From<Call<Self, I>>;

        type PriceProvider: SymbolInfo;

        type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
    }

    // Map<Symbol, Index> Manager Symbol Estimates Id
    #[pallet::storage]
    pub type SymbolEstimatesId<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Identity,
        Vec<u8>, //symbol
        u64,     //id
    >;

    #[pallet::storage]
    pub type SymbolRewardPool<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Identity,
        Vec<u8>,         //symbol
        BalanceOf<T, I>, //id
    >;

    #[pallet::storage]
    pub type LockedEstimates<T: Config<I>, I: 'static = ()> =
    StorageValue<_, T::BlockNumber, ValueQuery>;

    #[pallet::storage]
    pub type MinimumTicketPrice<T: Config<I>, I: 'static = ()> =
    StorageValue<_, BalanceOf<T, I>, ValueQuery>;

    #[pallet::storage]
    pub type ActivePallet<T: Config<I>, I: 'static = ()> = StorageValue<_, bool, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn admin_members)]
    pub type AdminMembers<T: Config<I>, I: 'static = ()> =
    StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn unsigned_members)]
    pub type UnsignedMembers<T: Config<I>, I: 'static = ()> =
    StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    pub type PreparedEstimates<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Identity,
        Vec<u8>,                                                // symbol
        SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>, // config
    >;

    #[pallet::storage]
    pub type ActiveEstimates<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Identity,
        Vec<u8>,                                                // symbol
        SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>, // config
    >;

    #[pallet::storage]
    pub type CompletedEstimates<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Identity,
        Vec<u8>,                                                     // symbol
        Vec<SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>>, // configs
    >;

    // Map<SymbolEstimatesConfig, Vec<AccountSymbolEstimates>>
    #[pallet::storage]
    pub type Participants<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Identity,
        Vec<u8>, // symbol
        Identity,
        u64, // id
        Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>>,
    >;

    #[pallet::storage]
    pub type Winners<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Identity,
        Vec<u8>, // symbol
        Identity,
        u64, // id
        Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>>,
    >;

    #[pallet::storage]
    pub type AccountInfo<T: Config<I>, I: 'static = ()> = StorageMap<
        _,
        Identity,
        T::AccountId,        // user
        Vec<(Vec<u8>, u64)>, // vec<symbol+id>
    >;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        Deposit(T::AccountId, BalanceOf<T, I>),

        Reserved([u8; 8], T::AccountId, BalanceOf<T, I>),

        NewEstimates(Vec<u8>, T::AccountId),
    }

    #[pallet::error]
    pub enum Error<T, I = ()> {
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
    }

    // #[pallet::genesis_config]
    // pub struct GenesisConfig<T: Config<I>, I: 'static = ()> {
    //     pub locked_estimates: T::BlockNumber,
    //     pub minimum_ticket_price: BalanceOf<T, I>,
    //     pub admin_members: Vec<T::AccountId>,
    // }
    //
    // #[cfg(feature = "std")]
    // impl<T: Config<I>, I: 'static> Default for GenesisConfig<T, I> {
    //     fn default() -> Self {
    //         Self {
    //             locked_estimates: T::BlockNumber::from(1800u8),
    //             minimum_ticket_price: <BalanceOf<T, I>>::from(0u32),
    //             admin_members: vec![],
    //         }
    //     }
    // }
    //
    // #[pallet::genesis_build]
    // impl<T: Config<I>, I: 'static> GenesisBuild<T, I> for GenesisConfig<T, I> {
    //     fn build(&self) {
    //         LockedEstimates::<T, I>::put(&self.locked_estimates);
    //         AdminMembers::<T, I>::put(&self.admin_members);
    //     }
    // }

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
        fn on_initialize(n: T::BlockNumber) -> Weight {
            if !Self::is_active() {
                return 0;
            }
            let mut to_active_symbol: Vec<Vec<u8>> = vec![];
            PreparedEstimates::<T, I>::iter().for_each(|(_symbol, config)| {
                if config.start <= n {
                    log::info!("symbol: {:?} to active .. config: {:?}", &_symbol, config);
                    to_active_symbol.push(_symbol);
                }
            });
            to_active_symbol.iter().for_each(|_symbol| {
                if !ActiveEstimates::<T, I>::contains_key(&_symbol) {
                    let mut config = PreparedEstimates::<T, I>::get(&_symbol);
                    let mut config = config.as_mut().unwrap();
                    config.state = EstimatesState::Active;
                    PreparedEstimates::<T, I>::remove(&_symbol);
                    ActiveEstimates::<T, I>::insert(&_symbol, config)
                }
            });
            0
        }

        fn offchain_worker(now: T::BlockNumber) {
            use sp_runtime::offchain::storage_lock::{BlockAndTime, StorageLock};
            if !Self::is_active() {
                return;
            }
            if !Self::can_submit_unsigned() {
                log::info!("can not run offchian worker...");
                return;
            }
            // Create a lock with the maximum deadline of number of blocks in the unsigned phase.
            // This should only come useful in an **abrupt** termination of execution, otherwise the
            // guard will be dropped upon successful execution.
            let mut lock =
                StorageLock::<BlockAndTime<frame_system::Pallet<T>>>::with_block_deadline(
                    b"price-estimates",
                    10,
                );

            match lock.try_lock() {
                Ok(_guard) => {
                    Self::do_synchronized_offchain_worker(now);
                }
                Err(deadline) => {
                    log::error!(
                        "offchain worker lock not released, deadline is {:?}",
                        deadline
                    );
                }
            };
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config<I>, I: 'static> ValidateUnsigned for Pallet<T, I> {
        type Call = Call<T, I>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            if !Self::is_active() {
                return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
            }
            if let Call::choose_winner_with_signed_payload(ref payload, ref _signature) = call {
                let symbol = &payload.symbol;
                let round_id = *(&payload.estimates_id.clone());
                log::info!(
                    "validate_unsigned-->estimates_price, on block: {:?}, symbol: {:?}, round_id: {:?} ",
                    frame_system::Pallet::<T>::block_number(),
                    sp_std::str::from_utf8(symbol).unwrap(), &round_id
                );

                let config = ActiveEstimates::<T, I>::get(&symbol);
                if config.is_none() {
                    return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
                }
                let config = config.unwrap();
                if round_id != config.id {
                    log::warn!("round id is not equal");
                    return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
                }

                let members = UnsignedMembers::<T, I>::try_get();
                if members.is_err() {
                    log::warn!("can not found any unsigned members");
                    return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
                }
                let members = members.unwrap();
                let signed_account: T::AccountId = payload.public.clone().into_account();
                if !members.contains(&signed_account) {
                    log::warn!("signed_account is not  member");
                    return Err(TransactionValidityError::Invalid(InvalidTransaction::Call));
                }
                ValidTransaction::with_tag_prefix("ares-estimates")
                    .priority(T::UnsignedPriority::get())
                    // .and_provides(payload.public.clone())
                    .and_provides(symbol)
                    .longevity(5)
                    .propagate(true)
                    .build()
            } else {
                // InvalidTransaction::Call.into()
                Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
            }
        }
    }

    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn new_estimates(
            origin: OriginFor<T>,
            symbol: Vec<u8>,
            start: T::BlockNumber,
            end: T::BlockNumber,
            distribute: T::BlockNumber,
            estimates_type: EstimatesType,
            deviation: Option<Permill>,
            range: Option<Vec<u64>>,
            #[pallet::compact] price: BalanceOf<T, I>,
        ) -> DispatchResult {
            ensure!(Self::is_active(), Error::<T, I>::PalletInactive);

            let caller = ensure_signed(origin.clone())?;
            let members: Vec<T::AccountId> = Self::admin_members();
            ensure!(members.contains(&caller), Error::<T, I>::NotMember);

            ensure!(
                start >= frame_system::Pallet::<T>::block_number(),
                Error::<T, I>::EstimatesStartTooEarly
            );

            ensure!(
                start < end && end < distribute && start + LockedEstimates::<T, I>::get() < end,
                Error::<T, I>::EstimatesConfigInvalid
            );

            ensure!(
                (estimates_type == EstimatesType::DEVIATION
                    && deviation.is_some()
                    && range.is_none())
                    || (estimates_type == EstimatesType::RANGE
                        && deviation.is_none()
                        && range.is_some()),
                Error::<T, I>::EstimatesConfigInvalid
            );

            let mut range = range.clone();
            if range.is_some() {
                let _range = range.as_mut().unwrap();
                _range.sort();
                range = Some(_range.to_vec());
            }

            let fraction = T::PriceProvider::fraction(&symbol);
            ensure!(fraction.is_some(), Error::<T, I>::SymbolNotSupported);
            ensure!(
                !PreparedEstimates::<T, I>::contains_key(&symbol),
                Error::<T, I>::PreparedEstimatesExist
            );
            // let _0 = BalanceOf::<T, I>::from(0u32);
            let mut estimates_config = SymbolEstimatesConfig {
                symbol: symbol.clone(),
                estimates_type,
                id: 0,
                ticket_price: price,
                symbol_completed_price: 0,
                start,
                end,
                distribute,
                deviation,
                symbol_fraction: fraction.unwrap(),
                total_reward: BalanceOf::<T, I>::from(0u32),
                state: EstimatesState::InActive,
                range,
            };
            let id = SymbolEstimatesId::<T, I>::get(&symbol);
            let accounts: Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>> = vec![];
            if let Some(id) = id {
                estimates_config.id = id; // symbol exist
            }

            let current_id = estimates_config.id;
            SymbolEstimatesId::<T, I>::insert(symbol.clone(), estimates_config.id.clone() + 1); //generate next id
            PreparedEstimates::<T, I>::insert(symbol.clone(), &estimates_config);
            Self::hex_display_estimates_config(&estimates_config);
            if !CompletedEstimates::<T, I>::contains_key(&symbol) {
                let val: Vec<SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>> = vec![];
                CompletedEstimates::<T, I>::insert(&symbol, val);
            }
            Participants::<T, I>::insert(symbol, current_id, accounts);
            Self::deposit_event(Event::NewEstimates(estimates_config.encode(), caller));
            Ok(())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn participate_estimates(
            origin: OriginFor<T>,
            symbol: Vec<u8>,
            estimated_price: Option<u64>,
            range_index: Option<u8>,
            multiplier: MultiplierOption,
            bsc_address: Vec<u8>,
        ) -> DispatchResult {
            ensure!(Self::is_active(), Error::<T, I>::PalletInactive);
            let caller = ensure_signed(origin.clone())?;

            let fraction = T::PriceProvider::fraction(&symbol);
            ensure!(fraction.is_some(), Error::<T, I>::SymbolNotSupported);

            // let price = sp_std::str::from_utf8(estimated_price.as_slice());
            // ensure!(price.is_ok(), Error::<T, I>::PriceInvalid);
            // let price = price.unwrap().parse::<f32>();
            // ensure!(price.is_ok(), Error::<T, I>::PriceInvalid);
            // let fraction: f32 = 10u64.pow(fraction.unwrap()) as f32;
            // let original_price: u64 = (price.unwrap() * fraction) as u64;

            let mut bytes = [0u8; 40];
            let r = hex::encode_to_slice(&bsc_address, &mut bytes);
            ensure!(r.is_ok(), Error::<T, I>::AddressInvalid);
            ensure!(is_hex_address(&bytes), Error::<T, I>::AddressInvalid);

            log::info!("price {:?}", estimated_price);

            let config = ActiveEstimates::<T, I>::get(&symbol);
            if config.is_none() {
                return Err(Error::<T, I>::ActiveEstimatesNotExist.into());
            }
            let config = config.unwrap();
            let start = config.start;
            let end = config.end;
            let estimates_id = config.id;
            let mut ticket_price = config.ticket_price.clone();
            let estimates_type = &config.estimates_type;
            let current = frame_system::Pallet::<T>::block_number();

            //TODO 价格竞猜如果快要结束，暂停继续参与 lock
            ensure!(
                start <= current && current + LockedEstimates::<T, I>::get() < end,
                Error::<T, I>::EstimatesStateError
            );

            match multiplier {
                MultiplierOption::Base1 => {}
                MultiplierOption::Base2 => {
                    let _2 = BalanceOf::<T, I>::from(2u32);
                    ticket_price = ticket_price * _2;
                }
                MultiplierOption::Base5 => {
                    let _5 = BalanceOf::<T, I>::from(5u32);
                    ticket_price = ticket_price * _5;
                }
            }

            match estimates_type {
                EstimatesType::DEVIATION => {
                    ensure!(
                        estimated_price.is_some() && range_index.is_none(),
                        Error::<T, I>::ParameterInvalid
                    );
                }
                EstimatesType::RANGE => {
                    ensure!(
                        estimated_price.is_none() && range_index.is_some(),
                        Error::<T, I>::ParameterInvalid
                    );
                    ensure!(
                        usize::from(range_index.unwrap()) <= config.range.unwrap().len(),
                        Error::<T, I>::ParameterInvalid
                    )
                }
            }

            Participants::<T, I>::try_mutate(&symbol, estimates_id, |accounts| -> DispatchResult {
                if let Some(accounts) = accounts {
                    let found = accounts
                        .iter()
                        .find(|account| -> bool { caller == account.account });
                    if let Some(_) = found {
                        return Err(Error::<T, I>::AccountEstimatesExist.into());
                    }
                    ensure!(
                        T::Currency::free_balance(&caller) > ticket_price,
                        Error::<T, I>::FreeBalanceTooLow
                    );
                    let estimates = AccountParticipateEstimates {
                        account: caller.clone(),
                        end,
                        estimates: estimated_price,
                        range_index,
                        bsc_address: Some(bsc_address),
                        multiplier,
                        reward: 0,
                    };
                    T::Currency::transfer(
                        &caller,
                        &Self::account_id(),
                        ticket_price,
                        ExistenceRequirement::KeepAlive,
                    )?;

                    accounts.push(estimates);

                    //symbol reward pool
                    let reward: Option<BalanceOf<T, I>> = SymbolRewardPool::<T, I>::get(&symbol);
                    if let Some(reward) = reward {
                        let reward = reward + ticket_price;
                        SymbolRewardPool::<T, I>::insert(&symbol, reward);
                    } else {
                        SymbolRewardPool::<T, I>::insert(&symbol, ticket_price);
                    }

                    //user information
                    if AccountInfo::<T, I>::contains_key(&caller) {
                        AccountInfo::<T, I>::mutate(&caller, |estimates| {
                            if let Some(estimates) = estimates {
                                estimates.push((symbol.clone(), estimates_id));
                            }
                        })
                    } else {
                        AccountInfo::<T, I>::insert(&caller, vec![(symbol.clone(), estimates_id)]);
                    }
                } else {
                    return Err(Error::<T, I>::ActiveEstimatesNotExist.into());
                }
                Ok(())
            })
        }

        #[pallet::weight(0)]
        pub fn choose_winner_with_signed_payload(
            origin: OriginFor<T>,
            winner_payload: ChooseWinnersPayload<T::Public, T::AccountId, T::BlockNumber>,
            _signature: T::Signature,
        ) -> DispatchResult {
            ensure!(Self::is_active(), Error::<T, I>::PalletInactive);
            ensure_none(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            let winners = winner_payload.winners;
            let symbol = winner_payload.symbol;
            let id = winner_payload.estimates_id;
            let config = ActiveEstimates::<T, I>::get(&symbol);
            let mut total_reward = BalanceOf::<T, I>::from(0u32);
            let price = winner_payload.price.0;
            if config.is_some() {
                let mut config = config.unwrap();
                let end = config.end;
                let state = config.state.clone();
                if now > end && state == EstimatesState::Active {
                    Winners::<T, I>::insert(&symbol, id, winners.clone());
                    ActiveEstimates::<T, I>::remove(&symbol);
                    for winner in winners {
                        let reward: BalanceOf<T, I> = (winner.reward).saturated_into();
                        //TODO ？？是否需要在这里发放奖励 reserve reward
                        // account id: 4SEv6hkVfPYpAEfhkbX2H72ft968HPnmMCjUGdDoQn1QyPiK
                        T::Currency::transfer(
                            &Self::account_id(),
                            &winner.account,
                            reward,
                            ExistenceRequirement::AllowDeath,
                        )?;
                        total_reward = total_reward + reward;
                        let id = T::PalletId::get().0;
                        T::Currency::reserve_named(&id, &winner.account, reward)?;
                    }
                    let reward = SymbolRewardPool::<T, I>::get(&symbol);
                    if let Some(reward) = reward {
                        SymbolRewardPool::<T, I>::insert(&symbol, reward - total_reward);
                    }
                    CompletedEstimates::<T, I>::mutate(&symbol, |configs| {
                        let configs = configs.as_mut().unwrap();
                        config.state = EstimatesState::Completed;
                        config.symbol_completed_price = price;
                        config.total_reward = total_reward;
                        configs.push(config);
                    });
                }
            }
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn claim(
            origin: OriginFor<T>,
            dest: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: BalanceOf<T, I>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin.clone())?;
            let members: Vec<T::AccountId> = Self::admin_members();
            ensure!(members.contains(&caller), Error::<T, I>::NotMember);

            let dest = T::Lookup::lookup(dest)?;
            let id = T::PalletId::get().0;
            T::Currency::unreserve_named(&id, &dest, value);
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn preference(
            origin: OriginFor<T>,
            new_members: Option<Vec<T::AccountId>>,
            unsigned_members: Option<Vec<T::AccountId>>,
            locked_estimates: Option<T::BlockNumber>,
            minimum_ticket_price: Option<BalanceOf<T, I>>,
        ) -> DispatchResult {
            ensure_root(origin)?;
            if let Some(new_members) = new_members {
                AdminMembers::<T, I>::put(new_members);
            }
            if let Some(unsigned_members) = unsigned_members {
                UnsignedMembers::<T, I>::put(unsigned_members);
            }
            if let Some(locked_estimates) = locked_estimates {
                LockedEstimates::<T, I>::put(locked_estimates);
            }
            if let Some(minimum_ticket_price) = minimum_ticket_price {
                MinimumTicketPrice::<T, I>::put(minimum_ticket_price);
            }
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn active_pallet(origin: OriginFor<T>, active: bool) -> DispatchResult {
            ensure_root(origin)?;
            ActivePallet::<T, I>::put(active);
            Ok(())
        }
    }
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
    pub fn is_active() -> bool {
        let active = ActivePallet::<T, I>::try_get();
        return if active.is_err() {
            true
        } else {
            active.unwrap()
        };
    }

    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account()
    }

    pub fn can_submit_unsigned() -> bool {
        return if Self::find_unsigned_account().is_some() {
            true
        } else {
            false
        };
    }

    pub fn find_unsigned_account() -> Option<T::Public> {
        let members = UnsignedMembers::<T, I>::try_get();
        if members.is_err() {
            return None;
        }
        let members = members.unwrap();
        let keys = <T::AuthorityId as AppCrypto<T::Public, T::Signature>>::RuntimeAppPublic::all();
        for key in keys {
            let generic_public =
                <T::AuthorityId as AppCrypto<T::Public, T::Signature>>::GenericPublic::from(key);
            let public: T::Public = generic_public.into();
            let account: T::AccountId = public.clone().into_account();
            if members.contains(&account) {
                return Some(public);
            }
        }
        None
    }

    fn do_synchronized_offchain_worker(now: T::BlockNumber) {
        ActiveEstimates::<T, I>::iter_keys().for_each(|symbol| {
            let config = ActiveEstimates::<T, I>::get(&symbol);
            let config = config.unwrap();
            let end = config.end;
            let symbol = config.symbol.clone();
            let deviation = config.deviation;
            let range = config.range.clone();
            let estimates_type = config.estimates_type.clone();
            let id = config.id;
            if now > end {
                let accounts = Participants::<T, I>::get(&symbol, id);
                if let Some(accounts) = accounts {
                    if accounts.len() == 0 {
                        Self::send_unsigned(now, vec![], &config, (0, 0));
                    } else {
                        let price: Result<(u64, FractionLength), ()> =
                            T::PriceProvider::price(&symbol);
                        log::info!("accounts: {:?}, price: {:?}", accounts, price);
                        let _total_reward = SymbolRewardPool::<T, I>::get(&symbol);
                        let mut total_reward = BalanceOf::<T, I>::from(0u32);
                        if _total_reward.is_some() {
                            total_reward = _total_reward.unwrap()
                        }
                        if price.is_ok() {
                            let winners: Vec<
                                AccountParticipateEstimates<T::AccountId, T::BlockNumber>,
                            > = Self::choose_winner(
                                estimates_type,
                                &symbol,
                                total_reward,
                                deviation,
                                range,
                                accounts,
                                price.unwrap(),
                            );
                            log::info!("winners: {:?}", winners);
                            Self::send_unsigned(now, winners, &config, price.unwrap())
                        }
                    }
                }
            }
        });
    }

    // fn get_price(symbol: Vec<u8>) -> Result<u64, ()> {
    //     Ok(123)
    // }
    pub fn choose_winner(
        estimates_type: EstimatesType,
        symbol: &Vec<u8>,
        total_reward: BalanceOf<T, I>,
        deviation: Option<Permill>,
        range: Option<Vec<u64>>,
        accounts: Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>>,
        price: (u64, FractionLength),
    ) -> Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>> {
        let mut count: u32 = 0;

        let mut push_winner =
            |winners: &mut Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>>,
             winner: AccountParticipateEstimates<T::AccountId, T::BlockNumber>| {
                match winner.multiplier {
                    MultiplierOption::Base1 => count += 1,
                    MultiplierOption::Base2 => count += 2,
                    MultiplierOption::Base5 => count += 5,
                }
                winners.push(winner);
            };

        let calculate_reward = |winners: &mut Vec<
            AccountParticipateEstimates<T::AccountId, T::BlockNumber>,
        >,
                                count: u32| {
            let mut avg_reward = BalanceOf::<T, I>::from(0u32);
            if count > 0 {
                avg_reward = total_reward / BalanceOf::<T, I>::from(count);
            }
            winners.into_iter().try_for_each(|winner| {
                match winner.multiplier {
                    MultiplierOption::Base1 => {
                        winner.reward = TryInto::<u128>::try_into(avg_reward).ok().unwrap();
                    }
                    MultiplierOption::Base2 => {
                        let _2 = BalanceOf::<T, I>::from(2u32);
                        winner.reward = TryInto::<u128>::try_into(avg_reward * _2).ok().unwrap();
                    }
                    MultiplierOption::Base5 => {
                        let _5 = BalanceOf::<T, I>::from(5u32);
                        winner.reward = TryInto::<u128>::try_into(avg_reward * _5).ok().unwrap();
                    }
                };
                Some(())
            });
        };

        let mut winners: Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>> = vec![];
        match estimates_type {
            EstimatesType::DEVIATION => {
                // let _base: u64 = 10;
                // let fraction = base.pow(price.1);
                let price: u64 = price.0;
                let div_price: u64 = deviation.unwrap().mul_ceil(price);
                let low_price: u64 = price - div_price;
                let high_price: u64 = price + div_price;
                log::info!("price_range: {:?} --- {:?}", low_price, high_price);
                for x in accounts {
                    let estimates = x.estimates.unwrap();
                    if low_price <= estimates && estimates <= high_price {
                        push_winner(&mut winners, x);
                    }
                }
            }
            EstimatesType::RANGE => {
                let range = range.unwrap();
                for x in accounts {
                    let range_index = usize::from(x.range_index.unwrap());
                    let price: u64 = price.0;
                    // let rangeIndex = 2;
                    if range_index == 0 && price <= range[0] {
                        log::info!("price:{:?} <= range:{:?}", price, range[0]);
                        /// price <= range[0]
                        push_winner(&mut winners, x);
                    } else if range_index == range.len() && price > range[range.len() - 1] {
                        log::info!("price:{:?} > range:{:?}", price, range[range.len() - 1]);
                        /// price > range[range.length-1]
                        push_winner(&mut winners, x);
                    } else if 0 < range_index
                        && range_index < range.len()
                        && range[range_index] < price
                        && price <= range[range_index + 1]
                    {
                        log::info!(
                            " range:{:?} < price:{:?} <= range:{:?}",
                            range[range_index - 1],
                            price,
                            range[range_index]
                        );
                        /// range[index-1] < price <= range[index]
                        push_winner(&mut winners, x);
                    }
                }
            }
        };

        //calculate reward
        calculate_reward(&mut winners, count);
        winners
    }

    pub fn send_unsigned(
        block_number: T::BlockNumber,
        winners: Vec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>>,
        estimates_config: &SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>,
        price: (u64, FractionLength),
    ) {
        let unsigned_account = Self::find_unsigned_account();
        if let Some(unsigned_account) = unsigned_account {
            Signer::<T, T::AuthorityId>::any_account()
                .with_filter(vec![unsigned_account])
                .send_unsigned_transaction(
                    |account| ChooseWinnersPayload {
                        block_number,
                        winners: winners.clone(),
                        public: account.public.clone(),
                        symbol: estimates_config.symbol.clone(),
                        estimates_id: estimates_config.id,
                        price,
                    },
                    |payload, signature| {
                        Call::choose_winner_with_signed_payload(payload, signature)
                    },
                )
                .ok_or("❗ No local accounts accounts available, `aura` StoreKey needs to be set.");
        } else {
            log::warn!("can not found any account")
        }

        // result.map_err(|()| "⛔ Unable to submit transaction");
    }

    pub fn hex_display_estimates_config(
        estimates_config: &SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>,
    ) {
        // let hash = Blake2_128Concat::hash(estimates_config.encode().as_slice());
        let encode = estimates_config.encode();
        log::info!(
            "estimates_encode: {:?}",
            HexDisplay::from(&encode) // encode
        );
    }
}
