use super::*;
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::traits::ConstU32;
use frame_support::BoundedVec;
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;
pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type NegativeImbalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

// TODO
pub type MaximumIdLength = ConstU32<100>;
pub type MaximumRewardPeriods = ConstU32<100>;

// pub type PurchaseId = BoundedVec<u8, MaximumIdLength>;
pub type PurchaseId = Vec<u8>;

pub type AskPeriodNum = u64;

pub type AskPointNum = u32;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum OcwPaymentResult<Balance> {
	InsufficientBalance(PurchaseId, Balance),
	Success(PurchaseId, Balance),
}
// "OcwPaymentResult": {
// "_enum": ["InsufficientBalance(PurchaseId,BalanceOf)","Success(PurchaseId,BalanceOf)"]
// }

#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct PaidValue<BlockNumber, Balance> {
	pub create_bn: BlockNumber,
	pub amount: Balance,
	pub is_income: bool,
}