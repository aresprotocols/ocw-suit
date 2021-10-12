#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;

use codec::{Decode, Encode};

use sp_std::convert::{TryFrom, TryInto};

use sp_consensus_aura::{
    ed25519::AuthorityId as AuraEd25519Id, sr25519::AuthorityId as AuraSr25519Id, AURA_ENGINE_ID,
};
use sp_consensus_babe::{AuthorityId as BabeId, BABE_ENGINE_ID};
use sp_core::{
    crypto::UncheckedFrom, ed25519::Public as Ed25519Public, sr25519::Public as Sr25519Public,
};

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{
        BalanceStatus, ChangeMembers, Currency, ExistenceRequirement, FindAuthor, Imbalance,
        LockIdentifier, LockableCurrency, NamedReservableCurrency, OnUnbalanced,
        ReservableCurrency, WithdrawReasons,
    },
    weights::{DispatchClass, Weight},
    PalletId,
};
use pallet_aura;
use sp_consensus_slots::Slot;

use sp_runtime::{
    traits::{AccountIdConversion, IsMember, SaturatedConversion, StaticLookup},
    RuntimeAppPublic,
};

use frame_system::pallet_prelude::*;
use log::info;

// #[cfg(test)]
// mod mock;
//
// #[cfg(test)]
// mod tests;
//
// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

type BalanceOf<T, I = ()> =
    <<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type NegativeImbalanceOf<T, I = ()> = <<T as Config<I>>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

type ReserveIdentifierOf<T, I = ()> = <<T as Config<I>>::Currency as NamedReservableCurrency<
    <T as frame_system::Config>::AccountId,
