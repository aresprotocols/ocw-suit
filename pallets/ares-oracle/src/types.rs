use super::*;
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::storage::bounded_btree_map::BoundedBTreeMap;
use frame_support::traits::ConstU32;
use frame_support::BoundedVec;
use oracle_finance::types::PurchaseId;
use scale_info::TypeInfo;
use sp_core::hexdisplay::HexDisplay;
use std::fmt::Formatter;

//TODO
pub type MaximumPurchaseLength = ConstU32<200>;
pub type MaximumURLLength = ConstU32<200>;
pub type MaximumMapCapacity = ConstU32<200>;
pub type MaximumPriceKeyList = ConstU32<500>;
pub type MaximumErrorTip = ConstU32<100>;
pub type MaximumJumpBlockList = ConstU32<500>;
pub type MaximumSubPriceList = ConstU32<500>;
pub type MaximumAuthorities = ConstU32<500>;
pub type MaximumPricesRequestList = ConstU32<500>;

pub type FractionLength = u32;
pub type RequestInterval = u8;
pub type OffchainSignature<T> = <T as SigningTypes>::Signature;

// pub type PriceKey = BoundedVec<u8, ares_oracle_provider_support::SymbolKeyLimit>;
// pub type PriceToken = BoundedVec<u8, ares_oracle_provider_support::SymbolKeyLimit>;
// pub type RawSourceKeys = BoundedVec<(PriceKey, PriceToken, FractionLength), MaximumPriceKeyList>;
// pub type RequestKeys = BoundedVec<PriceKey, MaximumPriceKeyList>;
// pub type PricePayloadSubPriceList = BoundedVec<PricePayloadSubPrice, MaximumSubPriceList>;
// pub type PricePayloadSubJumpBlockList = BoundedVec<PricePayloadSubJumpBlock, MaximumSubPriceList>;

pub type PriceKey = Vec<u8>;
pub type PriceToken = Vec<u8>;
pub type RawSourceKeys = Vec<(PriceKey, PriceToken, FractionLength)>;
pub type RequestKeys = Vec<PriceKey>;
pub type PricePayloadSubPriceList = Vec<PricePayloadSubPrice>;
pub type PricePayloadSubJumpBlockList = Vec<PricePayloadSubJumpBlock>;


#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct OcwControlData {
	pub need_verifier_check: bool,
	pub open_free_price_reporter: bool,
	pub open_paid_price_reporter: bool,
}

impl Default for OcwControlData {
	fn default() -> Self {
		Self {
			need_verifier_check: true,
			open_free_price_reporter: true,
			open_paid_price_reporter: true,
		}
	}
}

// Migration
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PurchasedDefaultData {
	pub submit_threshold: u8,
	pub max_duration: u64,
	pub avg_keep_duration: u64,
	/* TODO:: Will be delete.
	 * pub unit_price: u64, */
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

impl Default for PurchasedDefaultData {
	fn default() -> Self {
		Self {
			submit_threshold: 60,
			max_duration: 20,
			avg_keep_duration: 14400,
			// unit_price: 100_000_000_000_000,
		}
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo)]
pub struct PurchasedSourceRawKeys {
	pub purchase_id: PurchaseId,
	pub raw_source_keys: RawSourceKeys,
}

impl Default for PurchasedSourceRawKeys {
	fn default() -> Self {
		Self {
			purchase_id: Vec::default(),
			raw_source_keys: Vec::default(),
		}
	}
}

// Impl debug.
impl fmt::Debug for PurchasedSourceRawKeys {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let raw_keys: Vec<_> = self
			.raw_source_keys
			.iter()
			.map(|(price_key, parse_key, fraction_len)| {
				(
					str::from_utf8(price_key).unwrap(),
					str::from_utf8(parse_key).unwrap(),
					fraction_len,
				)
			})
			.collect();
		write!(
			f,
			"{{( purchase_id: {:?}, raw_source_keys: {:?} )}}",
			HexDisplay::from(&self.purchase_id.as_ref()),
			// str::from_utf8(&self.0).map_err(|_| fmt::Error)?,
			raw_keys,
		)
	}
}

//TODO debug fmt requestKeys
#[derive(Default, Encode, Decode, RuntimeDebug, Clone, PartialEq, Eq, TypeInfo)]
pub struct PurchasedRequestData<AccountId, Balance, BlockNumber> {
	pub account_id: AccountId,
	pub offer: Balance,
	pub create_bn: BlockNumber,
	pub submit_threshold: u8,
	pub max_duration: u64,
	pub request_keys: RequestKeys,
}

// impl<T: Config> Default for PurchasedRequestData<T> {
// 	fn default() -> Self {
// 		Self {
// 			account_id: Default::default(),
// 			offer: 0u32.into(),
// 			create_bn: Default::default(),
// 			submit_threshold: 0,
// 			max_duration: 0,
// 			request_keys: BoundedVec::default(),
// 		}
// 	}
// }

