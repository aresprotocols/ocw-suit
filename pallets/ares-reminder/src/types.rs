use super::*;
use frame_support::BoundedVec;
use frame_support::traits::{ConstU32, Currency};
use sp_runtime::{RuntimeDebug};
use scale_info::TypeInfo;
use sp_std::vec::Vec;
use codec::{ Decode, Encode, MaxEncodedLen};
use frame_system::offchain::{SignedPayload, SigningTypes};

#[cfg(feature = "std")]
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

pub type ReminderIden = u64;
pub type RepeatCount = u32;
pub type MaximumReminderListSize = ConstU32<10000>;
pub type ReminderIdenList = BoundedVec<ReminderIden, MaximumReminderListSize>; // Vec<u8>;
pub type MaximumUrlLenthSize = ConstU32<500>;
pub type ReminderCallBackUrl = BoundedVec<u8, MaximumUrlLenthSize>;
pub type MaximumUrlSignSize = ConstU32<100>;
pub type ReminderCallBackSign = BoundedVec<u8, MaximumUrlSignSize>;
pub type MaximumTimSignSize = ConstU32<100>;
pub type ReminderTriggerTip = BoundedVec<u8, MaximumTimSignSize>;
pub type MaximumSendList = ConstU32<5000>;
pub type ReminderSendList<RID, BN> = BoundedVec<(RID, BN), MaximumSendList>;

pub type OffchainSignature<T> = <T as SigningTypes>::Signature;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PriceTrigger<Account, Price, BlockNumber, RepeatCount, Condition, Receiver> {
    pub owner: Account,
    // // used to mark a key-pair.
    // pub price_key: Symbol,
    // Reminder interval.
    pub interval_bn: BlockNumber,
    // Reminder repetitions.
    pub repeat_count: RepeatCount,
    // // Anchored price, i.e. trigger check price.
    // pub anchor_price: Price,
    // Trigger create block number.
    pub create_bn: BlockNumber,
    // Record the last executed block, if the trigger runs.
    // pub update_bn: BlockNumber,
    // The price snapshot at the time of the last update (important data to later judge whether to rise or fall)
    pub price_snapshot: Price,
    //
    pub last_check_infos: Option<(Price, BlockNumber)>,
    //
    pub trigger_condition: Condition,
    //
    pub trigger_receiver: Receiver,
    //
    pub update_bn: BlockNumber,
    //
    pub tip: Option<ReminderTriggerTip>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ReminderCondition <Symbol, Price> {
    TargetPriceModel {
        price_key: Symbol,
        anchor_price: Price
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ReminderReceiver {
    HttpCallBack {
        url: ReminderCallBackUrl,
        sign: ReminderCallBackSign
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct CompletePayload<RID, ResponseMark, ResponseStatus, AuthorityId, Public, Bn> {
    pub reminder: (RID, Bn),
    pub response_mark: Option<ResponseMark>,
    pub status: Option<ResponseStatus>,
    pub auth: AuthorityId,
    pub public: Public,
}




