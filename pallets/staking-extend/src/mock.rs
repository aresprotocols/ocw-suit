use crate as member_extend;
use frame_support::sp_runtime::app_crypto::sp_core::sr25519::Signature;
use frame_support::sp_runtime::traits::{IdentifyAccount, Verify, Convert};
use frame_support::{
	assert_noop, assert_ok, ord_parameter_types, parameter_types,
	traits::{Contains, GenesisBuild, OnInitialize, SortedMembers},
	weights::Weight,
	PalletId,
};


use frame_system as system;
use frame_election_provider_support;
// use pallet_balances;
// use pallet_balances::{BalanceLock, Error as BalancesError};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

// use frame_benchmarking::frame_support::pallet_prelude::Get;
use frame_election_provider_support::{ElectionProvider, onchain, data_provider, VoteWeight, Supports, Support};
use frame_support::traits::ValidatorSet;
use frame_support::sp_runtime::Perbill;
use frame_support::pallet_prelude::PhantomData;

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

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		// ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},
		// Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
		// Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		// Historical: pallet_session::historical::{Pallet},

		MemberExtend: member_extend::{Pallet},

		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		// Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		// OcwFinance: ocw_finance::{Pallet, Call, Storage, Event<T>},
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
	type AccountData = () ;// pallet_balances::AccountData<Balance>;
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
	type DataProvider = TestStakingDataProvider;
	// type DebugError = <<Self as staking_extend::Config>::ElectionProvider as ElectionProvider<<Self as frame_system::Config>::AccountId, <Self as frame_system::Config>::BlockNumber>>::Error;
	type ElectionProvider = TestElectionProvider<member_extend::Pallet<Self>>;
	type OnChainAccuracy = Perbill;
	type GenesisElectionProvider = onchain::OnChainSequentialPhragmen<
		OnChainConfig<member_extend::Pallet<Self>>,
	>;
}

const CONST_VALIDATOR_ID_1:AccountId= 1001;
const CONST_VALIDATOR_ID_2:AccountId= 1002;
const CONST_VALIDATOR_ID_3:AccountId= 1003;
const CONST_VALIDATOR_ID_4:AccountId= 1004;
const CONST_FEATURE_VALIDATOR_ID_5:AccountId= 1005;
const CONST_VOTER_ID_1: AccountId = 1011;
const CONST_VOTER_ID_2: AccountId = 1012;

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

pub struct TestStakingDataProvider ;
impl frame_election_provider_support::ElectionDataProvider<AccountId, BlockNumber> for TestStakingDataProvider {
	const MAXIMUM_VOTES_PER_VOTER: u32 = 0;

	fn targets(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<AccountId>> {
		// submit 3 times. 3% , 2/3 10 block submit.
		Ok(vec![
			CONST_VALIDATOR_ID_1,
			CONST_VALIDATOR_ID_2,
			CONST_VALIDATOR_ID_3,
			CONST_VALIDATOR_ID_4,
			CONST_FEATURE_VALIDATOR_ID_5,
		])
	}

	fn voters(maybe_max_len: Option<usize>) -> data_provider::Result<Vec<(AccountId, VoteWeight, Vec<AccountId>)>> {
		Ok(vec![
			(CONST_VOTER_ID_1, 100, vec![CONST_VALIDATOR_ID_4, CONST_FEATURE_VALIDATOR_ID_5]),
			(CONST_VOTER_ID_2, 100, vec![CONST_FEATURE_VALIDATOR_ID_5])
		])
	}

	fn desired_targets() -> data_provider::Result<u32> {
		Ok(4)
	}

	fn next_election_prediction(now: BlockNumber) -> BlockNumber {
		400
	}
}

pub struct TestElectionProvider<TestDataProvider>(PhantomData<TestDataProvider>);
impl <TestDataProvider: frame_election_provider_support::ElectionDataProvider<AccountId, BlockNumber>>
	frame_election_provider_support::ElectionProvider<AccountId, BlockNumber> for TestElectionProvider<TestDataProvider> {
	// type Error = T::DebugError;
	type Error = (); // <Self as ElectionProvider<AccountId, BlockNumber>>::Error;
	type DataProvider = TestDataProvider;// <Test as crate::Config>::DataProvider ;

	fn elect() -> Result<Supports<AccountId>, Self::Error> {
		// let support = Support{
		// 	total: 10000,
		// 	voters: vec![]
		// };
		let mut supports = Supports::<AccountId>::new();
		supports.push((CONST_FEATURE_VALIDATOR_ID_5, Support{
			total: 10000,
			voters: vec![(CONST_VOTER_ID_1, 5000), (CONST_VOTER_ID_2, 5000), ]
		}));
		supports.push((CONST_VALIDATOR_ID_1, Support{
			total: 0,
			voters: vec![]
		}));
		supports.push((CONST_VALIDATOR_ID_2, Support{
			total: 0,
			voters: vec![]
		}));
		supports.push((CONST_VALIDATOR_ID_3, Support{
			total: 0,
			voters: vec![]
		}));

		Ok(supports)
	}
}

/// Wrapper type that implements the configurations needed for the on-chain backup.
pub struct OnChainConfig<TestDataProvider>(PhantomData<TestDataProvider>);
impl <TestDataProvider: frame_election_provider_support::ElectionDataProvider<AccountId, BlockNumber>>
	onchain::Config for OnChainConfig<TestDataProvider> {
	type AccountId = AccountId;
	type BlockNumber = BlockNumber;
	type Accuracy = Perbill;
	type DataProvider = TestDataProvider;
}


// pallet_election_provider_multi_phase

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