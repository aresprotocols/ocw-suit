use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::storage::bounded_btree_map::BoundedBTreeMap;
use frame_support::traits::{ConstU32};
use frame_support::BoundedVec;
use oracle_finance::types::PurchaseId;
use scale_info::TypeInfo;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::Zero;
// use sp_std::fmt::Debug;
use ares_oracle_provider_support::{MaximumPoolSize, PriceKey, RawSourceKeys, RequestKeys};

pub type FractionLength = u32;
pub type RequestInterval = u8;
pub type OffchainSignature<T> = <T as SigningTypes>::Signature;

// pub type PriceKey = Vec<u8>;
// pub type PriceToken = Vec<u8>;
// pub type RawSourceKeys = Vec<(PriceKey, PriceToken, FractionLength)>;
// pub type RequestKeys = Vec<PriceKey>;
pub type PricePayloadSubPriceList = BoundedVec<PricePayloadSubPrice, MaximumPoolSize>;
pub type PricePayloadSubJumpBlockList = BoundedVec<PricePayloadSubJumpBlock, MaximumPoolSize>;

/// Parameter group that controls the execution of ares-oracle
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct OcwControlData {
	/// Whether to start the validator checker,
	/// if not, the block producer will not be verified,
	/// this is only used in the debugging phase
	pub need_verifier_check: bool,
	/// Whether the `free-price` moudle is enabled
	pub open_free_price_reporter: bool,
	/// Whether the `ask-price` moudle is enabled
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


#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PurchasedDefaultData {
	pub submit_threshold: Percent,
	pub max_duration: u64,
	pub avg_keep_duration: u64,
}

impl PurchasedDefaultData {
	pub fn new(submit_threshold: Percent, max_duration: u64, avg_keep_duration: u64) -> Self {
		if submit_threshold == Zero::zero() {
			panic!("Submit Threshold range is (0 - 100] ");
		}
		if max_duration == 0 {
			panic!("Max Duration can not be 0.");
		}
		Self {
			submit_threshold,
			max_duration,
			avg_keep_duration,
		}
	}
}

// #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
// pub struct PurchasedDefaultData {
// 	pub submit_threshold: Percent,
// 	pub max_duration: u64,
// 	pub avg_keep_duration: u64,
// }

// impl PurchasedDefaultData {
// 	pub fn new(submit_threshold: Percent, max_duration: u64, avg_keep_duration: u64) -> Self {
// 		if submit_threshold == Zero::zero() {
// 			panic!("Submit Threshold range is (0 - 100] ");
// 		}
// 		if max_duration == 0 {
// 			panic!("Max Duration can not be 0.");
// 		}
// 		Self {
// 			submit_threshold,
// 			max_duration,
// 			avg_keep_duration,
// 		}
// 	}
// }

