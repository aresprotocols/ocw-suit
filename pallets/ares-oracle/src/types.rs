
use super::*;

use sp_core::hexdisplay::HexDisplay;
// use sp_runtime::traits::{Saturating, Zero};
// use frame_support::sp_runtime::Percent;

pub type FractionLength = u32;
pub type RequestInterval = u8;

pub type OffchainSignature<T> = <T as SigningTypes>::Signature;

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
    // TODO:: Will be delete.
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
    // where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
          // u64: From<<T as frame_system::Config>::BlockNumber>,
{
    pub account_id: T::AccountId,
    pub offer: BalanceOf<T>,
    pub create_bn: T::BlockNumber,
    pub submit_threshold: u8,
    pub max_duration: u64,
    pub request_keys: Vec<Vec<u8>>,
}

impl <T: Config> Default for PurchasedRequestData<T>
    // where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
          // u64: From<<T as frame_system::Config>::BlockNumber>,
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
    // where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
          // u64: From<<T as frame_system::Config>::BlockNumber>,
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
pub struct AresPriceData<AccountId, BlockNumber>
    // where sp_runtime::AccountId32: From<AccountId>,
    //       u64: From<BlockNumber>,
{
    pub price: u64,
    pub account_id: AccountId,
    pub create_bn: BlockNumber,
    pub fraction_len: FractionLength,
    pub raw_number: JsonNumberValue,
    pub timestamp: u64,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PreCheckCompareLog {
    pub chain_avg_price_list: BTreeMap::<Vec<u8>, (u64, FractionLength)>,
    pub validator_up_price_list: BTreeMap::<Vec<u8>, (u64, FractionLength)>,
    pub raw_precheck_list: Vec<PreCheckStruct>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct HttpErrTraceData<BlockNumber, AuthorityId> {
    pub block_number: BlockNumber,
    // pub request_list: Vec<(Vec<u8>, Vec<u8>, u32)>,
    pub err_auth: AuthorityId,
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



/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PricePayload<Public, BlockNumber, AuthorityId> {
    pub block_number: BlockNumber,
    // price_key,price_val, fraction len
    pub price: Vec<PricePayloadSubPrice>,
    pub jump_block: Vec<PricePayloadSubJumpBlock>,
    pub auth: AuthorityId,
    pub public: Public,
}

impl<T: SigningTypes + Config > SignedPayload<T> for PricePayload<T::Public, T::BlockNumber, T::AuthorityAres> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PurchasedForceCleanPayload<Public, BlockNumber, AuthorityId> {
    pub block_number: BlockNumber,
    pub purchase_id_list: Vec<Vec<u8>>,
    pub auth: AuthorityId,
    pub public: Public,
}

impl<T: SigningTypes + Config > SignedPayload<T> for PurchasedForceCleanPayload<T::Public, T::BlockNumber, T::AuthorityAres> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PurchasedPricePayload<Public, BlockNumber, AuthorityId> {
    pub block_number: BlockNumber,
    pub purchase_id: Vec<u8>,
    pub price: Vec<PricePayloadSubPrice>,
    pub auth: AuthorityId,
    pub public: Public,
}

impl<T: SigningTypes + Config > SignedPayload<T> for PurchasedPricePayload<T::Public, T::BlockNumber, T::AuthorityAres> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

/// stash: T::AccountId, auth: T::AuthorityAres, bn: T::BlockNumber
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PreCheckPayload<Public, BlockNumber, AccountId, AuthorityId> {
    pub block_number: BlockNumber,
    pub pre_check_stash: AccountId,
    pub pre_check_auth: AuthorityId,
    pub auth: AuthorityId,
    pub public: Public,
}

impl<T: SigningTypes + Config > SignedPayload<T> for PreCheckPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>  {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PreCheckResultPayload<Public, BlockNumber, AccountId, AuthorityId> {
    pub block_number: BlockNumber,
    pub pre_check_list: Vec<PreCheckStruct>,
    pub pre_check_stash: AccountId,
    pub pre_check_auth: AuthorityId,
    pub public: Public,
}

impl<T: SigningTypes + Config > SignedPayload<T> for PreCheckResultPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct HttpErrTracePayload<Public, BlockNumber, AuthorityId> {
    pub trace_data: HttpErrTraceData<BlockNumber, AuthorityId>,
    pub auth: AuthorityId,
    pub public: Public,
}

impl<T: SigningTypes + Config > SignedPayload<T> for HttpErrTracePayload<T::Public, T::BlockNumber, T::AuthorityAres> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PricePayloadSubPrice(pub Vec<u8>, pub u64, pub FractionLength, pub JsonNumberValue, pub u64,);

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
