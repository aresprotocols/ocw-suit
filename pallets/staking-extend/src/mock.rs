use crate as member_extend;
use frame_support::sp_runtime::app_crypto::sp_core::sr25519::Signature;
use frame_support::sp_runtime::traits::{IdentifyAccount, Verify, Convert, OpaqueKeys};
use frame_support::{
	assert_noop, assert_ok, ord_parameter_types, parameter_types,
	traits::{Contains, GenesisBuild, OnInitialize, SortedMembers},
	weights::Weight,
	PalletId,
};

use sp_runtime::{
	curve::PiecewiseLinear,
	testing::{Header, TestXt, UintAuthorityId},
	traits::{IdentityLookup, Zero, BlakeTwo256},
};

use frame_system as system;
use frame_election_provider_support;
// use pallet_balances;
// use pallet_balances::{BalanceLock, Error as BalancesError};

// use frame_benchmarking::frame_support::pallet_prelude::Get;
use frame_election_provider_support::{ElectionProvider, onchain, data_provider, VoteWeight, Supports, Support};
use frame_support::traits::{ValidatorSet, OneSessionHandler, Hooks, Get};
use frame_support::sp_runtime::Perbill;
use frame_support::pallet_prelude::PhantomData;
use std::borrow::BorrowMut;
use std::{cell::RefCell, collections::HashSet};
use pallet_staking::{StakerStatus, EraIndex};
// use sp_core::{crypto::key_types::DUMMY, H256};
use sp_core::{H256};
use frame_system::limits::BlockWeights;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
// pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub(crate) type AccountId = u64;
/// Balance of an account.
pub type Balance = u64;
pub type BlockNumber = u64;
pub type AskPeriodNum = u64;
pub type SessionIndex = u32;
pub const DOLLARS: u64 = 1_000_000_000_000;

thread_local! {
	static SESSION: RefCell<(Vec<AccountId>, HashSet<AccountId>)> = RefCell::new(Default::default());
}

/// Another session handler struct to test on_disabled.
pub struct OtherSessionHandler;

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
	type Public = UintAuthorityId;
	// type Public = <Test as crate::Config>::AuthorityId;
}

impl OneSessionHandler<AccountId> for OtherSessionHandler {
	// type Key = <Test as crate::Config>::AuthorityId;//
	type Key = UintAuthorityId;

	fn on_genesis_session<'a, I: 'a>(_: I)
		where
			I: Iterator<Item = (&'a AccountId, UintAuthorityId)>,
			AccountId: 'a,
	{
		// println!(" Debug . on_genesis_session ");
	}

	fn on_new_session<'a, I: 'a>(_: bool, validators: I, queued_validators: I)
		where
			I: Iterator<Item = (&'a AccountId, UintAuthorityId)>,
			AccountId: 'a,
	{
		// println!(" Debug . on_new_session ");
		// let current_validators = validators.map(|(_, k)| k).collect::<Vec<_>>();
		let next_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();
		// println!("*** LINDEBUG:: current_validators == {:?}", current_validators);
		// println!("*** LINDEBUG:: next_authorities == {:?}", next_authorities);

		SESSION.with(|x| {
			*x.borrow_mut() = (validators.map(|x| x.0.clone()).collect(), HashSet::new())
		});
	}

	fn on_disabled(validator_index: usize) {
		// println!(" Debug . on_disabled ");
		SESSION.with(|d| {
			let mut d = d.borrow_mut();
			let value = d.0[validator_index];
			d.1.insert(value);
		})
	}
}


// -------------------------

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{

		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config},

		// ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},
		Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		// Historical: pallet_session::historical::{Pallet},
		StakingExtend: crate::{Pallet},

		MemberExtend: member_extend::{Pallet},
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
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
	pub const MinimumPeriod: u64 = 5;
}
impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
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

impl crate::Config for Test {
	type ValidatorId = AccountId;
	type ValidatorSet = TestValidatorSet;

	// type WithSessionHandler = OtherSessionHandler ;
	// type AuthorityId = <OtherSessionHandler as OneSessionHandler<AccountId>>::Key;
	type AuthorityId = UintAuthorityId;

	// type StashId = AccountId;// <Self as frame_system::Config>::AccountId;
	// type IStakingNpos = Self;
	type DataProvider = Staking  ;// TestStakingDataProvider;
	// type DebugError = <<Self as staking_extend::Config>::ElectionProvider as ElectionProvider<<Self as frame_system::Config>::AccountId, <Self as frame_system::Config>::BlockNumber>>::Error;
	type ElectionProvider = TestElectionProvider<member_extend::Pallet<Self>>;
	type OnChainAccuracy = Perbill;

	type GenesisElectionProvider = onchain::OnChainSequentialPhragmen<
		OnChainConfig<member_extend::Pallet<Self>>,
	>;

