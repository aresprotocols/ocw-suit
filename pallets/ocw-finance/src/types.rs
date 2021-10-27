use super::*;

pub type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
pub type PurchaseId = Vec<u8>;