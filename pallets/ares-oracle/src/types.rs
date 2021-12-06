
use super::*;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::{Saturating, Zero};
use frame_support::sp_runtime::Percent;

pub type FractionLength = u32;
pub type RequestInterval = u8;



#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct OcwControlData
{
    pub need_verifier_check: bool,
    pub open_free_price_reporter: bool,
    pub open_paid_price_reporter: bool,
}

impl Default for OcwControlData
{
    fn default() -> Self {
        Self {
            need_verifier_check: true,
            open_free_price_reporter: true,
            open_paid_price_reporter: true,
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PurchasedDefaultData
{
    pub submit_threshold: u8,
    pub max_duration: u64,
    pub avg_keep_duration: u64,
    pub unit_price: u64,
}


impl PurchasedDefaultData {
    pub fn new(submit_threshold: u8, max_duration: u64, avg_keep_duration: u64, unit_price: u64) -> Self {
        if submit_threshold == 0 || submit_threshold > 100 {
            panic!("Submit Threshold range is (0 - 100] ");
        }
        if max_duration == 0 {
            panic!("Max Duration can not be 0.");
        }
        Self {
            submit_threshold,
            max_duration,
            avg_keep_duration,
            unit_price,
        }
    }
}

impl Default for PurchasedDefaultData
{
    fn default() -> Self {
        Self {
            submit_threshold: 60,
            max_duration: 20,
            avg_keep_duration: 14400,
            unit_price: 100_000_000_000_000,
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct PurchasedSourceRawKeys
{
    pub purchase_id: Vec<u8>,
    pub raw_source_keys: Vec<(Vec<u8>, Vec<u8>, FractionLength)>,
}

impl Default for PurchasedSourceRawKeys
{
    fn default() -> Self {
        Self {
            purchase_id: Vec::new(),
            raw_source_keys: Vec::new(),
        }
    }
}

// Impl debug.
impl fmt::Debug for PurchasedSourceRawKeys {
    // `fmt` converts the vector of bytes inside the struct back to string for
    //  more friendly display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let raw_keys: Vec<_> = self.raw_source_keys.iter().map(
            |(price_key,parse_key,fraction_len)| {
                (
                    str::from_utf8(price_key).unwrap(),
                    str::from_utf8(parse_key).unwrap(),
                    fraction_len
                )
            }
        ).collect();
        write!(
            f,
            "{{( purchase_id: {:?}, raw_source_keys: {:?} )}}",
            HexDisplay::from(&self.purchase_id),
            // str::from_utf8(&self.0).map_err(|_| fmt::Error)?,
            raw_keys,
        )
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq,)]
pub struct PurchasedRequestData<T: Config>
    where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
          u64: From<<T as frame_system::Config>::BlockNumber>,
{
    pub account_id: T::AccountId,
    pub offer: BalanceOf<T>,
    pub create_bn: T::BlockNumber,
    pub submit_threshold: u8,
    pub max_duration: u64,
    pub request_keys: Vec<Vec<u8>>,
}

impl <T: Config> Default for PurchasedRequestData<T>
    where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
          u64: From<<T as frame_system::Config>::BlockNumber>,
{
    fn default() -> Self {
        Self {
            account_id: Default::default(),
            offer: 0u32.into(),
            create_bn: Default::default(),
            submit_threshold: 0,
            max_duration: 0,
            request_keys: Vec::new(),
        }
    }
}

// Impl debug.
impl <T: Config> fmt::Debug for PurchasedRequestData<T>
    where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
          u64: From<<T as frame_system::Config>::BlockNumber>,
{
    // `fmt` converts the vector of bytes inside the struct back to string for
    //  more friendly display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let request_keys: Vec<_> = self.request_keys.iter().map(
            |x| {
                str::from_utf8(x).unwrap()
            }
        ).collect();
        write!(
            f,
            "{{( account_id: {:?}, offer: {:?}, submit_threshold: {:?}, max_duration: {:?}, request_keys: {:?} )}}",
            &self.account_id,
            &self.offer,
            &self.submit_threshold,
            &self.max_duration,
            request_keys,
        )
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PurchasedAvgPriceData
{
    pub create_bn: u64,
    pub reached_type: u8,
    pub price_data: (u64, FractionLength),
}

impl Default for PurchasedAvgPriceData
{
    fn default() -> Self {
        Self {
            create_bn: 0,
            reached_type: 0,
            price_data: (0, 0)
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PerCheckStruct {
    pub price_key: Vec<u8>,
    pub number_val: JsonNumberValue,
    pub max_offset: Percent,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct AresPriceData<AccountId, BlockNumber>
    // where sp_runtime::AccountId32: From<AccountId>,
    //       u64: From<BlockNumber>,
{
    pub price: u64,
    pub account_id: AccountId,
    pub create_bn: BlockNumber,
    pub fraction_len: FractionLength,
    pub raw_number: JsonNumberValue,
}


// impl <T: Config > AresPriceData<T>
//     where sp_runtime::AccountId32: From<T::AccountId>,
//           u64: From<T::BlockNumber>,
// {
//     pub fn from_tuple(param: (u64, T::AccountId, T::BlockNumber, FractionLength, JsonNumberValue)) -> Self {
//         Self {
//             price: param.0,
//             account_id: param.1,
//             create_bn: param.2,
//             fraction_len: param.3,
//             raw_number: param.4,
//         }
//     }
// }

// #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
// pub struct AresPriceData2<AccountId, BlockNumber>
//     where sp_runtime::AccountId32: From<AccountId>,
//           u64: From<BlockNumber>,
// {
//     pub price: u64,
//     pub account_id: AccountId,
//     pub create_bn: BlockNumber,
//     pub fraction_len: FractionLength,
//     pub raw_number: JsonNumberValue,
// }
//
// impl <T: Config > AresPriceData2<T::AccountId, T::BlockNumber>
// {
//     pub fn from_tuple(param: (u64, T::AccountId, T::BlockNumber, FractionLength, JsonNumberValue)) -> Self {
//         Self {
//             price: param.0,
//             account_id: param.1,
//             create_bn: param.2,
//             fraction_len: param.3,
//             raw_number: param.4,
//         }
//     }
// }

// pub struct AresPriceData3<T: Config>
// {
// }
// impl <T: Config > AresPriceData3<T>
// {
//     pub fn from_tuple(param: (u64, T::AccountId, T::BlockNumber, FractionLength, JsonNumberValue)) -> Self {
//     }
// }


// trait IFromPriceData <AccountId,BlockNumber>  {
//     fn from_tuple(param: (u64, AccountId, BlockNumber, FractionLength, JsonNumberValue)) -> Self;
// }

// impl <T: Config> IFromPriceData<T::AccountId, T::BlockNumber> for AresPriceData2<T::AccountId, T::BlockNumber> {
//     fn from_tuple(param: (u64, T::AccountId, T::BlockNumber, u32, JsonNumberValue)) -> Self {
//         todo!()
//     }
// }



// impl <T: Config, AccountId, BlockNumber> AresPriceData<AccountId, BlockNumber, T>
//     where sp_runtime::AccountId32: From<AccountId>,
//           u64: From<BlockNumber>,
//            AccountId: From<<T as frame_system::Config>::AccountId>,
//            BlockNumber: From<<T as frame_system::Config>::BlockNumber>,
// {
//     pub fn from_tuple(param: (u64, AccountId, BlockNumber, FractionLength, JsonNumberValue)) -> Self {
//         Self {
//             price: param.0,
//             account_id: param.1,
//             create_bn: param.2,
//             fraction_len: param.3,
//             raw_number: param.4,
//         }
//     }
// }

// impl <T: Config> AresPriceData<T::AccountId, T::BlockNumber>
//     where sp_runtime::AccountId32: From<T::AccountId>,
//           u64: From<T::BlockNumber>,
// {
//     pub fn from_tuple(param: (u64, T::AccountId, T::BlockNumber, FractionLength, JsonNumberValue)) -> Self {
//         Self {
//             price: param.0,
//             account_id: param.1,
//             create_bn: param.2,
//             fraction_len: param.3,
//             raw_number: param.4,
//         }
//     }
// }

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

    pub fn toPrice(&self, fraction_number: FractionLength) -> u64 {
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

/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PricePayload<Public, BlockNumber> {
    pub block_number: BlockNumber,
    // price_key,price_val, fraction len
    pub price: Vec<PricePayloadSubPrice>,
    pub jump_block: Vec<PricePayloadSubJumpBlock>,
    pub public: Public,
}

impl<T: SigningTypes> SignedPayload<T> for PricePayload<T::Public, T::BlockNumber> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PurchasedForceCleanPayload<Public, BlockNumber> {
    pub block_number: BlockNumber,
    pub purchase_id_list: Vec<Vec<u8>>,
    pub public: Public,
}

impl<T: SigningTypes> SignedPayload<T> for PurchasedForceCleanPayload<T::Public, T::BlockNumber> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}


#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PurchasedPricePayload<Public, BlockNumber> {
    pub block_number: BlockNumber,
    pub purchase_id: Vec<u8>,
    pub price: Vec<PricePayloadSubPrice>,
    pub public: Public,
}

impl<T: SigningTypes> SignedPayload<T> for PurchasedPricePayload<T::Public, T::BlockNumber> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PerCheckPayload<Public, BlockNumber> {
    pub block_number: BlockNumber,
    // price_key,price_val, fraction len
    pub price: Vec<PricePayloadSubPrice>,
    pub jump_block: Vec<PricePayloadSubJumpBlock>,
    pub public: Public,
}

impl<T: SigningTypes> PerCheckPayload<T> for PricePayload<T::Public, T::BlockNumber> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}


#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PricePayloadSubPrice(pub Vec<u8>, pub u64, pub FractionLength, pub JsonNumberValue);

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct PricePayloadSubJumpBlock(pub Vec<u8>, pub RequestInterval); // price_key ,request_interval

// Impl debug.
impl fmt::Debug for PricePayloadSubJumpBlock {
    // `fmt` converts the vector of bytes inside the struct back to string for
    //  more friendly display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{( price_key: {}, request_interval: {} )}}",
            str::from_utf8(&self.0).map_err(|_| fmt::Error)?,
            &self.1,
        )
    }
}

// The following code for `per check` functionable
//
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum PerCheckStatus {
    Review,
    Prohibit,
    Pass,
}

impl Default for PerCheckStatus {
    fn default() -> Self { Self::Prohibit }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PerCehckTaskConfig {
    pub check_token_list: Vec<Vec<u8>>,
    pub allowable_offset: Percent,
    // pub max_repeat_times: u8, // The current version is forbidden first.
    // pub pass_percent: Percent, // The current version is forbidden first.
}

impl Default for PerCehckTaskConfig
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