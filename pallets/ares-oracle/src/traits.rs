use super::*;
use frame_support::weights::Weight;
use frame_support::sp_runtime::Percent;

pub trait SymbolInfo {
    fn price(symbol: &Vec<u8>) -> Result<(u64, FractionLength), ()>;

    fn fraction(symbol: &Vec<u8>) -> Option<FractionLength>;
}

// pub trait IAresOraclePreCheck <AccountId, AuthorityId, BlockNumber, Error>
// {
//     //
//     fn has_per_check_task(stash: AccountId) -> bool;
//
//     fn is_authority_set_has_task(auth_list: Vec<AuthorityId>) -> bool;
//
//     //
//     fn check_and_clean_obsolete_task(maximum_due: BlockNumber) -> Weight;
//
//     // Obtain a set of price data according to the task configuration structure.
//     fn take_price_for_per_check(check_config: PreCheckTaskConfig) -> Vec<PerCheckStruct>;
//
//     // Record the per check results and add them to the storage structure.
//     fn save_per_check_result(stash: AccountId, bn: BlockNumber, per_check_list: Vec<PerCheckStruct>);
//
//     fn get_per_check_status(stash: AccountId) -> Option<(BlockNumber, PerCheckStatus)> ;
//
//     fn create_pre_check_task(stash: AccountId, auth: AuthorityId, bn: BlockNumber) -> Result<(), Error>;
// }