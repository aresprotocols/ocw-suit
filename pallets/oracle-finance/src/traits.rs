use super::*;
// use types::*;

pub trait IForBase<T: Config> {
	//
	fn current_era_num() -> EraIndex;
	//
	fn get_earliest_reward_era() -> Option<EraIndex>;
}

pub trait IForPrice<T: Config>: IForBase<T> {
	// Calculate purchased price request fee.
	fn calculate_fee_of_ask_quantity(price_count: u32) -> BalanceOf<T> ;

	// Pay purchased price request fee.
	fn reserve_for_ask_quantity(who: T::AccountId, p_id: PurchaseId, price_count: u32) -> OcwPaymentResult<BalanceOf<T>> ;

	// Refund fee of purchased price reqest.
	fn unreserve_ask (p_id: PurchaseId) -> Result<(), Error<T>>;

	fn pay_to_ask(p_id: PurchaseId) -> Result<(), Error<T>>;
}

pub trait IForReporter<T: Config>: IForBase<T> {
	//
	fn record_submit_point (who: T::AccountId, p_id: PurchaseId, bn: T::BlockNumber, ask_point: AskPointNum) -> Result<(), Error<T>>;
	//
	fn get_record_point(ask_era: u32, who: T::AccountId, p_id: PurchaseId) -> Option<AskPointNum>;
}

pub trait IForReward<T: Config>: IForBase<T> {
	//
	fn take_reward(ask_era: EraIndex, who: T::AccountId) -> Result<BalanceOf<T>, Error<T>>;

	//
	fn get_era_income(ask_era: EraIndex) -> BalanceOf<T>;

	//
	fn get_era_point(ask_era: EraIndex) -> AskPointNum;

	//
	fn check_and_slash_expired_rewards(ask_era: EraIndex) -> Option<BalanceOf<T>>;
}