>>::ReserveIdentifier;

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct Detail<AccountId, Balance> {
    provider: AccountId,
    dest: AccountId,
    deposit: Balance,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::traits::IsSubType;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T, I = ()>(_);

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config<I: 'static = ()>: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

        type Currency: Currency<Self::AccountId>
            + NamedReservableCurrency<Self::AccountId, ReserveIdentifier = [u8; 8]>;

        type CouncilMajorityOrigin: EnsureOrigin<Self::Origin>;

        /// The minimum amount to be checked price
        #[pallet::constant]
        type MinimumDeposit: Get<BalanceOf<Self, I>>;

        #[pallet::constant]
        type BidderMinimumDeposit: Get<BalanceOf<Self, I>>;

        /// The demo's module id
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        // Handler for the unbalanced reduction when the
        type SlashProposer: OnUnbalanced<NegativeImbalanceOf<Self, I>>;

        // type SlashBidder: OnUnbalanced<NegativeImbalanceOf<Self, I>>;

        type IsAuthority: IsMember<Self::AuthorityId>;

        //Copy Aura::AuthorityId
        type AuthorityId: Member
            + Parameter
            + RuntimeAppPublic
            + Default
            + MaybeSerializeDeserialize
            + UncheckedFrom<[u8; 32]>;

        // type FindAuthor: FindAuthor<Self::AuthorityId>;
    }

    // The pallet's runtime storage items.
    // https://substrate.dev/docs/en/knowledgebase/runtime/storage
    #[pallet::storage]
    #[pallet::getter(fn something)]
    // Learn more about declaring storage items:
    // https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
    pub type Something<T: Config<I>, I: 'static = ()> = StorageValue<_, u32>;

    #[pallet::storage]
    pub type Proposals<T: Config<I>, I: 'static = ()> =
        StorageMap<_, Identity, T::Hash, Detail<T::AccountId, BalanceOf<T, I>>>;

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config<I>, I: 'static = ()> {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        SomethingStored(u32, T::AccountId),

        /// parameters. [who, value]
        CheckedNoPassSlashed(T::AccountId, BalanceOf<T, I>),

        /// Some funds have been deposited. \[deposit\]
        Deposit(T::AccountId, BalanceOf<T, I>),

        Reserved([u8; 8], T::AccountId, BalanceOf<T, I>),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T, I = ()> {
        /// Error names should be descriptive.
        NoneValue,
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,
        /// Deposit too low
        DepositLow,
        /// Free balance too low
        FreeBalanceLow,
        /// Proposal already noted
        DuplicateProposal,
        /// Proposal is missing
        MissingProposal,

        BadOrigin,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config<I>, I: 'static> Pallet<T, I> {
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn check(
            origin: OriginFor<T>,
            dest: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] price: BalanceOf<T, I>,
            #[pallet::compact] deposit: BalanceOf<T, I>,
        ) -> DispatchResult {
            info!("only check do nothing");
            let who = T::CouncilMajorityOrigin::ensure_origin(origin)?;
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn demo(origin: OriginFor<T>) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            info!(
                "is_aura: {:?}, is_authority: {:?}",
                Self::is_aura(),
                Self::is_authority(&who)
            );
            Ok(())
        }

        #[pallet::weight(10_000)]
        pub fn deposit_by_bidder(
            origin: OriginFor<T>,
            #[pallet::compact] deposit: BalanceOf<T, I>,
        ) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            ensure!(
                deposit >= T::BidderMinimumDeposit::get(),
                Error::<T, I>::DepositLow
            );
            ensure!(
                deposit <= T::Currency::free_balance(&who),
                Error::<T, I>::FreeBalanceLow
            );
            ensure!(Self::is_authority(&who), Error::<T, I>::BadOrigin);
            let id = T::PalletId::get().0;
            T::Currency::reserve_named(&id, &who, deposit)
        }

        #[pallet::weight(10_000)]
        pub fn enough(origin: OriginFor<T>) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            let id = T::PalletId::get().0;
            let balance = T::Currency::reserved_balance_named(&id, &who);
            info!("reserved balance: {:?}", balance);
            Ok(())
        }
    }

    // #[pallet::hooks]
    // impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
    // 	/// Weight: see `begin_block`
    // 	fn on_initialize(n: T::BlockNumber) -> Weight {
    // 		let digest = frame_system::Pallet::<T>::digest();
    // 		info!("digest: {:?}", digest);
    // 		0
    // 	}
    // }
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
    /// The account ID of the treasury pot.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account()
    }

    /// The account ID of the payouts pot. This is where payouts are made from.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn payouts() -> T::AccountId {
        T::PalletId::get().into_sub_account(b"payouts")
    }

    pub fn is_aura() -> bool {
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

    pub fn is_babe() -> bool {
        let digest = frame_system::Pallet::<T>::digest();
        let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
        // let author = T::FindAuthor::find_author(pre_runtime_digests.clone());
        let mut is_babe = false;
        for (id, _) in pre_runtime_digests {
            if id == BABE_ENGINE_ID {
                is_babe = true;
            }
        }
        is_babe
    }

    pub fn is_authority(who: &T::AccountId) -> bool {
        let encode_data = who.encode();
        if encode_data.len() != 32 {
            return false;
        }
        let raw: Result<[u8; 32], _> = encode_data.try_into();
        if raw.is_err() {
            return false;
        }
        let raw = raw.unwrap();
        let _author = T::AuthorityId::unchecked_from(raw);
        let is_author = T::IsAuthority::is_member(&_author);
        is_author
    }
}

// Copy treasury:OnUnbalanced
impl<T: Config<I>, I: 'static> OnUnbalanced<NegativeImbalanceOf<T, I>> for Pallet<T, I> {
    fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T, I>) {
        let numeric_amount = amount.peek(); //import Imbalance

        // Must resolve into existing but better to be safe.
        let _ = T::Currency::resolve_creating(&Self::account_id(), amount);
        Self::deposit_event(Event::Deposit(Self::account_id(), numeric_amount));
    }
}

pub trait CheckFlow<AccountId, Hash, Balance> {
    type Balance;
    // fn prepare(who: &AccountId, proposal_hash: Hash, deposit: Self::Balance) -> DispatchResult;
    fn prepare(
        who: &AccountId,
        dest: &AccountId,
        proposal_hash: Hash,
        deposit: Balance,
    ) -> DispatchResult;

    fn slash(proposal_hash: Hash) -> DispatchResult;

    fn pass(proposal_hash: Hash) -> DispatchResult;
}

