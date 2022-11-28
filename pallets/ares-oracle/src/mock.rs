use super::Event as AresOcwEvent;
use crate as ares_oracle;
use crate::*;
use staking_extend::IStakingNpos;
use codec::Decode;
use frame_support::{
    assert_ok, ord_parameter_types, parameter_types, traits::{GenesisBuild, LockIdentifier}, ConsensusEngineId, PalletId,
};

use pallet_session::historical as pallet_session_historical;
// use frame_system::InitKind;
use sp_core::{
    sr25519::Signature,
    H256,
};
use std::cell::RefCell;

use sp_runtime::{
    testing::{Header, TestXt, UintAuthorityId},
    traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
    Perbill,
};

use sp_core::sr25519::Public;
// use pallet_session::historical as pallet_session_historical;
use frame_support::traits::{ConstU32, Everything, ExtrinsicCall, FindAuthor, OnInitialize};
// use pallet_authorship::SealVerify;
use sp_runtime::traits::{Convert};
use sp_staking::SessionIndex;

use frame_system::{EnsureRoot, EnsureSignedBy};
use sp_core::hexdisplay::HexDisplay;
use std::convert::TryInto;
use frame_support::instances::Instance1;
// use lite_json::JsonValue::Null;
use frame_support::sp_runtime::traits::IsMember;
// use crate::sr25519::AuthorityId;
use frame_support::sp_runtime::app_crypto::Ss58Codec;
use frame_support::sp_runtime::testing::{Digest, DigestItem};
use frame_support::sp_std::convert::TryFrom;
use sp_application_crypto::Pair;
use sp_consensus_aura::AURA_ENGINE_ID;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

pub type Balance = u64;
pub type BlockNumber = u64;
pub type AskPeriodNum = u64;
pub const DOLLARS: u64 = 1_000_000_000_000;

type OracleFinanceInstance = oracle_finance::Instance1;

// use oracle_finance::types::*;
use oracle_finance::traits::*;

// use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use crate::AuthorityId as AuraId;
use sp_runtime::offchain::OffchainDbExt;
use ares_oracle_provider_support::PurchaseId;
use crate::test_tools::to_test_vec;

// For testing the module, we construct a mock runtime.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		// Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Historical: pallet_session_historical::{Pallet},
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		AresOcw: ares_oracle::{Pallet, Call, Storage, Event<T>, Config<T>, ValidateUnsigned},
		Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		OracleFinance: oracle_finance::<Instance1>::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const UnsignedInterval: u64 = 128;
	pub const UnsignedPriority: u64 = 1 << 20;
	pub const MaxCountOfPerRequest: u8 = 3;
	pub const FractionLengthNum: u32 = 2;
	pub const CalculationKind: u8 = 2;
	pub const ErrLogPoolDepth: u32 = 5;
}

impl crate::Config for Test {
    type Event = Event;
    type OffchainAppCrypto = crate::ares_crypto::AresCrypto<AuraId>;
    type AuthorityAres = AuraId;
    type Call = Call;
    type RequestOrigin = frame_system::EnsureRoot<AccountId>;
    type UnsignedPriority = UnsignedPriority;
    // type Currency = pallet_balances::Pallet<Self>;
    type CalculationKind = CalculationKind;
    type ErrLogPoolDepth = ErrLogPoolDepth;
    type AuthorityCount = TestAuthorityCount;
    type FinanceInstance = OracleFinanceInstance;
    type OracleFinanceHandler = OracleFinance;
    // type OracleFinanceHandler = oracle_finance::Pallet<Self, OracleFinanceInstance>;
    type AresIStakingNpos = NoNpos<Self>;
    type IOracleAvgPriceEvents = ();
    // type OrderId2 = PurchaseId;
    type WeightInfo = ();

}

parameter_types! {
	pub const AresFinancePalletId: PalletId = PalletId(*b"ocw/fund");
	pub const BasicDollars: Balance = DOLLARS;
	pub const AskPerEra: SessionIndex = 2;
	pub const HistoryDepth: u32 = 2;
    pub const TestFinanceLockIdentifier : LockIdentifier = *b"testing ";
}

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
    type WeightInfo = ();
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

#[cfg(feature = "historical")]
impl crate::historical::Config for Test {
    type FullIdentification = u64;
    type FullIdentificationOf = sp_runtime::traits::ConvertInto;
}

