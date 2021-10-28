use super::*;
use types::*;

pub trait IForBase<T: Config> {
	//
	fn make_period_num(bn: T::BlockNumber) -> AskPeriodNum;
}

pub trait IForPrice<T: Config>: IForBase<T> {
	// Calculate purchased price request fee.
	fn calculate_fee_of_ask_quantity(price_count: u32) -> BalanceOf<T> ;

	// Pay purchased price request fee.
	fn payment_for_ask_quantity(who: T::AccountId, p_id: PurchaseId, price_count: u32) -> OcwPaymentResult<T> ;

	// Refund fee of purchased price reqest.
	fn refund_ask_paid (p_id: PurchaseId) -> Result<(), Error<T>>;
}

pub trait IForReporter<T: Config>: IForBase<T> {
	//
	fn record_submit_point (who: T::AccountId, p_id: PurchaseId, bn: T::BlockNumber, ask_point: u64) -> Result<(), Error<T>>;
}

pub trait IForReward<T: Config>: IForBase<T> {
	//
	fn take_reward(ask_period: AskPeriodNum, who: T::AccountId) -> Result<(), Error<T>>;

	//
	fn get_earliest_reward_period(bn: T::BlockNumber) -> AskPeriodNum;

	//
	fn get_sum_of_record_point(ask_period: AskPeriodNum) -> AskPointNum;

	//
	fn check_and_slash_expired_rewards(ask_period: AskPeriodNum) -> Option<BalanceOf<T>>;
}