	type AresOraclePreCheck = ();
}

parameter_types! {
	pub const UncleGenerations: u64 = 0;
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(25);
	pub const MaxAuthorities: u32 = 100;
	pub static Period: BlockNumber = 5;
	pub static Offset: BlockNumber = 0;
}

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub dummy: OtherSessionHandler,
	}
}

impl From<UintAuthorityId> for SessionKeys {
	fn from(dummy: UintAuthorityId) -> Self {
		Self { dummy }
	}
}

impl pallet_session::Config for Test {
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
	type Keys = SessionKeys;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	// type SessionHandler = (StakingExtend,); // (OtherSessionHandler,); // (StakingExtend); // (OtherSessionHandler,);
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type Event = Event;
	type ValidatorId = AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Test>;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Test>;
}

impl pallet_authority_discovery::Config for Test {
	// type MaxAuthorities = MaxAuthorities;
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

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

pub type Extrinsic = TestXt<Call, ()>;
impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
	where
		Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

parameter_types! {
	pub const SessionsPerEra: sp_staking::SessionIndex = 10; // constants::time::EPOCH_DURATION_IN_BLOCKS (one session 10 min)
	pub const BondingDuration: pallet_staking::EraIndex = 8;
	pub const SlashDeferDuration: pallet_staking::EraIndex = 4; // 1/2 the bonding duration.
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const MaxNominatorRewardedPerValidator: u32 = 256;
	pub OffchainRepeat: BlockNumber = 5;
}

impl pallet_staking::Config for Test {
	const MAX_NOMINATIONS: u32 = 256;
	type Currency = Balances;
	type UnixTime = Timestamp;
	type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
	type RewardRemainder = ();
	type Event = Event;
	type Slash = (); // send the slashed funds to the treasury.
	type Reward = (); // rewards are minted from the void
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
	type SessionInterface = Self;
	type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
	type NextNewSession = Session;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	// type ElectionProvider =  ElectionProviderMultiPhase;
	type ElectionProvider = StakingExtend ;// StakingExtend;// // ElectionProviderMultiPhase;
	// type GenesisElectionProvider = onchain::OnChainSequentialPhragmen<
	// 	pallet_election_provider_multi_phase::OnChainConfig<Self>,
	// >;
	type GenesisElectionProvider = onchain::OnChainSequentialPhragmen<
		crate::OnChainConfig<Self>,
	>;
	type WeightInfo = pallet_staking::weights::SubstrateWeight<Test>;
}

pub(crate) const CONST_VALIDATOR_ID_1:AccountId= 1001;
pub(crate) const CONST_VALIDATOR_ID_2:AccountId= 1002;
pub(crate) const CONST_VALIDATOR_ID_3:AccountId= 1003;
pub(crate) const CONST_VALIDATOR_ID_4:AccountId= 1004;
pub(crate) const CONST_VALIDATOR_ID_5:AccountId= 1005;
pub(crate) const CONST_VALIDATOR_ID_6:AccountId= 1006;
pub(crate) const CONST_VOTER_ID_1: AccountId = 1011;
pub(crate) const CONST_VOTER_ID_2: AccountId = 1012;

pub struct TestStashOf<AccountId>(PhantomData<AccountId>);
impl Convert<AccountId, Option<AccountId>> for TestStashOf<AccountId> {
	fn convert(controller: AccountId) -> Option<AccountId> {
		Some(controller)
	}
}
pub struct TestValidatorSet ;
impl ValidatorSet<AccountId> for TestValidatorSet {
	type ValidatorId = AccountId;
	type ValidatorIdOf = TestStashOf<AccountId>;

	fn session_index() -> SessionIndex {
		1
	}