impl<T: Config<I>, I: 'static>
    CheckFlow<
        <T as frame_system::Config>::AccountId,
        <T as frame_system::Config>::Hash,
        BalanceOf<T, I>,
    > for Pallet<T, I>
{
    type Balance = BalanceOf<T, I>;
    fn prepare(
        who: &<T as frame_system::Config>::AccountId,
        dest: &<T as frame_system::Config>::AccountId,
        proposal_hash: <T as frame_system::Config>::Hash,
        deposit: BalanceOf<T, I>,
    ) -> DispatchResult {
        ensure!(
            !<Proposals<T, I>>::contains_key(&proposal_hash),
            Error::<T, I>::DuplicateProposal
        );
        ensure!(
            deposit >= T::MinimumDeposit::get(),
            Error::<T, I>::DepositLow
        );
        ensure!(
            deposit <= T::Currency::free_balance(&who),
            Error::<T, I>::FreeBalanceLow
        );

        let id = T::PalletId::get().0;
        // let dest = T::Lookup::lookup(dest)?;
        T::Currency::reserve_named(&id, &who, deposit)?;
        let detail = Detail {
            provider: who.clone(),
            dest: dest.clone(),
            deposit,
        };
        Self::deposit_event(Event::Reserved(id, who.clone(), deposit));
        <Proposals<T, I>>::insert(&proposal_hash, detail);
        Ok(())
    }

    fn slash(proposal_hash: <T as frame_system::Config>::Hash) -> DispatchResult {
        // ensure!(<Proposals<T,I>>::contains_key(&proposal_hash), Error::<T,I>::MissingProposal);
        let proposal =
            <Proposals<T, I>>::get(&proposal_hash).ok_or(Error::<T, I>::MissingProposal)?;
        let who = proposal.provider;
        let deposit = proposal.deposit;

        let id = T::PalletId::get().0;
        let (imbalance, _remainder) = T::Currency::slash_reserved_named(&id, &who, deposit);
        T::SlashProposer::on_unbalanced(imbalance);
        Self::deposit_event(Event::CheckedNoPassSlashed(who.clone(), deposit));
        <Proposals<T, I>>::remove(&proposal_hash);
        Ok(())
    }

    fn pass(proposal_hash: <T as frame_system::Config>::Hash) -> DispatchResult {
        info!("abc");
        let proposal =
            <Proposals<T, I>>::get(&proposal_hash).ok_or(Error::<T, I>::MissingProposal)?;

        let who = proposal.provider;
        let dest = proposal.dest;
        let deposit = proposal.deposit;
        let id = T::PalletId::get().0;
        info!("start");
        T::Currency::unreserve_named(&id, &who, deposit);

        //TODO Slash bidder
        let mut slash_value: u128 = TryInto::<u128>::try_into(deposit).ok().unwrap();
        slash_value = slash_value * 7;
        let _slash_value: Self::Balance = slash_value.saturated_into();
        let (imbalance, _remainder) = T::Currency::slash_reserved_named(&id, &dest, _slash_value);
        // T::SlashProposer::on_unbalanced(imbalance);
        <Proposals<T, I>>::remove(&proposal_hash);
        let value = TryInto::<u128>::try_into(imbalance.peek()).ok();
        if value.is_some() {
            let _value = value.unwrap();
            let _value = _value / 7;
            let treasury_reward: Self::Balance = (_value * 2).saturated_into();
            let proposer_reward: Self::Balance = (_value * 5).saturated_into();
            let _who = who.clone();
            let _reward_acc = Self::payouts();
            info!("who: {:?}, proposer_reward: {:?}, reward: {:?}, treasury_reward: {:?}", _who, proposer_reward, _reward_acc ,treasury_reward);
            let _ = T::Currency::deposit_creating(&who, proposer_reward);
            let _ = T::Currency::deposit_creating(&Self::payouts(), treasury_reward);

            Self::deposit_event(Event::Deposit(who, proposer_reward));
            Self::deposit_event(Event::Deposit(Self::payouts(), treasury_reward));
        }
        Ok(())
    }
}

pub trait CheckDeposit<AccountId> {
    fn enough(who: &AccountId) -> bool;
}

impl<T: Config<I>, I: 'static> CheckDeposit<<T as frame_system::Config>::AccountId>
    for Pallet<T, I>
{
    fn enough(who: &<T as frame_system::Config>::AccountId) -> bool {
        let id = T::PalletId::get().0;
        let balance = T::Currency::reserved_balance_named(&id, &who);
        if balance < T::BidderMinimumDeposit::get() {
            return false;
        }
        true
    }
}
