# ocw-suit
## Project Overview

Ares Protocol is a decentralized cross-chain Oracle that truly implements a new generation of cross-chain Oracle service protocol for data on-chain verification, and open a compensation channel for data users who suffer business losses from using Oracle data.

Ares Protocol is a stochastic secure Oracle solution developed based on the Substrate framework, supporting hybrid Babe and off-chain workhorses, incorporating on-chain governance, bringing the Optimistic challenge model into the Oracle space, supporting the Ink contract ecosystem, and bringing better quality Oracle services to Defi developers.

### Project Details

The project structure consists of staking-extends that control Pallet-Staking, ares-oracle that controls quotation and audit work, oracle-finance that controls financial management, and their high-level abstraction layer oracle-provider-support.

From the perspective of validators, new nodes will have the opportunity to apply to become validators of ares-chain by staking on Pallet-Staking. The staking-extends module is responsible for capturing new validators, 
and ares-oracle is responsible for creating audit tasks for new validators. and ask these nodes to execute, If the node executes successfully, it means that the audit is passed, otherwise the staking-extend module will remove the failed ones and make it impossible to participate in the election normally.

After the audit, the passed nodes participate in the operation of ares-chain. In addition to enjoying the normal NPos rewards, they can also receive commissions for paid quotations. These proof-of-work and reward distribution are all done through the oracle-finance module.

