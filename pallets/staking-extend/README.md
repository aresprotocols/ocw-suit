
## Methods

### current_staking_era

```rust
fn current_staking_era() -> u32
```

Get the Era of the current Staking pallet.

### near_era_change

```rust
fn near_era_change(period_multiple: BlockNumber) -> bool
```

Used to determine whether era changes are about to occur.
```text
period_multiple: session-multi indicates the trigger pre-check session period before the era.

```

### calculate_near_era_change

```rust
fn calculate_near_era_change(
    period_multiple: BlockNumber,
    current_bn: BlockNumber,
    session_length: BlockNumber,
    per_era: BlockNumber
) -> bool
```

### old_npos

```rust
fn old_npos() -> Vec<Self::StashId>
```

Get the list of validators before the election.

### pending_npos

```rust
fn pending_npos() -> Vec<(Self::StashId, Option<AuthorityId>)>
```

Get the list of new validators after the current election, excluding validators from the previous session.

## Workflow

### Adapter execution

1. `ElectionProvider` of `Staking Config` is set to `staking_extend::elect::OnChainSequentialPhragmen`,
   `GenesisElectionProvider` is set to `staking_extend::elect::OnChainSequentialPhragmenGenesis`
2. `DataProvider` of `frame_election_provider_support::onchain::Config` is set to `staking_extend::data::DataProvider`
3. `DataProvider` of `pallet_election_provider_multi_phase::Config` is set to `staking_extend::data::DataProvider<Self>`
4. After that, the staking election request will be sent to `staking_extend`, 
   and then the Election module will also obtain the candidate list from `staking_extend`, 
   and the adapter connection is successful.
5. Call the `pending_npos` method to obtain a list of the new elected list, 
   and then cooperate with the implementation of the `IAresOraclePreCheck::get_pre_check_status` trait to block the newly selected validator. 