pub struct TestFindAuthor;
impl FindAuthor<u32> for TestFindAuthor {
    fn find_author<'a, I>(digests: I) -> Option<u32>
        where
            I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        Some(1)
    }
}

parameter_types! {
	pub const UncleGenerations: u32 = 5;
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}

parameter_types! {
	pub const MinimumPeriod: u64 = 3;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl frame_system::Config for Test {
    type BaseCallFilter = Everything; //frame_support::traits::AllowAll;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = Public;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub type Extrinsic = TestXt<Call, ()>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
    where
        Call: From<LocalCall>,
{
    type OverarchingCall = Call;
    type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
    where
        Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        _public: <Signature as Verify>::Signer,
        _account: AccountId,
        nonce: u64,
    ) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
        Some((call, (nonce, ())))
    }
}

// ord_parameter_types! {
// 	pub const One: u64 = 1;
// 	pub const Two: u64 = 2;
// 	pub const Three: u64 = 3;
// 	pub const Four: u64 = 4;
// 	pub const Five: u64 = 5;
// 	pub const Six: u64 = 6;
// }

pub struct NoNpos<T>(PhantomData<T>);
impl <A,B,T:ares_oracle::Config> IStakingNpos<A, B> for NoNpos<T> {
    type StashId = <T as frame_system::Config>::AccountId;

    fn current_staking_era() -> u32 {
        0
    }

    fn near_era_change(period_multiple: B) -> bool {
        false
    }

    fn calculate_near_era_change(period_multiple: B, current_bn: B, session_length: B, per_era: B) -> bool {
        false
    }

    fn old_npos() -> Vec<Self::StashId> {
        Vec::new()
    }

