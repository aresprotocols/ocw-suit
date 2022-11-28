use sp_std::fmt::Debug;
use super::*;
use frame_system::offchain::{SignedPayload, SigningTypes};
// use hex::ToHex;

// use ares_oracle::types::FractionLength;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::BoundedVec;


use scale_info::TypeInfo;
use sp_runtime::{
	traits::{Hash, Keccak256},
	Permill,
};
use frame_support::traits::ConstU32;
use frame_support::traits::tokens::Balance;
use sp_runtime::traits::AtLeast32BitUnsigned;
use sp_std::str;
use ares_oracle_provider_support::{FractionLength, JsonNumberValue, PriceKey};

pub type StringLimit = ConstU32<50>;

pub type MaximumOptions = ConstU32<10>;

pub type MaximumWinners = ConstU32<50000>;

pub type MaximumParticipants = ConstU32<50000>;

pub type MaximumEstimatesPerSymbol = ConstU32<10000>;

pub type MaximumEstimatesPerAccount = ConstU32<10000>;

pub type MaximumAdmins = ConstU32<100>;

pub type MaximumWhitelist = ConstU32<100>;

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo, Copy)]
pub enum Releases {
	V0,
	V1,
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum MultiplierOption {
	Base(u8),
}

impl Default for MultiplierOption {
	fn default() -> Self {
		MultiplierOption::Base(1)
	}
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum EstimatesState {
	InActive,
	Active,
	WaitingPayout,
	Completed,
	Unresolved,
}

impl Default for EstimatesState {
	fn default() -> Self {
		EstimatesState::InActive
	}
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum EstimatesType {
	DEVIATION,
	RANGE,
}

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode, MaxEncodedLen, TypeInfo)]
pub enum ChooseType<AccountId> {
	DEVIATION {
		low_price: u64,
		high_price: u64,
		check_list: Vec<(AccountId, Option<u64>)>
	},
	RANGE {
		range: BoundedVec<u64, MaximumOptions>,
		check_list: Vec<(AccountId, Option<u8>)>
	}
}


impl EstimatesType {
	pub fn get_type_number(self)-> u8 {
		match self {
			EstimatesType::DEVIATION => {2u8}
			EstimatesType::RANGE => {1u8}
		}
	}
}

impl Default for EstimatesType {
	fn default() -> Self {
		EstimatesType::DEVIATION
	}
}


// pub(crate) type BoundedVecOfSymbol = BoundedVec<u8, StringLimit>;
// pub(crate) type BoundedVecOfSymbol = BoundedVec<u8, StringLimit>;
pub(crate) type BoundedVecOfSymbol = PriceKey;

pub(crate) type BoundedVecOfAdmins<Account> = BoundedVec<Account, MaximumAdmins>;
pub(crate) type BoundedVecOfMultiplierOption = BoundedVec<MultiplierOption, MaximumOptions>;
pub(crate) type BoundedVecOfChooseWinnersPayload<ACC, BN> = BoundedVec<AccountParticipateEstimates<ACC, BN>, MaximumWinners>;
pub(crate) type BoundedVecOfCompletedEstimates<BN, Balance> = BoundedVec<SymbolEstimatesConfig<BN, Balance>, MaximumEstimatesPerSymbol>;

#[derive(Encode, Decode, Clone, Default, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct SymbolEstimatesConfig<BlockNumber, Balance> {
	pub symbol: BoundedVecOfSymbol,
	pub estimates_type: EstimatesType,
	/// Round ID
	pub id: u64,
	/// Price per entry.
	pub ticket_price: Balance,
	pub symbol_completed_price: u64,
	pub symbol_fraction: FractionLength,
	/// Starting block of the estimates.
	pub start: BlockNumber,
	/// ending block of the estimates
	pub end: BlockNumber,
	/// Delay for payout the winner of the estimates. (start + length + delay = payout).
	pub distribute: BlockNumber,
	// pub multiplier: BoundedVec<MultiplierOption, MaximumOptions>,
	pub multiplier: BoundedVecOfMultiplierOption,
	pub deviation: Option<Permill>,
	pub range: Option<BoundedVec<u64, MaximumOptions>>,
	pub total_reward: Balance,
	pub state: EstimatesState,
}

pub(crate) type BoundedVecOfBscAddress = BoundedVec<u8, StringLimit>;

#[derive(Encode, Decode, Clone, Default, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct AccountParticipateEstimates<Account, BlockNumber> {
	pub account: Account,

	pub end: BlockNumber,

	pub estimates: Option<u64>,

	pub range_index: Option<u8>,

	pub bsc_address: Option<BoundedVecOfBscAddress>,

	pub multiplier: MultiplierOption,

	pub reward: u128,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ChooseWinnersPayload<Public, AccountId, BlockNumber> {
	pub block_number: BlockNumber,
	pub winners: BoundedVecOfChooseWinnersPayload<AccountId, BlockNumber>,
	pub public: Option<Public>,
	pub estimates_id: u64,
	pub symbol: (BoundedVecOfSymbol,EstimatesType),
	pub price: Option<(u64, FractionLength, BlockNumber)>,
}

impl<T: SigningTypes> SignedPayload<T> for ChooseWinnersPayload<T::Public, T::AccountId, T::BlockNumber> {
	fn public(&self) -> T::Public {
		self.public.clone().unwrap()
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ChooseTrigerPayload<Public> {
	pub symbol: (BoundedVecOfSymbol, EstimatesType),
	pub public: Public,
}

impl<T: SigningTypes> SignedPayload<T> for ChooseTrigerPayload<T::Public> {
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}
