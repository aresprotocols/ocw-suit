
## Methods

### current_era_num
```rust
fn current_era_num() -> EraIndex
```
To get the current ear, you need to consider that if the era-length changes, you still need to guarantee that the vector of time zones increases.

### get_earliest_reward_era
```rust
fn get_earliest_reward_era() -> Option<EraIndex>
```
Get the earliest reward era.

### calculate_fee_of_ask_quantity
```rust
fn calculate_fee_of_ask_quantity(price_count: u32) -> BalanceOf<T>
```
Input in a price_count to calculate the cost of the purchase.

* params
```text
price_count: Expected count of aggregate trade-paris.
```

### reserve_for_ask_quantity
```rust
fn reserve_for_ask_quantity(
    who: T::AccountId,
    p_id: PurchaseId,
    price_count: u32
) -> OcwPaymentResult<BalanceOf<T>, PurchaseId>
```
Keep the balance for the purchase.
* params
```text
who: Origin account id.
p_id: Purchase order id.
price_count: Expected count of aggregate trade-paris.
```

### unreserve_ask
```rust
fn unreserve_ask(p_id: PurchaseId) -> Result<(), Error<T>>
```

Release the funds reserved for the purchase, which occurs after the failure of the ask-price.

```text
p_id: Purchase order id.
```

### pay_to_ask
```rust
fn pay_to_ask(p_id: PurchaseId, agg_count: usize) -> Result<(), Error<T>>
```

Execute the payment, which will transfer the user’s balance to the Pallet
```text
p_id: Purchase order id.
price_count: The count of actual aggregate trade-paris
```

### record_submit_point
```rust
fn record_submit_point(
    who: T::AccountId,
    p_id: PurchaseId,
    bn: T::BlockNumber,
    ask_point: AskPointNum
) -> Result<(), Error<T>>
```

Record the Points of the validator under an order

```text
who: A validator id.
p_id: Purchase order id.
bn: The corresponding block when the order is generated.
ask_point: A number of u32
```

### get_record_point
```rust
fn get_record_point(
    ask_era: u32,
    who: T::AccountId,
    p_id: PurchaseId
) -> Option<AskPointNum>
```

Get the Point of the record

```text
ask_era： Era of the reward.
who: A validator id.
p_id: Purchase order id.
```

### take_reward
```rust
fn take_reward(
    ask_era: EraIndex,
    who: T::AccountId
) -> Result<BalanceOf<T>, Error<T>>
```

Claim all rewards under a given era

```text
ask_era： Era of the reward.
who: A validator id.
```

### get_era_income
```rust
fn get_era_income(ask_era: EraIndex) -> BalanceOf<T>
```

Get total income balance for an era.

```text
ask_era： Era of the reward.
```

### get_era_point
```rust
fn get_era_point(ask_era: EraIndex) -> AskPointNum
```

Read the total balance for an era.

```text
ask_era： Era of the reward.
```

### check_and_slash_expired_rewards
```rust
fn check_and_slash_expired_rewards(ask_era: EraIndex) -> Option<BalanceOf<T>>
```
Check for expired rewards and destroy them if overdue

## Tx method ##

### take_purchase_reward

Validators get rewards corresponding to eras.

Note: An ear cannot be the current unfinished era, and rewards are not permanently stored. If the reward exceeds the depth defined by T::HistoryDepth, you will not be able to claim it.

The dispatch origin for this call must be Signed

Earliest reward Era = Current-Era - T::HistoryDepth

```text
ask_era: Era number is a u32
```

* Fields of take_purchase_reward
```text
ask_era: EraIndex
```

### take_all_purchase_reward

Validators get rewards, it will help validators get all available rewards

Note: An ear cannot be the current unfinished era, and rewards are not permanently stored. If the reward exceeds the depth defined by T::HistoryDepth, you will not be able to claim it.

The dispatch origin for this call must be Signed

Earliest reward Era = Current-Era - T::HistoryDepth


## Workflow


### Reward Deposit

1. Payment is through the `Trait` provided by `IForPrice`. It is necessary to call `reserve_for_ask_quantity`
   to reserve the part balance of asker and associate it with the order-id.

2. Through the Trait implementation provided by `IForReporter`, call `record_submit_point` to save the point.
   The block height needs to be passed in, and the pallet will convert it to the corresponding era and record it under an order-id.

3. After the price-response is successful, call `pay_to_ask` to release the Balance reserved above,
   and pay the actual ask fee to the oracle-finance pallet.

### Claim Reward

1. Rewards are claimed by era, and rewards cannot be obtained for unfinished eras. You need to calculate
   the 'Balance' corresponding to each point in the corresponding era. Algorithm: `Single Point Bonus` = `Total Revenue`/`Total Points` on era.
2. The validator needs to use the `Controller` account to perform the claim operation, This is for security reasons.
   The total amount received is equal to:  `Validator Points` * `Single Point Bonus`.
3. Rewards are transferred to the validator's Stash account via `oracle-finance` pallet.

