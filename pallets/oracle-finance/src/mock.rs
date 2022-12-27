use crate as oracle_finance;

use crate::types::{EraIndex};
use frame_support::sp_std::convert::TryInto;
use frame_support::{parameter_types, traits::{Get, Contains, GenesisBuild, Hooks, LockIdentifier}, PalletId, BoundedVec};
use frame_system as system;
use pallet_balances;
use sp_core::H256;
use sp_runtime::testing::UintAuthorityId;
use sp_runtime::traits::Convert;
use sp_runtime::{ testing::Header, traits::{BlakeTwo256, IdentityLookup, Zero}};
use sp_std::convert::TryFrom;
use sp_runtime::curve::PiecewiseLinear;
use ares_oracle_provider_support::{OrderIdEnum};
use crate::test_tools::*;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;


frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Timestamp: pallet_timestamp,
		Session: pallet_session,
		Balances: pallet_balances,
		OracleFinance: crate::<Instance1>,
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

pub struct StashOf;
impl Convert<AccountId, Option<AccountId>> for StashOf {
	fn convert(controller: AccountId) -> Option<AccountId> {
		Some(controller)
	}
}


pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		// min_inflation: 0_025_000,
		// max_inflation: 0_100_000,
		min_inflation: 0_024_575,
		max_inflation: 0_024_576,
		// 3:2:1 staked : parachains : float.
		// while there's no parachains, then this is 75% staked : 25% float.
		ideal_stake: 0_750_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub const AresFinancePalletId: PalletId = PalletId(*b"ocw/fund");
	pub const BasicDollars: Balance = DOLLARS;
	// pub const AskPeriod: BlockNumber = 10;
	pub const AskPerEra: SessionIndex = 2;
	pub const HistoryDepth: u32 = 2;
	pub const TestFinanceLockIdentifier : LockIdentifier = *b"testing ";

	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
}

type OracleFinanceInstance = oracle_finance::Instance1;
impl oracle_finance::Config<OracleFinanceInstance> for Test {
	type AskPerEra = AskPerEra;
	type BasicDollars = BasicDollars;
	type Currency = pallet_balances::Pallet<Self>;
	type Event = Event;
	type HistoryDepth = HistoryDepth;
	type LockIdentifier = TestFinanceLockIdentifier;
	type OnSlash = ();
	type PalletId = AresFinancePalletId;
	type SessionManager = ();
	type ValidatorId = AccountId;
	type ValidatorIdOf = StashOf;
	// type OrderId = PurchaseId;
	// type OrderId2 = OrderIdEnum::String(PurchaseId);
	type WeightInfo = ();
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
	crate::GenesisConfig::<Test, crate::Instance1> {
		_pt: Default::default()
	}.assimilate_storage(&mut t).unwrap();
	
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn to_test_vec<MaxLen: Get<u32>>(to_str: &str) -> BoundedVec<u8, MaxLen> {
	to_str.as_bytes().to_vec().try_into().unwrap()
}

pub fn to_enum_id(to_str: &str) -> OrderIdEnum {
	OrderIdEnum::String(to_test_vec(to_str))
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


