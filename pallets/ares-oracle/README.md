## Workflow ##

### Validator Audit
1. Ares-oracle obtains the newly elected validator through the `IStakingNpos::pending_npos` trait,
   and submits the `pre-check` task to the chain through `submit_create_pre_check_task` extrinsics.
2. Use the `IStakingNpos::near_era_change` trait to determine whether the session period close to the election has been reached, and if so, submit the new verification task to the chain through `submit_offchain_pre_check_result` extrinsics.
3. Validators that do not pass the review will not appear in the list of targets for elections.

### Free Trade-price
1. When the offchain is working, the block author of the current block will be obtained according to the data provided by `authorship`.
2. If the "authority-id" corresponding to the block author is consistent with the local "keystore",
   obtain the list of "transaction pairs" corresponding to the block, and send an http request to obtain its price,
   then call the `submit_price_unsigned_with_signed_payload`  extrinsics upload the result to the chain .
3. The chain will verify whether the submission matches the block author of the current block, and if so, the result will be stored in the `price` pool.
4. When the `price` pool reaches the specified depth (this depth can be modified by the extrinsics `update_pool_depth_propose`),
   the average price aggregation event `Event::AggregatedPrice` occurs, which will generate an average price to the chain.
5. The price related to a `trade pairs` can be read through the `aresOracle.aresAvgPrice store`.

### Paid Trade-price
1. Users can submit a `submit_ask_price` transaction to have all nodes on the chain make an offer,
   but this requires payment some fee, which is more time-sensitive than getting the price for free.
2. If submit a task successfully, you need to get the `purchase_id` from the `Event::NewPurchasedRequest` event,
   which will be used as the associated key value for other queries. The amount of payment is related to the number
   of `trade pairs` requests. If the nodes participating in the response are less than the response threshold,
   the task will fail, and the fee will not be deducted. (This threshold can be modified by extrinsics `update_purchased_param`).
3. Once the price are aggregated successfully, an `Event::PurchasedAvgPrice` event will be generated,
   and the relevant fees will be deducted from the `origin` account. Users can also read the corresponding result data through the `aresOracle.purchasedAvgPrice` store.
   
### Block Author Submission Rules
1. When `offchain_worker` is executed, it will get the author of the current block, and if its corresponding `ares-authority` exists in the local `keystore`, 
   it will get the right to submit the price.
2. After the price is submitted, the `validate_unsigned` method will confirm that the author is in the validator set.
3. Complete the third validation through `AresOracleFilter` in `apply_extrinsic` and `validate_transaction` of Runtime. 
   This is very important, without this layer of checking it may generate redundant commits during forks or cannot avoid old block attacks.
   
