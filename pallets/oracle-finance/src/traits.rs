#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::fmt::Formatter;
use codec::{Decode, Encode, EncodeLike, FullCodec, FullEncode, Input, MaxEncodedLen, WrapperTypeEncode};
use frame_support::traits::IsType;
use scale_info::TypeInfo;
use sp_runtime::BoundedVec;
use sp_std::convert::TryInto;
use sp_std::fmt::Debug;
use ares_oracle_provider_support::{PurchaseId, OrderIdEnum};
use super::*;

/// Get basic data related to Era
pub trait IForBase<T: Config<I>, I: 'static = ()> {
	/// To get the current ear, you need to consider that if the era-length changes,
	/// you still need to guarantee that the vector of time zones increases.
	fn current_era_num() -> EraIndex;

	/// Get the earliest reward era.
	fn get_earliest_reward_era() -> Option<EraIndex>;
}

/// Traits for service fee management
pub trait IForPrice<T: Config<I>, I: 'static = ()>: IForBase<T, I> {

	/// Input in a price_count to calculate the cost of the purchase.
	///
	/// - price_count: Expected count of aggregate `trade-paris`.
	fn calculate_fee(price_count: u32) -> BalanceOf<T, I> ;

	/// Keep the balance for the purchase.
	///
	/// - who: Origin account id.
	/// - p_id: Purchase order id.
	/// - price_count: Expected count of aggregate `trade-paris`.
	fn reserve_fee(who: &T::AccountId, p_id: &OrderIdEnum, price_count: u32) -> OcwPaymentResult<BalanceOf<T, I>, OrderIdEnum> ;

	/// Release the funds reserved for the purchase, which occurs after the failure of the ask-price.
	///
	/// - p_id: Purchase order id.
	fn unreserve_fee (p_id: &OrderIdEnum) -> Result<(), Error<T, I>>;

	/// Get reserve fee.
	///
	/// - p_id: Purchase order id.
	fn get_reserve_fee(p_id: &OrderIdEnum) -> BalanceOf<T, I>;

	/// Lock up deposits, for guarantees, etc, which will not be paid.
	/// - who: Origin account id.
	/// - p_id: Purchase order id.
	fn lock_deposit(who: &T::AccountId, pid: &OrderIdEnum, deposit: BalanceOf<T, I>) -> Result<(), Error<T, I>>;

	/// Unlock deposits.
	///
	/// - p_id: Purchase order id.
	fn unlock_deposit(pid: &OrderIdEnum) -> Result<(), Error<T, I>>;

	/// Unlock deposits.
	///
	/// - p_id: Purchase order id.
	fn get_locked_deposit(pid: &OrderIdEnum) -> BalanceOf<T, I>;

	/// Get all locked balance of user account.
	/// - who: Origin account id.
	fn get_all_locked_deposit_with_acc(who: &T::AccountId) -> BalanceOf<T, I>;

	/// Execute the payment, which will transfer the user's `balance` to the Pallet
	///
	/// - p_id: Purchase order id.
	/// - price_count: The count of actual aggregate `trade-paris`
	fn pay_to(p_id: &OrderIdEnum, agg_count: usize) -> Result<(), Error<T, I>>;
}

/// Traits used to support Piont records, points are used to calculate validator rewards.
pub trait IForReporter<T: Config<I>, I: 'static = ()>: IForBase<T, I> {

	/// Record the Points of the validator under an order
	///
	/// - who: A validator id.
	/// - p_id: Purchase order id.
	/// - bn: The corresponding block when the order is generated.
	/// - ask_point: A number of u32
	fn record_submit_point (who: &T::AccountId, p_id: &OrderIdEnum, bn: T::BlockNumber, ask_point: AskPointNum) -> Result<(), Error<T, I>>;

	/// Get the Point of the record
	///
	/// - ask_era： Era of the reward.
	/// - who: A validator id.
	/// - p_id: Purchase order id.
	fn get_record_point(ask_era: u32, who: &T::AccountId, p_id: &OrderIdEnum) -> Option<AskPointNum>;
}

/// Traits for reward claiming and management.
pub trait IForReward<T: Config<I>, I: 'static = ()>: IForBase<T, I> {
	/// Claim all rewards under a given era
	///
	/// - ask_era： Era of the reward.
	/// - who: A validator id.
	fn take_reward(ask_era: EraIndex, who: T::AccountId) -> Result<BalanceOf<T, I>, Error<T, I>>;

	/// Get total income balance for an era.
	///
	/// - ask_era： Era of the reward.
	fn get_era_income(ask_era: EraIndex) -> BalanceOf<T, I>;

	/// Read the total balance for an era.
	///
	/// - ask_era： Era of the reward.
	fn get_era_point(ask_era: EraIndex) -> AskPointNum;

	/// Check for expired rewards and destroy them if overdue
	fn check_and_slash_expired_rewards(ask_era: EraIndex) -> Option<BalanceOf<T, I>>;
}