impl Default for PurchasedDefaultData {
	fn default() -> Self {
		Self {
			// submit_threshold: 60,
			submit_threshold: Percent::from_percent(60),
			max_duration: 20,
			avg_keep_duration: 14400,
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
			purchase_id: BoundedVec::default(),
			raw_source_keys: Default::default(),
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
#[derive(Encode, Decode, RuntimeDebug, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
pub struct PurchasedRequestData<AccountId, Balance, BlockNumber> {
	pub account_id: AccountId,
	pub offer: Balance,
	pub create_bn: BlockNumber,
	// pub submit_threshold: u8,
	pub submit_threshold: Percent,
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AresPriceData<AccountId, BlockNumber> {
	pub price: u64,
	pub account_id: AccountId,
	pub create_bn: BlockNumber,
	pub fraction_len: FractionLength,
	pub raw_number: JsonNumberValue,
	pub timestamp: u64,
	pub update_bn: BlockNumber,
}

// migrated
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AvgPriceData {
	pub integer: u64,
	pub fraction_len: FractionLength,
}

pub type MaximumLogSize = ConstU32<5000>;
pub type CompareLogBTreeMap = BoundedBTreeMap<PriceKey, (u64, FractionLength), MaximumLogSize>;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PreCheckCompareLog {
	pub chain_avg_price_list: CompareLogBTreeMap,
	pub validator_up_price_list: CompareLogBTreeMap,
	pub raw_precheck_list: PreCheckList,
	// pub chain_avg_price_list: BTreeMap<PriceKey, (u64, FractionLength)>,
	// pub validator_up_price_list: BTreeMap<PriceKey, (u64, FractionLength)>,
	// pub raw_precheck_list: PreCheckList,
}

pub type MaximumTraceDataTipLength = ConstU32<200>;
pub type DataTipVec = BoundedVec<u8, MaximumTraceDataTipLength> ;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct HttpErrTraceData<BlockNumber, AuthorityId> {
	pub block_number: BlockNumber,
	// pub request_list: Vec<(Vec<u8>, Vec<u8>, u32)>,
	pub err_auth: AuthorityId,
	pub err_status: HttpError,
	pub tip: DataTipVec,
}

/// data required to submit a transaction.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PreCheckResultPayload<Public, BlockNumber, AccountId, AuthorityId> {
	pub block_number: BlockNumber,
	pub pre_check_list: PreCheckList,
	pub pre_check_stash: AccountId,
	pub pre_check_auth: AuthorityId,
	pub task_at: BlockNumber,
	pub public: Public,
}

impl<T: SigningTypes + Config> SignedPayload<T>
	for PreCheckResultPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>
{
	fn public(&self) -> T::Public {
		self.public.clone()
	}
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PricePayloadSubPrice(pub PriceKey, pub u64, pub FractionLength, pub JsonNumberValue, pub u64);

#[derive(Encode, Decode, Clone, PartialEq, Eq, TypeInfo, MaxEncodedLen)]
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum HttpError {
	IoErr(DataTipVec),
	TimeOut(DataTipVec),
	StatusErr(DataTipVec, u16),
	ParseErr(DataTipVec),
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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

pub type AresPriceDataVec<AccountId, BlockNumber> = BoundedVec<AresPriceData<AccountId, BlockNumber>, MaximumPoolSize>;

// validator,
// create_bn,
// price_key,
// retry_count,
// price_pool,
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct AggregatedMission< BlockNumber, AccountId> {
	// pub validator: AuthorityId,
	pub create_bn: BlockNumber,
	pub price_key: PriceKey,
	pub retry_count: u32,
	pub price_pool: AresPriceDataVec<AccountId, BlockNumber>,
}

// pub trait ToBoundVec<B, S, T> {
// 	fn to_bound_vec(self) -> BoundedVec<T, S>;
// }
//
// impl <B, S, T> ToBoundVec<B, S, T> for Vec<T>
// 	where B: IsType<BoundedVec<T, S>> + TryFrom<Vec<T>> + Default,
// {
//
// 	fn to_bound_vec(self) -> BoundedVec<T, S> {
// 		B::try_from(self).unwrap_or(B::default()).into()
// 	}
// }

//
// pub trait BoundVecHelper<T, S> {
// 	type Error;
// 	fn create_on_vec(v: Vec<T>) -> Self;
// 	fn check_push(&mut self, v: T) ;
// 	fn try_create_on_vec(v: Vec<T>) -> Result<Self, Self::Error> where Self: Sized;
// }
//
// impl <T, S> BoundVecHelper<T, S> for BoundedVec<T, S>
// 	where
// 		  S: Get<u32>,
// 		  BoundedVec<T, S>: Debug,
// 		  BoundedVec<T, S>: TryFrom<Vec<T>> + Default,
// 		  <BoundedVec<T, S> as TryFrom<Vec<T>>>::Error: Debug,
// {
// 	type Error = <Self as TryFrom<Vec<T>>>::Error;
// 	fn create_on_vec(v: Vec<T>) -> Self {
// 		Self::try_from(v).expect("`BoundedVec` MaxEncodedLen Err")
// 	}
// 	fn try_create_on_vec(v: Vec<T>) -> Result<Self, Self::Error> where Self: Sized {
// 		Self::try_from(v)
// 	}
// 	fn check_push(&mut self, v: T) {
// 		self.try_push(v).expect("`BoundedVec` try push err.");
// 	}
// }