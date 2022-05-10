use super::*;

/// Get basic data related to Era
pub trait IForBase<T: Config> {
	/// To get the current ear, you need to consider that if the era-length changes,
	/// you still need to guarantee that the vector of time zones increases.
	fn current_era_num() -> EraIndex;

	/// Get the earliest reward era.
	fn get_earliest_reward_era() -> Option<EraIndex>;
}

/// Traits for service fee management
pub trait IForPrice<T: Config>: IForBase<T> {

	/// Input in a price_count to calculate the cost of the purchase.
	///
	/// - price_count: Expected count of aggregate `trade-paris`.
	fn calculate_fee_of_ask_quantity(price_count: u32) -> BalanceOf<T> ;

	/// Keep the balance for the purchase.
	///
	/// - who: Origin account id.
	/// - p_id: Purchase order id.
	/// - price_count: Expected count of aggregate `trade-paris`.
	fn reserve_for_ask_quantity(who: T::AccountId, p_id: PurchaseId, price_count: u32) -> OcwPaymentResult<BalanceOf<T>, PurchaseId> ;

	/// Release the funds reserved for the purchase, which occurs after the failure of the ask-price.
	///
	/// - p_id: Purchase order id.
	fn unreserve_ask (p_id: PurchaseId) -> Result<(), Error<T>>;

	/// Execute the payment, which will transfer the user's `balance` to the Pallet
	///
	/// - p_id: Purchase order id.
	/// - price_count: The count of actual aggregate `trade-paris`
	fn pay_to_ask(p_id: PurchaseId, agg_count: usize) -> Result<(), Error<T>>;
}

/// Traits used to support Piont records, points are used to calculate validator rewards.
pub trait IForReporter<T: Config>: IForBase<T> {

	/// Record the Points of the validator under an order
	///
	/// - who: A validator id.
	/// - p_id: Purchase order id.
	/// - bn: The corresponding block when the order is generated.
	/// - ask_point: A number of u32
	fn record_submit_point (who: T::AccountId, p_id: PurchaseId, bn: T::BlockNumber, ask_point: AskPointNum) -> Result<(), Error<T>>;

	/// Get the Point of the record
	///
	/// - ask_era： Era of the reward.
	/// - who: A validator id.
	/// - p_id: Purchase order id.
	fn get_record_point(ask_era: u32, who: T::AccountId, p_id: PurchaseId) -> Option<AskPointNum>;
}

/// Traits for reward claiming and management.
pub trait IForReward<T: Config>: IForBase<T> {
	/// Claim all rewards under a given era
	///
	/// - ask_era： Era of the reward.
	/// - who: A validator id.
	fn take_reward(ask_era: EraIndex, who: T::AccountId) -> Result<BalanceOf<T>, Error<T>>;

	/// Get total income balance for an era.
	///
	/// - ask_era： Era of the reward.
	fn get_era_income(ask_era: EraIndex) -> BalanceOf<T>;

	/// Read the total balance for an era.
	///
	/// - ask_era： Era of the reward.
	fn get_era_point(ask_era: EraIndex) -> AskPointNum;

	/// Check for expired rewards and destroy them if overdue
	fn check_and_slash_expired_rewards(ask_era: EraIndex) -> Option<BalanceOf<T>>;
}
