#![cfg_attr(not(feature = "std"), no_std)]
#![feature(type_name_of_val)]

mod tests;

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;

use codec::{Decode, Encode, MaxEncodedLen};

use sp_std::convert::TryInto;

use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::*,
	traits::{Currency, Imbalance, NamedReservableCurrency, OnUnbalanced},
	weights::Weight,
	PalletId,
};
use sp_consensus_aura::AURA_ENGINE_ID;
use sp_consensus_babe::BABE_ENGINE_ID;
use sp_std::boxed::Box;

use frame_system::pallet_prelude::*;
use log::info;
use sp_core::crypto::UncheckedFrom;
use sp_runtime::{
	traits::{AccountIdConversion, Hash, IsMember, SaturatedConversion, StaticLookup},
	RuntimeAppPublic,
};
// #[cfg(test)]
// mod mock;
//
// #[cfg(test)]
// mod tests;
//
// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

type BalanceOf<T, I = ()> = <<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type NegativeImbalanceOf<T, I = ()> =
	<<T as Config<I>>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

// type ReserveIdentifierOf<T, I = ()> =
// 	<<T as Config<I>>::Currency as NamedReservableCurrency<<T as
// frame_system::Config>::AccountId>>::ReserveIdentifier;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
	use frame_support::traits::IsSubType;
	use sp_runtime::traits::Dispatchable;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	// #[pallet::generate_storage_info]
	pub struct Pallet<T, I = ()>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config + pallet_collective::Config<I> {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self, I>> + IsType<<Self as frame_system::Config>::Event>;

		type Proposal: Parameter
			+ sp_runtime::traits::Dispatchable<
				Origin = <Self as frame_system::Config>::Origin,
				PostInfo = PostDispatchInfo,
			> + From<pallet_collective::Call<Self, I>>
			+ From<crate::Call<Self, I>>
			+ From<frame_system::Call<Self>>
			+ GetDispatchInfo
			+ IsSubType<pallet_collective::Call<Self, I>>;

		type Currency: Currency<Self::AccountId> + NamedReservableCurrency<Self::AccountId, ReserveIdentifier = [u8; 8]>;

		type CouncilMajorityOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		/// The minimum amount to be checked price
		#[pallet::constant]
		type MinimumDeposit: Get<BalanceOf<Self, I>>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		// Handler for the unbalanced reduction when the
		type SlashProposer: OnUnbalanced<NegativeImbalanceOf<Self, I>>;

		type IsAuthority: IsMember<Self::AuthorityId>;

		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			// + Default
			+ MaybeSerializeDeserialize
			+ UncheckedFrom<[u8; 32]>;

		// type FindAuthor: FindAuthor<Self::AuthorityId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn proposals)]
	pub type Proposals<T: Config<I>, I: 'static = ()> =
		StorageMap<_, Identity, T::Hash, ChallengeInfo<T::AccountId, T::Hash, BalanceOf<T, I>, T::BlockNumber>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// parameters. [who, value]
		CheckedNoPassSlashed { who: T::AccountId, amount: BalanceOf<T, I> },

		/// Some funds have been deposited. \[deposit\]
		Deposit { who: T::AccountId, amount: BalanceOf<T, I> },

		Reserved {
			id: [u8; 8],
			who: T::AccountId,
			amount: BalanceOf<T, I>,
		},
	}

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

	#[pallet::hooks]
	impl<T: Config<I>, I: 'static> Hooks<BlockNumberFor<T>> for Pallet<T, I> {
		fn on_finalize(n: BlockNumberFor<T>) {
			let ii: u64 = n.saturated_into::<u64>();
			if ii > 0 && ii % 100 == 0 {
				<Proposals<T, I>>::iter()
					.filter(|(_, info)| n > info.end)
					.for_each(|(hash, info)| {
						let proposal_hash = info.proposal;
						if !<pallet_collective::ProposalOf<T, I>>::contains_key(&proposal_hash) {
							log::warn!("Proposal not found.. clear.. slash proposer");
							let r = Self::slash_proposer(hash);
							if r.is_err() {
								log::error!("slash proposer error: {:?}", r);
							}
						}
					});
			}
		}

		/// Initialization
		fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
			0
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn new_challenge(
			origin: OriginFor<T>,
			delegatee: <T::Lookup as StaticLookup>::Source,
			validator: <T::Lookup as StaticLookup>::Source,
			block_hash: T::Hash,
			#[pallet::compact] deposit: BalanceOf<T, I>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let delegatee = T::Lookup::lookup(delegatee)?;
			let validator = T::Lookup::lookup(validator)?;
			ensure!(deposit >= T::MinimumDeposit::get(), Error::<T, I>::DepositLow);
			ensure!(
				deposit <= T::Currency::free_balance(&who),
				Error::<T, I>::FreeBalanceLow
			);
			let members = pallet_collective::Members::<T, I>::get();
			ensure!(
				members.contains(&delegatee),
				pallet_collective::Error::<T, I>::NotMember
			);
			let challenge_validator = ChallengeValidator {
				validator: validator.clone(),
				block_hash: block_hash.clone(),
			};
			let challenge_validator_hash = T::Hashing::hash_of(&challenge_validator);
			ensure!(
				!Proposals::<T, I>::contains_key(&challenge_validator_hash),
				Error::<T, I>::DuplicateProposal
			);
			let proposal = Self::create_proposal(members.len() as u32, challenge_validator_hash.clone());

			if let Ok((proposal, proposal_hash)) = proposal {
				let real: <T as frame_system::Config>::Origin = frame_system::RawOrigin::Signed(delegatee).into();
				// let proposal_hash = T::Hashing::hash_of(&proposal);
				proposal.dispatch(real).map_err(|e| e.error)?;
				let id = T::PalletId::get().0;

				let end = frame_system::Pallet::<T>::block_number() + T::MotionDuration::get();
				let info = ChallengeInfo {
					who: who.clone(),
					target: challenge_validator,
					deposit,
					proposal: proposal_hash,
					end,
				};
				<Proposals<T, I>>::insert(challenge_validator_hash, info);
				T::Currency::reserve_named(&id, &who, deposit)?;
				Self::deposit_event(Event::Reserved {
					id,
					who: who.clone(),
					amount: deposit,
				});
			} else {
				// TODO create proposal error
			}
			Ok(())
		}

		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn challenge_success(
			origin: OriginFor<T>,
			challenge_hash: <T as frame_system::Config>::Hash,
		) -> DispatchResult {
			let _ = T::CouncilMajorityOrigin::ensure_origin(origin)?;
			let info = <Proposals<T, I>>::get(&challenge_hash).ok_or(Error::<T, I>::MissingProposal)?;

			let who = info.who;
			let validator = info.target.validator;
			let deposit = info.deposit;
			let id = T::PalletId::get().0;
			T::Currency::unreserve_named(&id, &who, deposit);

			// Slash validator
			let mut slash_value: u128 = TryInto::<u128>::try_into(deposit).ok().unwrap();
			slash_value = slash_value * 7;
			let _slash_value: BalanceOf<T, I> = slash_value.saturated_into();
			let (imbalance, _remainder) = T::Currency::slash_reserved_named(&id, &validator, _slash_value);

			<Proposals<T, I>>::remove(&challenge_hash);
			let value = TryInto::<u128>::try_into(imbalance.peek()).ok();
			if let Some(_value) = value {
				let _value = _value / 7;
				let treasury_reward: BalanceOf<T, I> = (_value * 2).saturated_into();
				let proposer_reward: BalanceOf<T, I> = (_value * 5).saturated_into();
				let reward_acc = Self::account_id();
				info!(
					"who: {:?}, proposer_reward: {:?}, reward: {:?}, treasury_reward: {:?}",
					&who, &proposer_reward, &reward_acc, &treasury_reward
				);
				T::Currency::deposit_creating(&who, proposer_reward);
				T::Currency::deposit_creating(&reward_acc, treasury_reward);

				Self::deposit_event(Event::Deposit {
					who,
					amount: proposer_reward,
				});
				Self::deposit_event(Event::Deposit {
					who: reward_acc,
					amount: treasury_reward,
				});
				log::debug!("challenge success");
			}
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn reserve(origin: OriginFor<T>, #[pallet::compact] deposit: BalanceOf<T, I>) -> DispatchResult {
			let who: T::AccountId = ensure_signed(origin)?;
			// ensure!(deposit >= T::BidderMinimumDeposit::get(), Error::<T, I>::DepositLow);
			ensure!(
				deposit <= T::Currency::free_balance(&who),
				Error::<T, I>::FreeBalanceLow
			);
			ensure!(Self::is_authority(&who), Error::<T, I>::BadOrigin);
			let id = T::PalletId::get().0;
			T::Currency::reserve_named(&id, &who, deposit)
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I>
where
	T: frame_system::Config,
{
	fn create_proposal(
		members_count: u32,
		challenge_validator_hash: T::Hash,
	) -> Result<(<T as crate::Config<I>>::Proposal, T::Hash), ()> {
		let proposal: <T as crate::Config<I>>::Proposal = crate::Call::challenge_success {
			// challenge_hash: T::Hashing::hash_of(&challenge_validator),
			challenge_hash: challenge_validator_hash,
		}
		.into();
		let data = proposal.encode();
		let proposal_len: u32 = proposal.using_encoded(|p| p.len() as u32);
		let proposal_hash: T::Hash = <T as frame_system::Config>::Hashing::hash_of(&proposal);
		if let Ok(proposal) = <T as pallet_collective::Config<I>>::Proposal::decode(&mut &data[..]) {
			let call: pallet_collective::Call<T, I> = pallet_collective::Call::propose {
				threshold: members_count,
				proposal: Box::new(proposal),
				length_bound: proposal_len,
			};
			return Ok((call.into(), proposal_hash));
		}
		Err(())
	}

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
		Self::deposit_event(Event::Deposit {
			who: Self::account_id(),
			amount: numeric_amount,
		});
	}
}

pub trait ChallengeFlow<AccountId, Hash, Balance> {
	type Balance;

	fn slash_proposer(proposal_hash: Hash) -> DispatchResult;
}

impl<T: Config<I>, I: 'static>
	ChallengeFlow<<T as frame_system::Config>::AccountId, <T as frame_system::Config>::Hash, BalanceOf<T, I>>
	for Pallet<T, I>
{
	type Balance = BalanceOf<T, I>;

	fn slash_proposer(proposal_hash: <T as frame_system::Config>::Hash) -> DispatchResult {
		let proposal = <Proposals<T, I>>::get(proposal_hash).ok_or(Error::<T, I>::MissingProposal)?;
		let who = proposal.who;
		let deposit = proposal.deposit;

		let id = T::PalletId::get().0;
		let (imbalance, _remainder) = T::Currency::slash_reserved_named(&id, &who, deposit);
		T::SlashProposer::on_unbalanced(imbalance);
		Self::deposit_event(Event::CheckedNoPassSlashed {
			who: who.clone(),
			amount: deposit,
		});
		<Proposals<T, I>>::remove(proposal_hash);
		Ok(())
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ChallengeValidator<AccountId, Hash> {
	validator: AccountId,
	block_hash: Hash,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ChallengeInfo<AccountId, Hash, Balance, BlockNumber> {
	who: AccountId,
	target: ChallengeValidator<AccountId, Hash>,
	deposit: Balance,
	proposal: Hash,
	end: BlockNumber,
}
