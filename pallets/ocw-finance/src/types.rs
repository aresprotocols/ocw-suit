use super::*;
use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;

pub type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
    <T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

pub type PurchaseId = Vec<u8>;

pub type AskPeriodNum = u64;

pub type AskPointNum = u32;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub enum OcwPaymentResult<T: Config> {
    InsufficientBalance(PurchaseId,BalanceOf<T>),
    Success(PurchaseId,BalanceOf<T>)
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct PaidValue<T: Config> {
    pub create_bn: T::BlockNumber,
    pub amount: BalanceOf<T>,
    pub is_income: bool,
}

impl <T: Config> Default for PaidValue<T>
{
    fn default() -> Self {
        Self {
            create_bn: Default::default(),
            amount: Default::default(),
            is_income: Default::default()
        }
    }
}
