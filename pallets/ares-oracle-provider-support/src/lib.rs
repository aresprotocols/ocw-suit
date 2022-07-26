#![cfg_attr(not(feature = "std"), no_std)]

// use ares_common::limit::{MaximumSymbolList, StringLimit};
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::sp_runtime::Percent;
use frame_support::traits::ConstU32;
use frame_support::weights::Weight;
use frame_support::{BoundedVec, RuntimeDebug};
use lite_json::NumberValue;
use scale_info::TypeInfo;
use sp_std::vec::Vec;

pub const LOCAL_STORAGE_PRICE_REQUEST_MAKE_POOL: &[u8] = b"are-ocw::make_price_request_pool";
pub const LOCAL_STORAGE_PRICE_REQUEST_LIST: &[u8] = b"are-ocw::price_request_list";
pub const LOCAL_STORAGE_PRICE_REQUEST_DOMAIN: &[u8] = b"are-ocw::price_request_domain";
pub const LOCAL_HOST_KEY: &[u8] = b"are-ocw::local_host_key";

/// For `ares` authority.
pub mod crypto;

pub type MaximumPriceKey = ConstU32<15>;
pub type MaximumPriceToken = ConstU32<15>;
pub type MaximumPreCheckListSize = ConstU32<500>;
pub type MaximumTokeListSize = ConstU32<1000>;
pub type MaximumRequestBaseUrlSize =  ConstU32<500>;
pub type MaximumAresOracleAuthoritieSize =  ConstU32<500>;
pub type MaximumPoolSize = ConstU32<1000>;

pub type PriceKey = BoundedVec<u8, MaximumPriceKey>; // Vec<u8>;
pub type PriceToken = BoundedVec<u8, MaximumPriceToken>; // Vec<u8>;
pub type FractionLength = u32;

pub type RawSourceKeys = BoundedVec<(PriceKey, PriceToken, FractionLength), MaximumPoolSize>;
pub type RequestKeys = BoundedVec<PriceKey, MaximumPoolSize>;

pub type PreCheckList = BoundedVec<PreCheckStruct, MaximumPoolSize>;
pub type TokenList = BoundedVec<PriceToken, MaximumPoolSize>;

// A wrapper structure for NumberValue that handles the conversion of precision to u64
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct JsonNumberValue {
	pub integer: u64,
	pub fraction: u64,
	pub fraction_length: u32,
	pub exponent: i32,
}

/// Convert `NumberValue` to `JsonNumberValue`
///
/// # Examples
/// ```
/// let number1 = JsonNumberValue {
///  	integer: 8,
///  	fraction: 87654,
///  	fraction_length: 5,
///  	exponent: 0,
/// };
/// assert_eq!(8876540, number1.to_price(6));
/// assert_eq!(887654, number1.to_price(5));
/// assert_eq!(88765, number1.to_price(4));
/// assert_eq!(8876, number1.to_price(3));
/// assert_eq!(887, number1.to_price(2));
/// assert_eq!(88, number1.to_price(1));
/// assert_eq!(8, number1.to_price(0));
/// ```
impl JsonNumberValue {
	/// Input `NumberValue` to create a `JsonNumberValue`
	pub fn new(number_value: NumberValue) -> Self {
		let res = Self::try_new(number_value);
		if res.is_some() {
			return res.unwrap();
		}
		panic!("⛔ Error source NumberValue integer");
	}

	pub fn try_new(number_value: NumberValue) -> Option<Self> {
		if number_value.integer < 0 {
			return None;
		}
		Some(Self {
			fraction_length: number_value.fraction_length,
			fraction: number_value.fraction,
			exponent: number_value.exponent,
			integer: number_value.integer as u64,
		})
	}

	/// Formats a u64 integer given a fractional length
	pub fn to_price(&self, fraction_number: FractionLength) -> u64 {
		let mut price_fraction = self.fraction;
		let fraction_number = fraction_number as i32 + self.exponent;
		if fraction_number<0 {
			return 0
		}
		let fraction_number = fraction_number as FractionLength;
		if price_fraction < 10u64.pow(fraction_number) {
			price_fraction *= 10u64.pow(fraction_number.checked_sub(self.fraction_length).unwrap_or(0));
		}
		let exp = self.fraction_length.checked_sub(fraction_number).unwrap_or(0);
		self.integer as u64 * (10u64.pow(fraction_number)) + (price_fraction / 10_u64.pow(exp))
	}
}

