use super::*;
use frame_support::weights::Weight;

pub trait SymbolInfo {
    fn price(symbol: &Vec<u8>) -> Result<(u64, FractionLength), ()>;

    fn fraction(symbol: &Vec<u8>) -> Option<FractionLength>;
}

pub trait IOcwPerCheck <AccountId, BlockNumber, Error>
{
    //
    fn has_per_check_task() -> bool;

    //
    fn check_and_clean_obsolete_task() -> Weight;

    // Obtain a set of price data according to the task configuration structure.
    fn take_price_for_per_check(check_config: PerCehckTaskConfig) -> Vec<AresPriceData<AccountId, BlockNumber>>;

    // Record the per check results and add them to the storage structure.
    fn save_per_check_result(acc: AccountId, bn: BlockNumber, round: u8, result: bool );

    fn get_per_check_status(acc: AccountId) -> Option<PerCheckStatus>;

    fn create_pre_check_task(acc: AccountId, bn: BlockNumber) -> Result<(), Error>;
}