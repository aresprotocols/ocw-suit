#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::Weight;
use frame_support::RuntimeDebug;
use frame_support::pallet_prelude::{Decode, Encode};
use frame_support::sp_runtime::Percent;
use lite_json::NumberValue;
use sp_std::vec::Vec;

pub mod crypto;

pub type FractionLength = u32;

// warp NumberValue
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
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
            price_fraction *= 10u64.pow(
                fraction_number
                    .checked_sub(self.fraction_length)
                    .unwrap_or(0),
            );
        }
        let exp = self
            .fraction_length
            .checked_sub(fraction_number)
            .unwrap_or(0);
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PreCheckStruct {
    pub price_key: Vec<u8>,
    pub number_val: JsonNumberValue,
    pub max_offset: Percent,
    pub timestamp: u64,
}

// The following code for `per check` functionable
//
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum PreCheckStatus {
    Review,
    Prohibit,
    Pass,
}

impl Default for PreCheckStatus {
    fn default() -> Self { Self::Prohibit }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PreCheckTaskConfig {
    pub check_token_list: Vec<Vec<u8>>,
    pub allowable_offset: Percent,
    // pub max_repeat_times: u8, // The current version is forbidden first.
    // pub pass_percent: Percent, // The current version is forbidden first.
}

impl Default for PreCheckTaskConfig
{
    fn default() -> Self {
        Self {
            check_token_list: Vec::new(),
            allowable_offset: Percent::from_percent(0),
            // max_repeat_times: 5,
            // pass_percent: Percent::from_percent(100),
        }
    }
}

pub trait IAresOraclePreCheck <AccountId, AuthorityId, BlockNumber>
{
    //
    fn has_pre_check_task(stash: AccountId) -> bool;

    //
    fn get_pre_task_by_authority_set(auth_list: Vec<AuthorityId>) -> Option<(AccountId, AuthorityId, BlockNumber)>;

    //
    fn check_and_clean_obsolete_task(maximum_due: BlockNumber) -> Weight;

    // Obtain a set of price data according to the task configuration structure.
    fn take_price_for_per_check(check_config: PreCheckTaskConfig) -> Vec<PreCheckStruct>;

    // Record the per check results and add them to the storage structure.
    fn save_pre_check_result(stash: AccountId, bn: BlockNumber, pre_check_list: Vec<PreCheckStruct>);

    //
    fn get_pre_check_status(stash: AccountId) -> Option<(BlockNumber, PreCheckStatus)> ;

    //
    fn clean_pre_check_status(stash: AccountId)  ;

    //
    fn create_pre_check_task(stash: AccountId, auth: AuthorityId, bn: BlockNumber) -> bool;
}

impl <AC,AU,B> IAresOraclePreCheck <AC,AU,B> for () {
    fn has_pre_check_task(_stash: AC) -> bool {
        false
    }

    fn get_pre_task_by_authority_set(_auth_list: Vec<AU>) -> Option<(AC, AU, B)> {
        None
    }

    fn check_and_clean_obsolete_task(_maximum_due: B) -> u64 {
        0
    }

    fn take_price_for_per_check(_check_config: PreCheckTaskConfig) -> Vec<PreCheckStruct> {
        Vec::new()
    }

    fn save_pre_check_result(_stash: AC, _bn: B, _pre_check_list: Vec<PreCheckStruct>) {}

    fn get_pre_check_status(_stash: AC) -> Option<(B, PreCheckStatus)> {
        None
    }

    fn clean_pre_check_status(_stash: AC) {}

    fn create_pre_check_task(_stash: AC, _auth: AU, _bn: B) -> bool {
       false
    }
}