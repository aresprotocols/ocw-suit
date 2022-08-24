use super::*;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::ConstU32;
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
// use sp_std::vec::Vec;
use frame_support::{ BoundedVec};

pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type NegativeImbalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;

pub type MaximumPIDLength = ConstU32<100>;
pub type MaximumRewardEras = ConstU32<1000>;

// pub type PurchaseId = BoundedVec<u8, MaximumIdLength>;
// pub type PurchaseId = Vec<u8>;
pub type PurchaseId = BoundedVec<u8, MaximumPIDLength>;

pub type AskPointNum = u32;

pub type SessionIndex = u32;

pub type EraIndex = u32;

// #[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
// pub enum OcwPaymentResult<Balance> {
// 	InsufficientBalance(PurchaseId, Balance),
// 	Success(PurchaseId, Balance),
// }

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum OcwPaymentResult<Balance, PID> {
	InsufficientBalance(PID, Balance),
	Success(PID, Balance),
}

// "OcwPaymentResult": {
// "_enum": ["InsufficientBalance(PurchaseId,BalanceOf)","Success(PurchaseId,BalanceOf)"]
// }

#[derive(Default, Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct PaidValue<BlockNumber, Balance, EraIndexT> {
	pub create_bn: BlockNumber,
	pub paid_era: EraIndexT,
	pub amount: Balance,
	pub is_income: bool,
}