	fn validators() -> Vec<Self::ValidatorId> {
		vec![
			CONST_VALIDATOR_ID_1,
			CONST_VALIDATOR_ID_2,
			CONST_VALIDATOR_ID_3,
			CONST_VALIDATOR_ID_4,
		]
	}
}

// pub struct TestStakingDataProvider ;
// impl frame_election_provider_support::ElectionDataProvider<AccountId, BlockNumber> for TestStakingDataProvider {
// 	const MAXIMUM_VOTES_PER_VOTER: u32 = 0;
//
// 	fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<AccountId>> {
// 		// submit 3 times. 3% , 2/3 10 block submit.
// 		Ok(vec![
// 			CONST_VALIDATOR_ID_1,
// 			CONST_VALIDATOR_ID_2,
// 			CONST_VALIDATOR_ID_3,
// 			CONST_VALIDATOR_ID_4,
// 			CONST_FEATURE_VALIDATOR_ID_5,
// 		])
// 	}
//
// 	fn voters(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<(AccountId, VoteWeight, Vec<AccountId>)>> {
// 		Ok(vec![
// 			(CONST_VOTER_ID_1, 100, vec![CONST_VALIDATOR_ID_4, CONST_FEATURE_VALIDATOR_ID_5]),
// 			(CONST_VOTER_ID_2, 100, vec![CONST_FEATURE_VALIDATOR_ID_5])
// 		])
// 	}
//
// 	fn desired_targets() -> data_provider::Result<u32> {
// 		Ok(4)
// 	}
//
// 	fn next_election_prediction(now: BlockNumber) -> BlockNumber {
// 		400
// 	}
// }

pub struct TestElectionProvider<TestDataProvider>(PhantomData<TestDataProvider>);
impl <TestDataProvider: frame_election_provider_support::ElectionDataProvider<AccountId, BlockNumber>>
	frame_election_provider_support::ElectionProvider<AccountId, BlockNumber> for TestElectionProvider<TestDataProvider> {
	// type Error = T::DebugError;
	type Error = (); // <Self as ElectionProvider<AccountId, BlockNumber>>::Error;
	type DataProvider = TestDataProvider;// <Test as crate::Config>::DataProvider ;

	fn elect() -> Result<(Supports<AccountId>, Weight), Self::Error> {
		// let support = Support{
		// 	total: 10000,
		// 	voters: vec![]
		// };
		let mut supports = Supports::<AccountId>::new();
		// supports.push((CONST_FEATURE_VALIDATOR_ID_5, Support{
		// 	total: 10000,
		// 	voters: vec![(CONST_VOTER_ID_1, 5000), (CONST_VOTER_ID_2, 5000), ]
		// }));
		supports.push((CONST_VALIDATOR_ID_1, Support{
			total: 0,
			voters: vec![]
		}));
		supports.push((CONST_VALIDATOR_ID_2, Support{
			total: 0,
			voters: vec![]
		}));
		Ok((supports, 0))
	}
}

/// Wrapper type that implements the configurations needed for the on-chain backup.
pub struct OnChainConfig<TestDataProvider>(PhantomData<TestDataProvider>);
impl <TestDataProvider: frame_election_provider_support::ElectionDataProvider<AccountId, BlockNumber>>
	onchain::Config for OnChainConfig<TestDataProvider> {
	type BlockWeights = ();
	type AccountId = AccountId;
	type BlockNumber = BlockNumber;
	type Accuracy = Perbill;
	type DataProvider = TestDataProvider;
}


// pallet_election_provider_multi_phase

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(CONST_VALIDATOR_ID_1, 1000000000100),
			(CONST_VALIDATOR_ID_2, 2000000000100),
			(CONST_VALIDATOR_ID_3, 3000000000100),
			(CONST_VALIDATOR_ID_4, 4000000000100),
			// (CONST_VALIDATOR_ID_5, 5000000000100),
			// (CONST_VALIDATOR_ID_6, 6000000000100)
		],
	}.assimilate_storage(&mut t)
	.unwrap();

	pallet_staking::GenesisConfig::<Test> {
		validator_count: 2u32,
		minimum_validator_count: 2u32,
		invulnerables: vec![],
		slash_reward_fraction: Perbill::from_percent(10),
		stakers: vec![
			(CONST_VALIDATOR_ID_1, CONST_VALIDATOR_ID_1, 1000,StakerStatus::Validator),
			(CONST_VALIDATOR_ID_2, CONST_VALIDATOR_ID_2, 1000,StakerStatus::Validator),
		],
		..Default::default()
	}.assimilate_storage(&mut t)
	.unwrap();

	// crate::GenesisConfig::<Test> {
	// 	_pt: Default::default()
	// }.assimilate_storage(&mut t).unwrap();
	
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn to_test_vec(to_str: &str) -> Vec<u8> {
	to_str.as_bytes().to_vec()
}

// -------------- Session Management.

pub const INIT_TIMESTAMP: u64 = 30_000;
pub const BLOCK_TIME: u64 = 1000;

pub(crate) fn run_to_block(n: BlockNumber) {
	Staking::on_finalize(System::block_number());
	for b in (System::block_number() + 1)..=n {
		System::set_block_number(b);
		Session::on_initialize(b);
		<Staking as Hooks<u64>>::on_initialize(b);
		Timestamp::set_timestamp(System::block_number() * BLOCK_TIME + INIT_TIMESTAMP);
		if b != n {
			Staking::on_finalize(System::block_number());
		}
	}
}

pub(crate) fn active_era() -> EraIndex {
	Staking::active_era().unwrap().index
}

pub(crate) fn current_era() -> EraIndex {
	Staking::current_era().unwrap()
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
	start_session((era_index * <SessionsPerEra as Get<u32>>::get()).into());
	assert_eq!(active_era(), era_index);
	// One way or another, current_era must have changed before the active era, so they must match
	// at this point.
	assert_eq!(current_era(), active_era());
}