// Impl debug.
// impl<T: Config> fmt::Debug for PurchasedRequestData<T> {
// 	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
// 		let request_keys: Vec<_> = self.request_keys.iter().map(|x|
// str::from_utf8(x).unwrap()).collect(); 		write!(
// 			f,
// 			"{{( account_id: {:?}, offer: {:?}, submit_threshold: {:?}, max_duration: {:?}, request_keys:
// {:?} )}}", 			&self.account_id, &self.offer, &self.submit_threshold, &self.max_duration,
// request_keys, 		)
// 	}
// }

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PurchasedAvgPriceData {
	pub create_bn: u64,
	pub reached_type: u8,
	pub price_data: (u64, FractionLength),
}

impl Default for PurchasedAvgPriceData {
	fn default() -> Self {
		Self {
			create_bn: 0,
			reached_type: 0,
			price_data: (0, 0),
		}
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct AresPriceData<AccountId, BlockNumber> {
	pub price: u64,
	pub account_id: AccountId,
	pub create_bn: BlockNumber,
	pub fraction_len: FractionLength,
	pub raw_number: JsonNumberValue,
	pub timestamp: u64,
}

// migrated
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct AvgPriceData {
	pub integer: u64,
	pub fraction_len: FractionLength,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PreCheckCompareLog {
	// pub chain_avg_price_list: BoundedBTreeMap<PriceKey, (u64, FractionLength), MaximumMapCapacity>,
	// pub validator_up_price_list: BoundedBTreeMap<PriceKey, (u64, FractionLength), MaximumMapCapacity>,
	// pub raw_precheck_list: PreCheckList,
	pub chain_avg_price_list: BTreeMap<PriceKey, (u64, FractionLength)>,
	pub validator_up_price_list: BTreeMap<PriceKey, (u64, FractionLength)>,
	pub raw_precheck_list: PreCheckList,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct HttpErrTraceData<BlockNumber, AuthorityId> {
	pub block_number: BlockNumber,
	// pub request_list: Vec<(Vec<u8>, Vec<u8>, u32)>,
	pub err_auth: AuthorityId,
	pub err_status: HttpError,
	pub tip: Vec<u8>,
}

/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PricePayload<Public, BlockNumber, AuthorityId> {
	pub block_number: BlockNumber,
	// price_key,price_val, fraction len
	pub price: PricePayloadSubPriceList,
	pub jump_block: PricePayloadSubJumpBlockList,
	pub auth: AuthorityId,
	pub public: Public,
}

impl<T: SigningTypes + Config> SignedPayload<T> for PricePayload<T::Public, T::BlockNumber, T::AuthorityAres> {
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PurchasedForceCleanPayload<Public, BlockNumber, AuthorityId> {
	pub block_number: BlockNumber,
	pub purchase_id_list: Vec<PurchaseId>,
	pub auth: AuthorityId,
	pub public: Public,
}

impl<T: SigningTypes + Config> SignedPayload<T>
	for PurchasedForceCleanPayload<T::Public, T::BlockNumber, T::AuthorityAres>
{
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PurchasedPricePayload<Public, BlockNumber, AuthorityId> {
	pub block_number: BlockNumber,
	pub purchase_id: PurchaseId,
	pub price: PricePayloadSubPriceList,
	pub auth: AuthorityId,
	pub public: Public,
}

impl<T: SigningTypes + Config> SignedPayload<T> for PurchasedPricePayload<T::Public, T::BlockNumber, T::AuthorityAres> {
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

/// stash: T::AccountId, auth: T::AuthorityAres, bn: T::BlockNumber
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PreCheckPayload<Public, BlockNumber, AccountId, AuthorityId> {
	pub block_number: BlockNumber,
	pub pre_check_stash: AccountId,
	pub pre_check_auth: AuthorityId,
	pub auth: AuthorityId,
	pub public: Public,
}

impl<T: SigningTypes + Config> SignedPayload<T>
	for PreCheckPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>
{
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PreCheckResultPayload<Public, BlockNumber, AccountId, AuthorityId> {
	pub block_number: BlockNumber,
	pub pre_check_list: PreCheckList,
	pub pre_check_stash: AccountId,
	pub pre_check_auth: AuthorityId,
	pub public: Public,
}

impl<T: SigningTypes + Config> SignedPayload<T>
	for PreCheckResultPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>
{
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct HttpErrTracePayload<Public, BlockNumber, AuthorityId, AccountId> {
	pub trace_data: HttpErrTraceData<BlockNumber, AccountId>,
	pub auth: AuthorityId,
	pub public: Public,
}

impl<T: SigningTypes + Config> SignedPayload<T>
	for HttpErrTracePayload<T::Public, T::BlockNumber, T::AuthorityAres, T::AccountId>
{
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PricePayloadSubPrice(pub PriceKey, pub u64, pub FractionLength, pub JsonNumberValue, pub u64);

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo)]
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum HttpError {
	IoErr(Vec<u8>),
	TimeOut(Vec<u8>),
	StatusErr(Vec<u8>, u16),
	ParseErr(Vec<u8>),
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo)]
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
