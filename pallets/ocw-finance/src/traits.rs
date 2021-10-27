use super::*;
use types::*;

// pub trait IOcwBaseParam {
// 	fn get_
// }

pub trait IForPrice<T: Config> {
	// Calculate purchased price request fee.
	fn calculate_fee_of_ask_quantity(price_count: u32) -> BalanceOf<T> ;

	// Pay purchased price request fee.
	fn payment_for_ask_quantity(who: T::AccountId, p_id: PurchaseId, price_count: u32) -> OcwPaymentResult<T> ;

	// Refund fee of purchased price reqest.
	fn refund_ask_paid (p_id: PurchaseId) -> bool;
}

pub enum OcwPaymentResult<T: Config> {
	InsufficientBalance(PurchaseId,BalanceOf<T>),
	Success(PurchaseId,BalanceOf<T>)
}