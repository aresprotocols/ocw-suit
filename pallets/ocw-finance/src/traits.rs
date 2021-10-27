use super::*;

trait IPuzzleCreators<T: Config> {
	// The deposit necessary for questioning, the default deposit is obtained through on-chain governance
	fn do_deposit();

	// Operation of releasing the deposit and returning the deposit
	fn withdraw_deposit();

	// Return value (fee, block_number)
	fn get_deposit() -> (BalanceOf<T>, T::BlockNumber);
}
