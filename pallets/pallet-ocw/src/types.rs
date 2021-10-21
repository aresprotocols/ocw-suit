
use super::*;

pub type FractionLength = u32;
pub type RequestInterval = u8;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PurchasedRequestData<T: Config>
    where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
          u64: From<<T as frame_system::Config>::BlockNumber>,
{
    pub account_id: T::AccountId,
    pub offer: u64,
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
            offer: 0,
            submit_threshold: 0,
            max_duration: 0,
            request_keys: Vec::new(),
        }
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
pub struct AresPriceData<T:Config>
    where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
          u64: From<<T as frame_system::Config>::BlockNumber>,
{
    pub price: u64,
    pub account_id: T::AccountId,
    pub create_bn: T::BlockNumber,
    pub fraction_len: FractionLength,
    pub raw_number: JsonNumberValue,
}

impl <T:Config> AresPriceData<T>
    where sp_runtime::AccountId32: From<<T as frame_system::Config>::AccountId>,
          u64: From<<T as frame_system::Config>::BlockNumber>,
{
    pub fn from_tuple(param: (u64, T::AccountId, T::BlockNumber, FractionLength, JsonNumberValue)) -> Self {
        Self {
            price: param.0,
            account_id: param.1,
            create_bn: param.2,
            fraction_len: param.3,
            raw_number: param.4,
        }
    }
}

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

