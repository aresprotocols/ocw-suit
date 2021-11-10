use crate as ocw_finance;
use frame_support::sp_runtime::app_crypto::sp_core::sr25519::Signature;
use frame_support::sp_runtime::traits::{IdentifyAccount, Verify};
use frame_support::{
	assert_noop, assert_ok, ord_parameter_types, parameter_types,
	traits::{Contains, GenesisBuild, OnInitialize, SortedMembers},
	weights::Weight,
	PalletId,
};


use frame_system as system;
use pallet_balances;
use pallet_balances::{BalanceLock, Error as BalancesError};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

use frame_benchmarking::frame_support::pallet_prelude::Get;
use frame_election_provider_support::{ElectionProvider, onchain};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
// pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type AccountId = u64;
/// Balance of an account.
pub type Balance = u64;
pub type BlockNumber = u64;
pub type AskPeriodNum = u64;
pub const DOLLARS: u64 = 1_000_000_000_000;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},
		Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Historical: pallet_session::historical::{Pallet},

		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		OcwFinance: ocw_finance::{Pallet, Call, Storage, Event<T>},
	}
);

// Scheduler must dispatch with root and no filter, this tests base filter is indeed not used.
pub struct BaseFilter;
impl Contains<Call> for BaseFilter {
	fn contains(_call: &Call) -> bool {
		true
	}
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
	type BaseCallFilter = BaseFilter;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	// type AccountData = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
}

parameter_types! {
	pub const AresFinancePalletId: PalletId = PalletId(*b"ocw/fund");
	pub const BasicDollars: Balance = DOLLARS;
	pub const AskPeriod: BlockNumber = 10;
	pub const RewardPeriodCycle: AskPeriodNum = 2;
	pub const RewardSlot: AskPeriodNum = 1;
}



impl crate::Config for Runtime {
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorSet = Historical;
	type DataProvider = Staking;
	type DebugError = <<Self as staking_extend::Config>::ElectionProvider as ElectionProvider<<Self as frame_system::Config>::AccountId, <Self as frame_system::Config>::BlockNumber>>::Error;
	type ElectionProvider = ElectionProviderMultiPhase;
	type OnChainAccuracy = Perbill;
	type GenesisElectionProvider = onchain::OnChainSequentialPhragmen<
		pallet_election_provider_multi_phase::OnChainConfig<Self>,
	>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	// pallet_balances::GenesisConfig::<Test> {
	// 	balances: vec![(1, 1000000000100), (2, 2000000000100), (3, 3000000000100), (4, 4000000000100), (5, 5000000000100), (6, 6000000000100)],
	// }
	// .assimilate_storage(&mut t)
	// .unwrap();
	// crate::GenesisConfig::<Test> {
	// 	_pt: Default::default()
	// }.assimilate_storage(&mut t).unwrap();
	
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn toVec(to_str: &str) -> Vec<u8> {
	to_str.as_bytes().to_vec()
}