#[cfg(feature = "std")]
impl Default for JsonNumberValue {
	fn default() -> Self {
		Self {
			fraction_length: 0,
			fraction: 0,
			exponent: 0,
			integer: 0,
		}
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PreCheckStruct {
	pub price_key: PriceKey,
	pub number_val: JsonNumberValue,
	pub max_offset: Percent,
	pub timestamp: u64,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum PreCheckStatus {
	/// Review status, waiting for validators to submit `price` data.
	Review,
	/// The validator has submitted the price data,
	/// but the deviation is too large after comparing with the data on the chain,
	/// and the review fails
	Prohibit,
	/// Review passed
	Pass,
}

impl Default for PreCheckStatus {
	fn default() -> Self {
		Self::Prohibit
	}
}

/// Pre-checked state configuration data, which is saved on-chain
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug,  TypeInfo)]
pub struct PreCheckTaskConfig {
	/// List of `Trading pairs` to check.
	pub check_token_list: TokenList,
	/// The maximum allowable offset percentage when compared
	/// to the on-chain average price
	pub allowable_offset: Percent,
}

impl Default for PreCheckTaskConfig {
	fn default() -> Self {
		Self {
			check_token_list: Default::default(),
			allowable_offset: Percent::from_percent(0),
			/* max_repeat_times: 5,
			 * pass_percent: Percent::from_percent(100), */
		}
	}
}

/// `Pre-Check`trait
pub trait IAresOraclePreCheck<AccountId, AuthorityId, BlockNumber> {

	/// Determine whether there is a pre-check task for the `validator` through a stash account.
	fn has_pre_check_task(stash: AccountId) -> bool;

	/// Get the pre-check information related to a certain `ares-authority` collection,
	/// the specific matching authority-id, account-id, and the block submitted by the task.
	///
	/// Precheck tasks that only match the first `ares-authority`
	fn get_pre_task_by_authority_set(auth_list: Vec<AuthorityId>) -> Option<(AccountId, AuthorityId, BlockNumber)>;

	/// Trigger this method on a specific cycle to clean up too old and passed tasks
	fn check_and_clean_obsolete_task(maximum_due: BlockNumber) -> Weight;

	/// Obtain `PreCheckList` result data according to `Trading pairs` specified by `check_config`
	fn take_price_for_pre_check(check_config: PreCheckTaskConfig) -> PreCheckList;

	/// Will verify the data on-chain based on the result of `PreCheckList` and return `PreCheckStatus` as the result
	fn save_pre_check_result(stash: AccountId, bn: BlockNumber, pre_check_list: PreCheckList, auth: AuthorityId) -> PreCheckStatus;

	/// Get the pre-check status that a validator has stored,
	/// this status will affect whether it will be added to the validator list.
	fn get_pre_check_status(stash: AccountId) -> Option<(BlockNumber, PreCheckStatus)>;

	/// Remove pre-check status stored by a validator
	fn clean_pre_check_status(stash: AccountId);

	/// Create a pre-check task, return true if the creation is successful else return false
	fn create_pre_check_task(stash: AccountId, auth: AuthorityId, bn: BlockNumber) -> bool;
}


impl<AC, AU, B> IAresOraclePreCheck<AC, AU, B> for () {
	fn has_pre_check_task(_stash: AC) -> bool {
		false
	}
	fn get_pre_task_by_authority_set(_auth_list: Vec<AU>) -> Option<(AC, AU, B)> {
		None
	}
	fn check_and_clean_obsolete_task(_maximum_due: B) -> u64 {
		0
	}
	fn take_price_for_pre_check(_check_config: PreCheckTaskConfig) -> PreCheckList {
		Default::default()
	}
	fn save_pre_check_result(_stash: AC, _bn: B, _pre_check_list: PreCheckList, auth: AU) -> PreCheckStatus { PreCheckStatus::Review }
	fn get_pre_check_status(_stash: AC) -> Option<(B, PreCheckStatus)> {
		None
	}
	fn clean_pre_check_status(_stash: AC) {}
	fn create_pre_check_task(_stash: AC, _auth: AU, _bn: B) -> bool {
		false
	}
}