    fn pending_npos() -> Vec<(Self::StashId, Option<A>)> {
        Vec::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct TestMember;
impl IsMember<AccountId> for TestMember {
    fn is_member(member_id: &AccountId) -> bool {
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct TestAuthorityCount;
impl ValidatorCount for TestAuthorityCount {
    fn get_validators_count() -> u64 {
        4
    }
}

impl pallet_authorship::Config for Test {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, TestFindAuthor>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = AresOcw;
}

impl pallet_session::historical::Config for Test {
    type FullIdentification = AccountId;
    type FullIdentificationOf = TestHistoricalConvertInto<Self>;
}

thread_local! {
	pub static VALIDATORS: RefCell<Option<Vec<AccountId>>> = RefCell::new(Some(vec![
		AccountId::from_raw([1;32]),
		AccountId::from_raw([2;32]),
		AccountId::from_raw([3;32]),
	]));
}

pub struct TestSessionManager;
impl pallet_session::SessionManager<AccountId> for TestSessionManager {
    fn new_session(_new_index: SessionIndex) -> Option<Vec<AccountId>> {
        VALIDATORS.with(|l| l.borrow_mut().take())
    }
    fn end_session(_: SessionIndex) {}
    fn start_session(_: SessionIndex) {}
}

impl pallet_session::historical::SessionManager<AccountId, AccountId> for TestSessionManager {
    fn new_session(_new_index: SessionIndex) -> Option<Vec<(AccountId, AccountId)>> {
        VALIDATORS.with(|l| {
            l.borrow_mut()
                .take()
                .map(|validators| validators.iter().map(|v| (*v, *v)).collect())
        })
    }
    fn end_session(_: SessionIndex) {}
    fn start_session(_: SessionIndex) {}
}

pub struct StashOf;
impl Convert<AccountId, Option<AccountId>> for StashOf {
    fn convert(controller: AccountId) -> Option<AccountId> {
        Some(controller)
    }
}

pub struct TestHistoricalConvertInto<T: pallet_session::historical::Config>(sp_std::marker::PhantomData<T>);
// type FullIdentificationOf: Convert<Self::ValidatorId, Option<Self::FullIdentification>>;
impl<T: pallet_session::historical::Config> sp_runtime::traits::Convert<T::ValidatorId, Option<T::FullIdentification>>
for TestHistoricalConvertInto<T>
    where
        <T as pallet_session::historical::Config>::FullIdentification: From<<T as pallet_session::Config>::ValidatorId>,
{
    fn convert(a: T::ValidatorId) -> Option<T::FullIdentification> {
        Some(a.into())
    }
}

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
}
parameter_types! {
	pub const Period: u64 = 1;
	pub const Offset: u64 = 0;
}
impl pallet_session::Config for Test {
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, TestSessionManager>;
    type SessionHandler = TestSessionHandler;
    type ValidatorId = AccountId;
    type ValidatorIdOf = TestSessionConvertInto<Self>;
    type Keys = UintAuthorityId;
    type Event = Event;
    // type DisabledValidatorsThreshold = (); // pallet_session::PeriodicSessions<(), ()>;
    type NextSessionRotation = (); //pallet_session::PeriodicSessions<(), ()>;
    type WeightInfo = ();
}

pub struct TestSessionConvertInto<T>(sp_std::marker::PhantomData<T>);
impl<T: pallet_session::Config> sp_runtime::traits::Convert<AccountId, Option<T::ValidatorId>>
for TestSessionConvertInto<T>
    where
        <T as pallet_session::Config>::ValidatorId: From<sp_application_crypto::sr25519::Public>,
{
    fn convert(a: AccountId) -> Option<T::ValidatorId> {
        Some(a.into())
    }
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
    const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[];

    fn on_genesis_session<Ks: sp_runtime::traits::OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

    fn on_new_session<Ks: sp_runtime::traits::OpaqueKeys>(_: bool, _: &[(AccountId, Ks)], _: &[(AccountId, Ks)]) {}
    fn on_disabled(_: u32) {}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    // let mut t = sp_io::TestExternalities::default();
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(AccountId::from_raw([1; 32]), 100000_000000000000)],
    }
        .assimilate_storage(&mut t)
        .unwrap();

    oracle_finance::GenesisConfig::<Test, Instance1> {
        _pt: Default::default(),
    }.assimilate_storage(&mut t)
        .unwrap();

    crate::GenesisConfig::<Test>{
        _phantom: Default::default(),
        request_base: "http://127.0.0.1:5566".as_bytes().to_vec()  ,
        price_allowable_offset: Percent::from_percent(10),
        price_pool_depth: 3u32,
        price_requests: vec![
            // price , key sign, version, fraction_length, request interval.
            (to_test_vec("btc_price"), to_test_vec("btc"), 2u32, 4u32, 1u8),
            (to_test_vec("eth_price"), to_test_vec("eth"), 2u32, 4u32, 2u8),
            (to_test_vec("dot_price"), to_test_vec("dot"), 2u32, 4u32, 3u8),
            (to_test_vec("xrp_price"), to_test_vec("xrp"), 2u32, 4u32, 4u8),
        ],
        authorities:  vec![
            (AccountId::from_raw([1;32]).try_into().unwrap(), get_account_id_from_seed::<AuraId>("hunter1").into()),
            (AccountId::from_raw([2;32]).try_into().unwrap(), get_account_id_from_seed::<AuraId>("hunter2").into()),
            (AccountId::from_raw([3;32]).try_into().unwrap(), get_account_id_from_seed::<AuraId>("hunter3").into()),
            (AccountId::from_raw([4;32]).try_into().unwrap(), get_account_id_from_seed::<AuraId>("hunter4").into()),
        ],
        data_submission_interval: 100u32,
    }
        .assimilate_storage(&mut t)
        .unwrap();

    t.into()
}

/// Generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: sp_core::Public>(seed: &str) -> AccountId
    where
        <Signature as Verify>::Signer: From<<TPublic::Pair as Pair>::Public>,
{
    <Signature as Verify>::Signer::from(get_from_seed::<TPublic>(seed)).into_account()
}

const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: sp_core::Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("{}/{}", PHRASE, seed), None)
        .expect("static values are valid; qed")
        .public()
}

pub fn ares_price_data_from_tuple(
    param: (u64, AccountId, BlockNumber, FractionLength, JsonNumberValue, u64, BlockNumber),
) -> AresPriceData<AccountId, BlockNumber> {
    AresPriceData {
        price: param.0,
        account_id: param.1,
        create_bn: param.2,
        fraction_len: param.3,
        raw_number: param.4,
        timestamp: param.5,
        update_bn: param.6,
    }
}

pub fn init_aura_enging_digest() {
    use sp_consensus_aura::Slot;
    let slot = Slot::from(1);
    let pre_digest = Digest {
        logs: vec![DigestItem::PreRuntime(AURA_ENGINE_ID, slot.encode())],
    };
    System::initialize(&42, &System::parent_hash(), &pre_digest);
}