![ares-component-workflow](https://raw.githubusercontent.com/aresprotocols/ocw-suit/master/docs/ares-component-workflow.png)

#### Pallet: ares-oracle
* submit_local_xray
  An offline method that submits and saves the local publicly data by the validator node to the chain for debugging.
- submit_ask_price
  Submit an ares-price request, The request is processed by all online validators, and the aggregated result is returned immediately.

* submit_forced_clear_purchased_price_payload_signed
  An offline method that submits and saves the purchase-result data. If the count of validator submitted by purchase-id already reached the threshold requirements, then average price will be aggregation and mark PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE

* submit_purchased_price_unsigned_with_signed_payload
  An offline method that submits and saves the purchase-result data. If all validators submit price-result, then average price will be aggregation and mark PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE

* submit_price_unsigned_with_signed_payload
  An offline method that submits and saves the free ares-price results.
* submit_create_pre_check_task
  Submit a pre-check task. When a new validator is elected, a pre_check_task task will be submitted through this method within a specific period.
  This task is used to ensure that the ares-price response function of the validator node can be used normally.
* submit_offchain_pre_check_result
  When the validator responds to the pre-check task, the pre-check result data is submitted to the chain. If approved, it will be passed in the next election cycle, not immediately.
* submit_offchain_http_err_trace_result
  When there is an error in the offchain http request, the error data will be submitted to the chain through this Call
* update_purchased_param
  Updating the purchase-related parameter settings requires the Technical-Committee signature to execute.
* update_ocw_control_setting
  Update the control parameters of ares-oracle
* revoke_update_request_propose
  Revoke the key-pair on the request token list.
* update_request_propose
  Modify or add a key-pair to the request list.
* update_allowable_offset_propose
  Update the value of the allowable offset parameter to determine the abnormal range of the submitted price
* update_pool_depth_propose
  Update the depth of the price pool. When the price pool reaches the maximum value, the average price will be aggregated and put on the chain.
* update_pre_check_token_list
  Update the pre-checked Trading pairs list for checking the validator price feature.
* update_pre_check_session_multi
  session-multi indicates the trigger pre-check session period before the era.
* update_pre_check_allowable_offset
  The maximum offset allowed for pre-check when validation.


#### Workflow
**Validator Pre-Audit**
1. Ares-oracle obtains the newly elected validator through the `IStakingNpos::pending_npos` trait,
   and submits the `pre-check` task to the chain through `submit_create_pre_check_task` extrinsics.
2. Use the `IStakingNpos::near_era_change` trait to determine whether the session period close to the election has been reached, and if so, submit the new verification task to the chain through `submit_offchain_pre_check_result` extrinsics.
3. Validators that do not pass the review will not appear in the list of targets for elections.

**Free Trade-price**
1. When the offchain is working, the block author of the current block will be obtained according to the data provided by `authorship`.
2. If the "authority-id" corresponding to the block author is consistent with the local "keystore",
   obtain the list of "transaction pairs" corresponding to the block, and send an http request to obtain its price,
   then call the `submit_price_unsigned_with_signed_payload`  extrinsics upload the result to the chain .
3. The chain will verify whether the submission matches the block author of the current block, and if so, the result will be stored in the `price` pool.
4. When the `price` pool reaches the specified depth (this depth can be modified by the extrinsics `update_pool_depth_propose`),
   the average price aggregation event `Event::AggregatedPrice` occurs, which will generate an average price to the chain.
5. The price related to a `trade pairs` can be read through the `aresOracle.aresAvgPrice store`.

**Paid Trade-price**
1. Users can submit a `submit_ask_price` transaction to have all nodes on the chain make an offer,
   but this requires payment some fee, which is more time-sensitive than getting the price for free.
2. If submit a task successfully, you need to get the `purchase_id` from the `Event::NewPurchasedRequest` event,
   which will be used as the associated key value for other queries. The amount of payment is related to the number
   of `trade pairs` requests. If the nodes participating in the response are less than the response threshold,
   the task will fail, and the fee will not be deducted. (This threshold can be modified by extrinsics `update_purchased_param`).
3. Once the price are aggregated successfully, an `Event::PurchasedAvgPrice` event will be generated,
   and the relevant fees will be deducted from the `origin` account. Users can also read the corresponding result data through the `aresOracle.purchasedAvgPrice` store.

#### Moudle: oracle-provider-support
* has_pre_check_task
  Determine whether there is a pre-check task for the validator through a stash account.

* get_pre_task_by_authority_set
  Get the pre-check information related to a certain ares-authority collection, the specific matching authority-id, account-id, and the block submitted by the task.
  Precheck tasks that only match the first ares-authority

* check_and_clean_obsolete_task
  Trigger this method on a specific cycle to clean up too old and passed tasks

* take_price_for_pre_check
  Obtain PreCheckList result data according to Trading pairs specified by check_config

* save_pre_check_result
  Will verify the data on-chain based on the result of PreCheckList and return PreCheckStatus as the result

*  get_pre_check_status
   Get the pre-check status that a validator has stored, this status will affect whether it will be added to the validator list.

* clean_pre_check_status
  Remove pre-check status stored by a validator

* create_pre_check_task
  Create a pre-check task, return true if the creation is successful else return false

#### Workflow
**KeyTypeId of Ares**
1. Configure the definition of the `sr25519 AuthorityId` provided by the `scrypto` module into `ares-oracle::AuthorityAres`.
2. If session is enabled, `ares-oracle` needs to be configured into SessionKey.

**Pre-review task flow**
1. Use `has_pre_check_task` to determine whether a validator has a pre-check task.
2. If there is a pre-review task, get the corresponding price response through `take_price_for_pre_check`.
3. Check and save the result data through `save_pre_check_result`,
   the returned results include `PreCheckStatus::Prohibit` and `PreCheckStatus::Pass`.
4. Pass the block height to the `check_and_clean_obsolete_task` function to remove obsolete pre-review tasks and pre-check result to prevent overbloating.


#### Pallet: oracle-finance
* current_era_num
  To get the current ear, you need to consider that if the era-length changes, you still need to guarantee that the vector of time zones increases.

* get_earliest_reward_era
  Get the earliest reward era.

* calculate_fee_of_ask_quantity
  Input in a price_count to calculate the cost of the purchase.

* reserve_for_ask_quantity
  Keep the balance for the purchase.

* unreserve_ask
  Release the funds reserved for the purchase, which occurs after the failure of the ask-price.

* pay_to_ask
  Execute the payment, which will transfer the user’s balance to the Pallet

* record_submit_point
  Record the Points of the validator under an order

* get_record_point
  Get the Point of the record
* take_reward
  Claim all rewards under a given era

*  get_era_income
   Get total income balance for an era.

* get_era_point
  Read the total balance for an era.

* check_and_slash_expired_rewards
  Check for expired rewards and destroy them if overdue

* take_purchase_reward
  Validators get rewards corresponding to eras.
  Note: An ear cannot be the current unfinished era, and rewards are not permanently stored. If the reward exceeds the depth defined by T::HistoryDepth, you will not be able to claim it.

* take_all_purchase_reward
  Validators get rewards, it will help validators get all available rewards
  Note: An ear cannot be the current unfinished era, and rewards are not permanently stored. If the reward exceeds the depth defined by T::HistoryDepth, you will not be able to claim it.

#### Workflow
**Reward Deposit**

1. Payment is through the `Trait` provided by `IForPrice`. It is necessary to call `reserve_for_ask_quantity`
   to reserve the part balance of asker and associate it with the order-id.

2. Through the Trait implementation provided by `IForReporter`, call `record_submit_point` to save the point.
   The block height needs to be passed in, and the pallet will convert it to the corresponding era and record it under an order-id.

3. After the price-response is successful, call `pay_to_ask` to release the Balance reserved above,
   and pay the actual ask fee to the oracle-finance pallet.

**Claim Reward**
1. Rewards are claimed by era, and rewards cannot be obtained for unfinished eras. You need to calculate
   the 'Balance' corresponding to each point in the corresponding era. Algorithm: `Single Point Bonus` = `Total Revenue`/`Total Points` on era.
2. The validator needs to use the `Controller` account to perform the claim operation, This is for security reasons.
   The total amount received is equal to:  `Validator Points` * `Single Point Bonus`.
3. Rewards are transferred to the validator's Stash account via `oracle-finance` pallet.

#### Moudle: staking-extend
* current_staking_era()  /
  Get the Era of the current Staking pallet.

* near_era_change()      
  period_multiple: session-multi indicates the trigger pre-check session period before the era.
* old_npos()             
  Get the list of validators before the election.
* pending_npos()         
  Get the list of new validators after the current election, excluding validators from the previous session.
#### Workflow
**Adapter execution**

1. `ElectionProvider` of `Staking Config` is set to `staking_extend::elect::OnChainSequentialPhragmen`,
   `GenesisElectionProvider` is set to `staking_extend::elect::OnChainSequentialPhragmenGenesis`
2. `DataProvider` of `frame_election_provider_support::onchain::Config` is set to `staking_extend::data::DataProvider`
3. `DataProvider` of `pallet_election_provider_multi_phase::Config` is set to `staking_extend::data::DataProvider<Self>`
4. After that, the staking election request will be sent to `staking_extend`,
   and then the Election module will also obtain the candidate list from `staking_extend`,
   and the adapter connection is successful.
5. Call the `pending_npos` method to obtain a list of the new elected list,
   and then cooperate with the implementation of the `IAresOraclePreCheck::get_pre_check_status` trait to block the newly selected validator.
