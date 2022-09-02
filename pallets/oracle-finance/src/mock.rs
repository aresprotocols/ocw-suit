use crate as oracle_finance;

use crate::types::{EraIndex};
use frame_support::sp_std::convert::TryInto;
use frame_support::{parameter_types, traits::{Get, Contains, GenesisBuild, Hooks}, PalletId, BoundedVec};
use frame_system as system;
use pallet_balances;
use sp_core::H256;
use sp_runtime::testing::UintAuthorityId;
use sp_runtime::traits::Convert;
use sp_runtime::{ testing::Header, traits::{BlakeTwo256, IdentityLookup, Zero}};
use sp_std::convert::TryFrom;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
// pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type AccountId = u64;
/// Balance of an account.
pub type Balance = u64;
pub type BlockNumber = u64;
pub type SessionIndex = u32;
pub const DOLLARS: u64 = 1_000_000_000_000;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		OracleFinance: oracle_finance::{Pallet, Call, Storage, Event<T>},
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
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const AresFinancePalletId: PalletId = PalletId(*b"ocw/fund");
	pub const BasicDollars: Balance = DOLLARS;
	// pub const AskPeriod: BlockNumber = 10;
	pub const AskPerEra: SessionIndex = 2;
	pub const HistoryDepth: u32 = 2;
}

impl oracle_finance::Config for Test {
	type Event = Event;
	type PalletId = AresFinancePalletId;
	type Currency = pallet_balances::Pallet<Self>;
	type BasicDollars = BasicDollars;
	type ValidatorId = AccountId;
	type ValidatorIdOf = StashOf;
	type AskPerEra = AskPerEra;
	type HistoryDepth = HistoryDepth;
	type SessionManager = ();
	type OnSlash = ();
}

parameter_types! {
	pub const Period: u64 = 5;
	pub const Offset: u64 = 0;
}
impl pallet_session::Config for Test {
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = OracleFinance;
	type SessionHandler = TestSessionHandler;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ();
	type Keys = UintAuthorityId;
	type Event = Event;
	// type DisabledValidatorsThreshold = (); // pallet_session::PeriodicSessions<(), ()>;
	type NextSessionRotation = (); //pallet_session::PeriodicSessions<(), ()>;
	type WeightInfo = ();
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[];
	fn on_genesis_session<Ks: sp_runtime::traits::OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}
	fn on_new_session<Ks: sp_runtime::traits::OpaqueKeys>(_: bool, _: &[(AccountId, Ks)], _: &[(AccountId, Ks)]) {}
	fn on_disabled(_: u32) {}
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 100;
	pub const MaxLocks: u32 = 10;
}
impl pallet_balances::Config for Test {
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type MaxLocks = MaxLocks;
	type Balance = Balance;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 1000000000100), (2, 2000000000100), (3, 3000000000100), (4, 4000000000100), (5, 5000000000100), (6, 6000000000100)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	crate::GenesisConfig::<Test> {
		_pt: Default::default()
	}.assimilate_storage(&mut t).unwrap();
	
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn to_test_vec<MaxLen: Get<u32>>(to_str: &str) -> BoundedVec<u8, MaxLen> {
	to_str.as_bytes().to_vec().try_into().unwrap()
}

// -------------- Session Management.

pub const INIT_TIMESTAMP: u64 = 30_000;
pub const BLOCK_TIME: u64 = 1000;

pub(crate) fn run_to_block(n: BlockNumber) {
	OracleFinance::on_finalize(System::block_number());
	for b in (System::block_number() + 1)..=n {
		System::set_block_number(b);
		Session::on_initialize(b);
		<OracleFinance as Hooks<u64>>::on_initialize(b);
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		if b != n {
			OracleFinance::on_finalize(System::block_number());
		}
	}
}

/// Progresses from the current block number (whatever that may be) to the `P * session_index + 1`.
pub(crate) fn start_session(session_index: SessionIndex) {
	let end: u64 = if Offset::get().is_zero() {
		(session_index as u64) * Period::get()
	} else {
		Offset::get() + (session_index.saturating_sub(1) as u64) * Period::get()
	};
	run_to_block(end);
	// session must have progressed properly.
	assert_eq!(
		Session::current_index(),
		session_index,
		"current session index = {}, expected = {}",
		Session::current_index(),
		session_index,
	);
}

/// Go one session forward.
pub(crate) fn advance_session() {
	let current_index = Session::current_index();
	start_session(current_index + 1);
}

/// Progress until the given era.
pub(crate) fn start_active_era(era_index: EraIndex) {
	start_session((era_index * <AskPerEra as Get<u32>>::get()).into());
}


pub struct StashOf;
impl Convert<AccountId, Option<AccountId>> for StashOf {
	fn convert(controller: AccountId) -> Option<AccountId> {
		Some(controller)
	}
}