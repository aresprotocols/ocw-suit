
## Methods 

### has_pre_check_task

```rust
fn has_pre_check_task (stash: AccountId) -> bool
```
Determine whether there is a pre-check task for the validator through a stash account.

### get_pre_task_by_authority_set
```rust
fn get_pre_task_by_authority_set(
    auth_list: Vec<AuthorityId>
) -> Option<(AccountId, AuthorityId, BlockNumber)>
```
Get the pre-check information related to a certain ares-authority collection, the specific matching authority-id, account-id, and the block submitted by the task.

Precheck tasks that only match the first ares-authority

### check_and_clean_obsolete_task
```rust
fn check_and_clean_obsolete_task(maximum_due: BlockNumber) -> Weight
```
Trigger this method on a specific cycle to clean up too old and passed tasks

### take_price_for_pre_check
```rust
fn take_price_for_pre_check(check_config: PreCheckTaskConfig) -> PreCheckList
```
Obtain PreCheckList result data according to Trading pairs specified by check_config

### save_pre_check_result
```rust
fn save_pre_check_result(
    stash: AccountId,
    bn: BlockNumber,
    pre_check_list: PreCheckList
) -> PreCheckStatus
```
Will verify the data on-chain based on the result of PreCheckList and return PreCheckStatus as the result

### get_pre_check_status
```rust
fn get_pre_check_status(
    stash: AccountId
) -> Option<(BlockNumber, PreCheckStatus)>
```
Get the pre-check status that a validator has stored, this status will affect whether it will be added to the validator list.

### clean_pre_check_status
```rust
fn clean_pre_check_status(stash: AccountId)
```
Remove pre-check status stored by a validator

### create_pre_check_task
```rust
fn create_pre_check_task(
    stash: AccountId,
    auth: AuthorityId,
    bn: BlockNumber
) -> bool
```
Create a pre-check task, return true if the creation is successful else return false

## Workflow 

### KeyTypeId of Ares
1. Configure the definition of the `sr25519 AuthorityId` provided by the `scrypto` module into `ares-oracle::AuthorityAres`.
2. If session is enabled, `ares-oracle` needs to be configured into SessionKey.

### Pre-review task flow
1. Use `has_pre_check_task` to determine whether a validator has a pre-check task.
2. If there is a pre-review task, get the corresponding price response through `take_price_for_pre_check`.
3. Check and save the result data through `save_pre_check_result`, 
   the returned results include `PreCheckStatus::Prohibit` and `PreCheckStatus::Pass`.
4. Pass the block height to the `check_and_clean_obsolete_task` function to remove obsolete pre-review tasks and pre-check result to prevent overbloating.
