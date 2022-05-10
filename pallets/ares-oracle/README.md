## Tx method ##
### submit_local_xray
An offline method that submits and saves the local publicly data by the validator node to the chain for debugging.

The dispatch origin fo this call must be none.

```text
host_key: A random u32 for a node
request_domain: The warehouse parameter currently set by the node.
authority_list: List of ares-authority public keys stored locally
network_is_validator: Whether the node validator
```
* Fields of submit_local_xray
```text
host_key: u32
request_domain: RequestBaseVecU8
authority_list: AuthorityAresVec<T>
network_is_validator: bool
```

### submit_ask_price

Submit an ares-price request, The request is processed by all online validators, and the aggregated result is returned immediately.

The dispatch origin for this call must be Signed.
```text
max_fee: The highest asking fee accepted by the signer.
request_keys: A list of Trading pairs, separated by commas if multiple, such as: eth-usdt, dot-sudt, etc.
```
* Fields of submit_ask_price
```text
max_fee: BalanceOf<T>
request_keys: Vec<u8>
```

### submit_forced_clear_purchased_price_payload_signed
An offline method that submits and saves the purchase-result data. If the count of validator submitted by purchase-id already reached the threshold requirements, then average price will be aggregation and mark PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE

The dispatch origin fo this call must be none.

```text
– price_payload: Submitted data.
```

* Fields of submit_forced_clear_purchased_price_payload_signed
```text
price_payload: PurchasedForceCleanPayload<T::Public, T::BlockNumber, T::AuthorityAres>
signature: OffchainSignature<T>
```

### submit_purchased_price_unsigned_with_signed_payload
An offline method that submits and saves the purchase-result data. If all validators submit price-result, then average price will be aggregation and mark PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE
The dispatch origin fo this call must be none.
```text
– price_payload: Submitted data.
```

* Fields of submit_purchased_price_unsigned_with_signed_payload
```text
price_payload: PurchasedPricePayload<T::Public, T::BlockNumber, T::AuthorityAres>
signature: OffchainSignature<T>
```

### submit_price_unsigned_with_signed_payload
An offline method that submits and saves the free ares-price results.

The dispatch origin fo this call must be none.
```text
price_payload: Ares-price data to be uploaded.
```
* Fields of submit_price_unsigned_with_signed_payload
```text
price_payload: PricePayload<T::Public, T::BlockNumber, T::AuthorityAres>
signature: OffchainSignature<T>
```

### submit_create_pre_check_task

Submit a pre-check task. When a new validator is elected, a pre_check_task task will be submitted through this method within a specific period.

This task is used to ensure that the ares-price response function of the validator node can be used normally.

The dispatch origin fo this call must be none.
```text
precheck_payload: Pre-Check task data, including validators and their authority account data.
```
* Fields of submit_create_pre_check_task
```text
precheck_payload: PreCheckPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>
signature: OffchainSignature<T>
```
### submit_offchain_pre_check_result
When the validator responds to the pre-check task, the pre-check result data is submitted to the chain. If approved, it will be passed in the next election cycle, not immediately.

The dispatch origin fo this call must be none.
```text
preresult_payload: Review response result data, which will be compared on-chain.
```
* Fields of submit_offchain_pre_check_result
```text
preresult_payload: PreCheckResultPayload<T::Public, T::BlockNumber, T::AccountId, T::AuthorityAres>
signature: OffchainSignature<T>
```

### submit_offchain_http_err_trace_result
When there is an error in the offchain http request, the error data will be submitted to the chain through this Call

The dispatch origin fo this call must be none.
```text
err_payload: Http err data.
```
* Fields of submit_offchain_http_err_trace_result
```text
err_payload: HttpErrTracePayload<T::Public, T::BlockNumber, T::AuthorityAres, T::AccountId>
signature: OffchainSignature<T>
```

### update_purchased_param
Updating the purchase-related parameter settings requires the Technical-Committee signature to execute.

The dispatch origin for this call must be Signed of Technical-Committee.
```text
submit_threshold: The threshold for aggregation is a percentage
max_duration: Maximum delay to wait for full node response.
avg_keep_duration: Maximum length to keep aggregated results.
```
* Fields of update_purchased_param
```text
submit_threshold: Percent
max_duration: u64
avg_keep_duration: u64
```

### update_ocw_control_setting
Update the control parameters of ares-oracle

The dispatch origin for this call must be Signed of Technical-Committee.

```text
need_verifier_check: Whether to start the validator checker.
open_free_price_reporter: Whether the free-price moudle is enabled.
open_paid_price_reporter: Whether the ask-price moudle is enabled.
```

* Fields of update_ocw_control_setting
```text
need_verifier_check: bool
open_free_price_reporter: bool
open_paid_price_reporter: bool
```

### revoke_update_request_propose
Revoke the key-pair on the request token list.

The dispatch origin for this call must be Signed of Technical-Committee.
```text
price_key: A price key, such as btc-usdt
```

* Fields of revoke_update_request_propose
```text
price_key: Vec<u8>
```
### update_request_propose
Modify or add a key-pair to the request list.

The dispatch origin for this call must be Signed of Technical-Committee.
```text
price_key: A price key, such as btc-usdt.
price_token: A price token, such as btc.
parse_version: Parse version, currently only parameter 2 is supported.
fraction_num: Fractions when parsing numbers.
request_interval: The interval between validators submitting price on chain.
```

* Fields of update_request_propose
```text
price_key: Vec<u8>
price_token: Vec<u8>
parse_version: u32
fraction_num: FractionLength
request_interval: RequestInterval
```

### update_allowable_offset_propose
Update the value of the allowable offset parameter to determine the abnormal range of the submitted price

The dispatch origin for this call must be Signed of Technical-Committee.
```text
offset: A Percent value.
```
* Fields of update_allowable_offset_propose
```text
offset: Percent
```

### update_pool_depth_propose
Update the depth of the price pool. When the price pool reaches the maximum value, the average price will be aggregated and put on the chain.

The dispatch origin for this call must be Signed of Technical-Committee.
```text
depth: u32 integer
```
* Fields of update_pool_depth_propose
```text
depth: u32
```

### update_pre_check_token_list
Update the pre-checked Trading pairs list for checking the validator price feature.

The dispatch origin for this call must be Signed of Technical-Committee.
```text
token_list: PriceToken
```

* Fields of update_pre_check_token_list
```text
token_list: TokenList
```

### update_pre_check_session_multi
session-multi indicates the trigger pre-check session period before the era.

The dispatch origin for this call must be Signed of Technical-Committee.

```text
multi: integer
```

* Fields of update_pre_check_session_multi
```text
multi: T::BlockNumber
```

### update_pre_check_allowable_offset
The maximum offset allowed for pre-check when validation.

The dispatch origin for this call must be Signed of Technical-Committee.

```text
offset: Percent
```

* Fields of update_pre_check_allowable_offset
```text
offset: Percent
```

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

### 出块人提交验证
1. When `offchain_worker` is executed, it will get the author of the current block, and if its corresponding `ares-authority` exists in the local `keystore`,
   it will get the right to submit the price.
2. After the price is submitted, the `validate_unsigned` method will confirm that the author is in the validator set.
3. Complete the third validation through `AresOracleFilter` in `apply_extrinsic` and `validate_transaction` of Runtime.
   This is very important, without this layer of checking it may generate redundant commits during forks or cannot avoid old block attacks.