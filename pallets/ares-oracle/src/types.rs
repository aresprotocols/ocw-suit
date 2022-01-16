
use super::*;

use sp_core::hexdisplay::HexDisplay;

pub type FractionLength = u32;
pub type RequestInterval = u8;
pub type OffchainSignature<T> = <T as SigningTypes>::Signature;

pub type PurchasedId = Vec<u8>;
pub type PriceKey = Vec<u8>;
pub type PriceToken = Vec<u8>;

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

// Migration
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PurchasedDefaultData
{
    pub submit_threshold: u8,
    pub max_duration: u64,
    pub avg_keep_duration: u64,
    // TODO:: Will be delete.
    // pub unit_price: u64,
}


impl PurchasedDefaultData {
    pub fn new(submit_threshold: u8, max_duration: u64, avg_keep_duration: u64) -> Self {
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
            // unit_price,
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
            // unit_price: 100_000_000_000_000,
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
{
    pub account_id: T::AccountId,
    pub offer: BalanceOf<T>,
    pub create_bn: T::BlockNumber,
    pub submit_threshold: u8,
    pub max_duration: u64,
    pub request_keys: Vec<Vec<u8>>,
}

impl <T: Config> Default for PurchasedRequestData<T>
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
{
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
{
    pub price: u64,
    pub account_id: AccountId,
    pub create_bn: BlockNumber,
    pub fraction_len: FractionLength,
    pub raw_number: JsonNumberValue,
    pub timestamp: u64,
}

// migrated
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct AvgPriceData {
    pub integer: u64,
    pub fraction_len: FractionLength,
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
    pub err_status: HttpError,
    pub tip: Vec<u8>,
}


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
pub struct HttpErrTracePayload<Public, BlockNumber, AuthorityId, AccountId> {
    pub trace_data: HttpErrTraceData<BlockNumber, AccountId>,
    pub auth: AuthorityId,
    pub public: Public,
}

impl<T: SigningTypes + Config > SignedPayload<T> for HttpErrTracePayload<T::Public, T::BlockNumber, T::AuthorityAres, T::AccountId> {
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PricePayloadSubPrice(pub PriceKey, pub u64, pub FractionLength, pub JsonNumberValue, pub u64,);

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct PricePayloadSubJumpBlock(pub PriceKey, pub RequestInterval); // price_key ,request_interval

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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum HttpError {
    IoErr(Vec<u8>),
    TimeOut(Vec<u8>),
    StatusErr(Vec<u8>, u16),
    ParseErr(Vec<u8>),
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug)]
pub(crate) enum Releases {
    V1_0_0_Ancestral,
    V1_0_1_HttpErrUpgrade,
    V1_1_0_HttpErrUpgrade,
    V1_2_0,
}

impl Default for Releases {
    fn default() -> Self {
        Releases::V1_0_0_Ancestral
    }
}