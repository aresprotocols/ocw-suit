use sp_runtime::traits::Convert;
use frame_support::{parameter_types, PalletId};
use sp_runtime::generic::UncheckedExtrinsic;
use sp_runtime::testing::Block;
use sp_std::convert::TryInto;
use sp_std::convert::TryFrom;
use crate as oracle_finance;

pub(crate) type AccountId = u64;
pub(crate) type Balance = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type SessionIndex = u32;
pub(crate) const DOLLARS: u64 = 1_000_000_000_000;
