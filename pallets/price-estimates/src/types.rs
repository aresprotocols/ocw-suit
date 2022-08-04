use sp_std::fmt::Debug;
use super::*;
use frame_system::offchain::{SignedPayload, SigningTypes};
use hex::ToHex;

// use ares_oracle::types::FractionLength;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::tokens::Balance;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{Hash, Keccak256},
	Permill,
};
use sp_runtime::traits::AtLeast32BitUnsigned;
use sp_std::str;
use ares_oracle_provider_support::JsonNumberValue;

pub type StringLimit = ConstU32<50>;

pub type MaximumOptions = ConstU32<10>;

pub type MaximumWinners = ConstU32<50000>;

pub type MaximumParticipants = ConstU32<50000>;

pub type MaximumEstimatesPerSymbol = ConstU32<10000>;

pub type MaximumEstimatesPerAccount = ConstU32<10000>;

pub type MaximumAdmins = ConstU32<100>;

pub type MaximumWhitelist = ConstU32<100>;

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

impl Default for EstimatesType {
	fn default() -> Self {
		EstimatesType::DEVIATION
	}
}

pub(crate) type BoundedVecOfMultiplierOption = BoundedVec<MultiplierOption, MaximumOptions>;
pub(crate) type BoundedVecOfConfigRange = BoundedVec<u64, MaximumOptions>;

#[derive(Encode, Decode, Clone, Default, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct SymbolEstimatesConfig<BlockNumber, Balance> {
	pub symbol: BoundedVecOfPreparedEstimates,

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

pub(crate) type BoundedVecOfChooseWinnersPayload<ACC, BN> = BoundedVec<AccountParticipateEstimates<ACC, BN>, MaximumWinners>;
pub(crate) type BoundedVecOfSymbol = BoundedVec<u8, StringLimit>;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ChooseWinnersPayload<Public, AccountId, BlockNumber> {
	pub block_number: BlockNumber,
	pub winners: BoundedVecOfChooseWinnersPayload<AccountId, BlockNumber>,
	pub public: Option<Public>,
	pub estimates_id: u64,
	pub symbol: BoundedVecOfSymbol,
	pub price: Option<(u64, FractionLength, BlockNumber)>,
}


impl<T: SigningTypes> SignedPayload<T> for ChooseWinnersPayload<T::Public, T::AccountId, T::BlockNumber> {
	fn public(&self) -> T::Public {
		self.public.clone().unwrap()
	}
}

pub fn is_eth_address(address: &[u8]) -> bool {
	let _address = str::from_utf8(address).unwrap();

	// let basic = Regex::new(r"^(0x)?(?i)([0-9a-f]{40})$").unwrap();
	// let lowercase = Regex::new(r"^(0x|0X)?[0-9a-f]{40}$").unwrap();
	// let uppercase = Regex::new(r"^(0x|0X)?[0-9A-F]{40}$").unwrap();

	// check if it has the basic requirements of an address( case-insensitive )
	// if basic.find(address).is_none() {
	//     false
	//     // If it's ALL lowercase or ALL uppercase
	// } else if lowercase.find(address).is_some() || uppercase.find(address).is_some() {
	//     true
	// } else {
	//     eth_checksum(address)
	// }
	eth_checksum(address)
}

pub fn is_hex_address(address: &[u8]) -> bool {
	// log::info!("test-hex: {:?} ,length: {}", address, address.len());
	if address.len() != 40 {
		return false;
	}
	for (_i, x) in address.iter().enumerate() {
		let c: char = char::from(*x);
		/*if i < 2 {
			// check 0x prefix
			if !((i == 0 && c == '0') || (i == 1 && c == 'x') || (i == 1 && c == 'X')) {
				return false;
			}
		} else {*/
		if !(('0' <= c && c <= '9') || ('a' <= c && c <= 'f') || ('A' <= c && c <= 'F')) {
			return false;
		}
		//}
	}
	true
}

fn eth_checksum(address: &[u8]) -> bool {
	let _address = address.to_ascii_lowercase();
	let address_hash = Keccak256::hash(_address.as_slice());
	let address_hash_bytes: Vec<char> = address_hash.encode_hex();
	let address_hash_bytes = address_hash_bytes.as_slice();
	// println!("checksum2 address_hash {:?}", &address_hash);
	// println!("checksum2 address_hash_bytes {:?}", address_hash_bytes);

	for (index, x) in address.iter().enumerate() {
		let c = address_hash_bytes[index];
		let n = c.to_digit(16).unwrap();

		let a = *x;
		let mut _tmp = a.clone();
		if n > 7 {
			_tmp.make_ascii_uppercase();
			if _tmp != a {
				return false;
			}
		} else {
			_tmp.make_ascii_lowercase();
			if _tmp != a {
				return false;
			}
		}
	}
	// println!("true");
	return true;
	// return str::from_utf8(b"aaa").unwrap();
}

pub trait ConvertChainPrice<B, F> {
	fn try_to_price(self, fraction: F) -> Option<B>;
	fn convert_to_json_number_value(self) -> JsonNumberValue;
}

#[derive(Encode, Decode, Clone, Default, Eq, PartialEq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ChainPrice {
	number: u64,
	fraction_length: u32,
}

impl ChainPrice {
	pub fn new(info: (u64, u32)) -> Self {
		Self {
			number: info.0,
			fraction_length: info.1
		}
	}
}

impl <B: Balance, F: AtLeast32BitUnsigned + Debug> ConvertChainPrice<B, F> for ChainPrice {
	fn try_to_price(self, to_fraction: F) -> Option<B> {
		let to_fraction: Option<u8> = to_fraction.try_into().ok();
		if let Some(to_fraction) = to_fraction {
			// let new_number = self.convert_to_json_number_value().to_price(to_fraction as u32);
			let new_number = ConvertChainPrice::<B,F>::convert_to_json_number_value(self).to_price(to_fraction as u32);
			return new_number.try_into().ok();
		}
		None
	}

	fn convert_to_json_number_value(self) -> JsonNumberValue {
		let integer = self.number / 10u64.pow(self.fraction_length);
		let fraction: u64 = self.number - integer.saturating_mul(10u64.pow(self.fraction_length));
		JsonNumberValue {
			integer: integer,
			fraction: fraction,
			fraction_length: self.fraction_length ,
			exponent: 0
		}
	}
}