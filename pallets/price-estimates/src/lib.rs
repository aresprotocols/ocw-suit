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

use types::{
    is_hex_address, AccountParticipateEstimates, ChooseWinnersPayload, EstimatesState,
    SymbolEstimatesConfig,
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
    traits::AccountIdConversion,
    transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
    AccountId32, Perbill, Permill, RuntimeAppPublic, RuntimeDebug,
};

use pallet_ocw::{types::FractionLength, AvgPrice};

type BalanceOf<T, I = ()> =
    <<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T, I = ()>(_);

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

        type Currency: Currency<Self::AccountId>;

        type Call: From<Call<Self, I>>;

        type PriceProvider: AvgPrice;

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

    // Map<Symbol, Vec<SymbolEstimatesConfig>>
    #[pallet::storage]
    pub type AllEstimates<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Identity,
        Vec<u8>, // symbol
        Identity,
        u64,                                                    // id
        SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>, // configs
    >;

    // Map<SymbolEstimatesConfig, Vec<AccountSymbolEstimates>>
    #[pallet::storage]
    pub type Participants<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Identity,
        Vec<u8>, // symbol
        Identity,
        u64, // id
        Vec<AccountParticipateEstimates<T::AccountId>>,
    >;

    #[pallet::storage]
    pub type Winners<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
        _,
        Identity,
        Vec<u8>, // symbol
        Identity,
        u64, // id
        Vec<AccountParticipateEstimates<T::AccountId>>,
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
        EthInvalid,

        FreeBalanceTooLow,

        EstimatesStartTooEarly,

        EstimatesStateError,

        EstimatesConfigNotExist,

        AccountEstimatesExist,

        AlreadyEnded,

        AlreadyParticipating,

        TooManyEstimates,
    }

    #[pallet::hooks]
    impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
        fn on_initialize(n: T::BlockNumber) -> Weight {
            AllEstimates::<T, I>::iter_keys().for_each(|(symbol, id)| {
                // let temp_key = key.clone();
                let _symbol = sp_std::str::from_utf8(&symbol).unwrap();
                AllEstimates::<T, I>::try_mutate(&symbol, id, |config| -> DispatchResult {
                    if let Some(config) = config.as_mut() {
                        let start = config.start;
                        let end = config.start + config.length;
                        if n % T::BlockNumber::from(5u32) == T::BlockNumber::from(0u32) {
                            log::info!("symbol: {:?} config: {:?}", _symbol, config);
                        }
                        if start <= n && config.state == EstimatesState::InActive {
                            // Self::hex_display_estimates_config(&config);
                            config.state = EstimatesState::Active;
                            // Self::hex_display_estimates_config(&config);
                        }
                    }
                    Ok(())
                });
            });
            0
        }

        fn offchain_worker(now: T::BlockNumber) {
            use sp_runtime::offchain::storage_lock::{BlockAndTime, StorageLock};

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
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            if let Call::choose_winner_with_signed_payload(ref payload, ref signature) = call {
                log::info!(
                    "🚅 Validate price payload data, on block: {:?} ",
                    frame_system::Pallet::<T>::block_number()
                );
                ValidTransaction::with_tag_prefix(
                    "pallet-price-estimates::validate_transaction_parameters",
                )
                .priority(T::UnsignedPriority::get())
                .and_provides(payload.public.clone())
                // .and_provides(&payload.block_number)
                .longevity(5)
                .propagate(true)
                .build()
            } else {
                log::info!("test123");
                InvalidTransaction::Call.into()
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
            length: T::BlockNumber,
            delay: T::BlockNumber,
            deviation: Permill,
            #[pallet::compact] price: BalanceOf<T, I>,
        ) -> DispatchResult {
            ensure!(
                start >= frame_system::Pallet::<T>::block_number(),
                Error::<T, I>::EstimatesStartTooEarly
            );

            let caller = ensure_signed(origin.clone())?;
            let zero = <BalanceOf<T, I>>::from(0u32);
            let mut estimates_config = SymbolEstimatesConfig {
                symbol: symbol.clone(),
                id: 0,
                price,
                start,
                length,
                delay,
                deviation,
                total_reward: zero,
                state: EstimatesState::InActive,
            };
            let id = SymbolEstimatesId::<T, I>::get(symbol.clone());
            let accounts: Vec<AccountParticipateEstimates<T::AccountId>> = vec![];
            if let Some(id) = id {
                estimates_config.id = id; // symbol exist
            }

            let current_id = estimates_config.id;
            SymbolEstimatesId::<T, I>::insert(symbol.clone(), estimates_config.id.clone() + 1); //generate next id
            AllEstimates::<T, I>::insert(symbol.clone(), current_id, &estimates_config);
            Self::hex_display_estimates_config(&estimates_config);
            Participants::<T, I>::insert(symbol, current_id, accounts);

            Self::deposit_event(Event::NewEstimates(estimates_config.encode(), caller));
            Ok(())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn participate_estimates(
            origin: OriginFor<T>,
            symbol: Vec<u8>,
            estimates_id: u64,
            estimated_price: u64,
            eth_address: Vec<u8>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin.clone())?;

            let mut bytes = [0u8; 40];
            let r = hex::encode_to_slice(&eth_address, &mut bytes);
            ensure!(r.is_ok(), Error::<T, I>::EthInvalid);
            ensure!(is_hex_address(&bytes), Error::<T, I>::EthInvalid);

            let zero = <BalanceOf<T, I>>::from(0u32);
            /*let config = */
            AllEstimates::<T, I>::try_mutate(&symbol, estimates_id, |config| -> DispatchResult {
                if config.is_none() {
                    return Err(Error::<T, I>::EstimatesConfigNotExist.into());
                }
                let config = config.as_mut().unwrap();
                let start = config.start;
                let end = config.start + config.length;
                let price = config.price;
                Participants::<T, I>::try_mutate(
                    &symbol,
                    estimates_id,
                    |accounts| -> DispatchResult {
                        if let Some(accounts) = accounts {
                            let current = frame_system::Pallet::<T>::block_number();
                            ensure!(
                                start <= current && current < end,
                                Error::<T, I>::EstimatesStateError
                            );

                            let found = accounts
                                .iter()
                                .find(|account| -> bool { caller == account.account });
                            if let Some(_) = found {
                                log::info!("not empty, should be error ---> account");
                                return Err(Error::<T, I>::AccountEstimatesExist.into());
                            } else {
                                ensure!(
                                    T::Currency::free_balance(&caller) > price,
                                    Error::<T, I>::FreeBalanceTooLow
                                );
                                let estimates = AccountParticipateEstimates {
                                    account: caller.clone(),
                                    estimates: estimated_price,
                                    eth_address: Some(eth_address),
                                };
                                T::Currency::transfer(
                                    &caller,
                                    &Self::account_id(),
                                    price,
                                    ExistenceRequirement::KeepAlive,
                                )?;
                                accounts.push(estimates);
                                config.total_reward += price;
                            }
                        } else {
                            log::info!("is empty, should be error");
                            // ensure!(false, Error::<T, I>::EstimatesConfigNotExist);
                            return Err(Error::<T, I>::EstimatesConfigNotExist.into());
                        }
                        Ok(())
                    },
                )
            })
        }

        #[pallet::weight(0)]
        pub fn choose_winner_with_signed_payload(
            origin: OriginFor<T>,
            winner_payload: ChooseWinnersPayload<T::Public, T::AccountId, T::BlockNumber>,
            _signature: T::Signature,
        ) -> DispatchResult {
            ensure_none(origin)?;
            let now = frame_system::Pallet::<T>::block_number();
            let winners = winner_payload.winners;
            let symbol = winner_payload.symbol;
            let id = winner_payload.estimates_id;
            AllEstimates::<T, I>::try_mutate(&symbol, id, |config| -> DispatchResult {
                if let Some(config) = config.as_mut() {
                    if now > config.start + config.length && config.state == EstimatesState::Active
                    {
                        log::info!("config found..... prepare update");
                        config.state = EstimatesState::WaitingPayout;
                        Winners::<T, I>::insert(&symbol, id, winners.clone());
                    }
                }
                Ok(())
            })
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn claim(origin: OriginFor<T>) -> DispatchResult {
            Ok(())
        }
    }
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account()
    }

    // pub fn insert_new_estimates(
    //     caller: T::AccountId,
    //     config: &mut Option<SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>>,
    //     item: SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>,
    // ) -> bool {
    //     if !Participants::<T, I>::contains_key(item.symbol.clone(), item.id) {
    //         <Participants<T, I>>::insert(item.symbol, item.id, a);
    //     } else {
    //         log::info!("should be error...");
    //         return false;
    //     }
    //
    //     //TODO check
    //     if let Some(configs) = configs.as_mut() {
    //         configs.push(item.clone());
    //
    //         return true;
    //     }
    //     return false;
    // }

    fn do_synchronized_offchain_worker(now: T::BlockNumber) {
        AllEstimates::<T, I>::iter_keys().for_each(|(symbol, id)| {
            let config = AllEstimates::<T, I>::get(symbol, id);
            let config = config.unwrap();
            let end = config.start + config.length;
            let symbol = config.symbol.clone();
            let deviation = config.deviation.clone();
            if now > end && config.state == EstimatesState::Active {
                let accounts = Participants::<T, I>::get(symbol.clone(), id);
                if let Some(accounts) = accounts {
                    if accounts.len() == 0 {
                        Self::send_unsigned(now, vec![], config.clone(), (0, 0));
                    } else {
                        let price: Result<(u64, FractionLength), ()> =
                            T::PriceProvider::price(symbol);
                        log::info!("accounts: {:?}, price: {:?}", accounts, price);
                        if price.is_ok() {
                            let winners: Vec<AccountParticipateEstimates<T::AccountId>> =
                                Self::choose_winner(deviation, accounts, price.unwrap());
                            log::info!("winners: {:?}", winners);
                            Self::send_unsigned(now, winners, config.clone(), price.unwrap())
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
        deviation: Permill,
        accounts: Vec<AccountParticipateEstimates<T::AccountId>>,
        price: (u64, FractionLength),
    ) -> Vec<AccountParticipateEstimates<T::AccountId>> {
        let mut result: Vec<AccountParticipateEstimates<T::AccountId>> = vec![];
        let base: u64 = 10;
        let price: u64 = price.0 / base.pow(price.1);
        let div_price: u64 = deviation.mul_ceil(price);
        let low_price: u64 = price - div_price;
        let high_price: u64 = price + div_price;
        log::info!("price_range: {:?} --- {:?}", low_price, high_price);
        for x in accounts {
            if low_price <= x.estimates && x.estimates <= high_price {
                result.push(x);
            }
        }
        result
    }

    pub fn send_unsigned(
        block_number: T::BlockNumber,
        winners: Vec<AccountParticipateEstimates<T::AccountId>>,
        estimates_config: SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T, I>>,
        price: (u64, FractionLength),
    ) {
        Signer::<T, T::AuthorityId>::any_account()
            .send_unsigned_transaction(
                |account| ChooseWinnersPayload {
                    block_number,
                    winners: winners.clone(),
                    public: account.public.clone(),
                    symbol: estimates_config.symbol.clone(),
                    estimates_id: estimates_config.id,
                    price,
                },
                |payload, signature| Call::choose_winner_with_signed_payload(payload, signature),
            )
            .ok_or("❗ No local accounts accounts available, `aura` StoreKey needs to be set.");
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
