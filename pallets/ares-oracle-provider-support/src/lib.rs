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

// warp NumberValue
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct JsonNumberValue {
	pub integer: u64,
	pub fraction: u64,
	pub fraction_length: u32,
	pub exponent: u32,
}

//
impl JsonNumberValue {
	pub fn new(number_value: NumberValue) -> Self {
		if number_value.integer < 0 || number_value.exponent != 0 {
			panic!("⛔ Error source NumberValue integer or exponent.");
		}
		Self {
			fraction_length: number_value.fraction_length,
			fraction: number_value.fraction,
			exponent: number_value.exponent as u32,
			integer: number_value.integer as u64,
		}
	}

	pub fn to_price(&self, fraction_number: FractionLength) -> u64 {
		let mut price_fraction = self.fraction;
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

// The following code for `per check` functionable
//
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum PreCheckStatus {
	Review,
	Prohibit,
	Pass,
}

impl Default for PreCheckStatus {
	fn default() -> Self {
		Self::Prohibit
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug,  TypeInfo)]
pub struct PreCheckTaskConfig {
	pub check_token_list: TokenList,
	pub allowable_offset: Percent,
	/* pub max_repeat_times: u8, // The current version is forbidden first.
	 * pub pass_percent: Percent, // The current version is forbidden first. */
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

/// `Pre-Check trait`
pub trait IAresOraclePreCheck<AccountId, AuthorityId, BlockNumber> {

	/// Determine whether there is a pre-check task for the `validator` through a stash account.
	fn has_pre_check_task(stash: AccountId) -> bool;

	/// Get the pre-check information related to a certain `ares-authority` collection,
	/// the specific matching authority-id, account-id, and the block submitted by the task.
	///
	/// Precheck tasks that only match the first `ares-authority`
	fn get_pre_task_by_authority_set(auth_list: Vec<AuthorityId>) -> Option<(AccountId, AuthorityId, BlockNumber)>;

	//
	fn check_and_clean_obsolete_task(maximum_due: BlockNumber) -> Weight;

	/// Obtain a set of price data according to the task configuration structure.
	fn take_price_for_pre_check(check_config: PreCheckTaskConfig) -> PreCheckList;

	/// Record the per check results and add them to the storage structure.
	fn save_pre_check_result(stash: AccountId, bn: BlockNumber, pre_check_list: PreCheckList) -> PreCheckStatus;

	//
	fn get_pre_check_status(stash: AccountId) -> Option<(BlockNumber, PreCheckStatus)>;

	//
	fn clean_pre_check_status(stash: AccountId);

	//
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
	fn save_pre_check_result(_stash: AC, _bn: B, _pre_check_list: PreCheckList) -> PreCheckStatus { PreCheckStatus::Review }
	fn get_pre_check_status(_stash: AC) -> Option<(B, PreCheckStatus)> {
		None
	}
	fn clean_pre_check_status(_stash: AC) {}
	fn create_pre_check_task(_stash: AC, _auth: AU, _bn: B) -> bool {
		false
	}
}
