use super::*;

impl<T: Config> Pallet<T> {
    /// Use to filter out those `format_data` of price that need to jump block.
    pub fn filter_jump_block_data (
        format_data: Vec<(PriceKey, PriceToken, FractionLength, RequestInterval)>,
        account: T::AccountId,
        _block_number: T::BlockNumber,
    ) -> (RawSourceKeys, PricePayloadSubJumpBlockList) {
        // isNeedUpdateJumpBlock
        let mut new_format_data: RawSourceKeys = Default::default();
        let mut jump_format_data: PricePayloadSubJumpBlockList = Default::default();

        format_data
            .iter()
            .any(|(price_key, price_token, fraction_length, request_interval)| {
                if Self::is_need_update_jump_block(price_key.clone(), account.clone()) {
                    let _res = jump_format_data.try_push(PricePayloadSubJumpBlock(price_key.clone(), *request_interval));
                } else {
                    let _res = new_format_data.try_push((price_key.clone(), price_token.clone(), *fraction_length));
                }
                false
            });
        (new_format_data, jump_format_data)
    }

    /// Judge the author who submitted the price last time, and return true if it is consistent with this time.
    pub fn is_need_update_jump_block(price_key: PriceKey, account: T::AccountId) -> bool {
        if !Self::is_aura() || 1 == T::AuthorityCount::get_validators_count() {
            return false;
        }
        match Self::get_last_price_author(price_key) {
            None => false,
            Some(x) => x.0 == account,
        }
    }

    /// Calculate jump blocks
    pub fn handle_calculate_jump_block(jump_format: (u64, u64)) -> (u64, u64) {
        let (interval, jump_block) = jump_format;
        assert!(interval > 0, "The minimum interval value is 1");

        let new_jump_block = match jump_block.checked_sub(1) {
            None => interval.saturating_sub(1),
            Some(_x) => jump_block.saturating_sub(1),
        };
        (interval, new_jump_block)
    }

    /// When in the Aura consensus system, in order to avoid a certain `trading-pairs`
    /// being repeatedly `submit` by a certain validator, the `jump block` mechanism appears.
    ///
    /// This method can change the execution interval of `submit` to avoid this problem.
    pub(crate) fn increase_jump_block_number(price_key: PriceKey, interval: u64) -> (u64, u64) {
        let (old_interval, jump_number) =
            Self::handle_calculate_jump_block((interval, Self::get_jump_block_number(price_key.clone())));
        <JumpBlockNumber<T>>::insert(&price_key, jump_number.clone());
        (old_interval, jump_number)
    }

    /// Get the number of `jump block` associated with `trading-pairs`.
    pub fn get_jump_block_number(price_key: PriceKey) -> u64 {
        <JumpBlockNumber<T>>::get(&price_key).unwrap_or(0)
    }

    /// Delete jump block data
    pub(crate) fn remove_jump_block_number(price_key: PriceKey) {
        <JumpBlockNumber<T>>::remove(&price_key);
    }
}