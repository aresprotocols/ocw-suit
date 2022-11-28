use crate as ares_reminder;
use sp_core::sr25519::Public;
use frame_support::traits::{ConstU16, ConstU64, GenesisBuild, LockIdentifier};
use frame_support::{parameter_types, PalletId};
use frame_system as system;
use oracle_finance::types::SessionIndex;
use sp_runtime::{BoundedVec, RuntimeAppPublic, testing::Header, traits::{BlakeTwo256, IdentityLookup, Extrinsic as ExtrinsicT}};
use sp_core::{
	sr25519::Signature,
	H256,
};
use sp_keystore::{
	testing::KeyStore,
	{KeystoreExt, SyncCryptoStore},
};
use sp_runtime::app_crypto::Pair;
use sp_runtime::offchain::{OffchainWorkerExt, testing, TransactionPoolExt};
use sp_runtime::offchain::testing::{TestOffchainExt, TestTransactionPoolExt};

use sp_runtime::testing::TestXt;
use sp_runtime::traits::{Get, Convert, IdentifyAccount, Verify};
use sp_std::sync::Arc;
use ares_oracle_provider_support::{FractionLength, IStashAndAuthority, PriceKey, SymbolInfo};
use ares_oracle_provider_support::crypto::sr25519::AuthorityId;
// use sp_runtime::{
// 	testing::{Header, TestXt, UintAuthorityId},
// 	traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
// 	Perbill,
// };

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type ReminderFinanceInstance = oracle_finance::Instance2;

pub type Extrinsic = TestXt<Call, ()>;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;


pub(crate) type Balance = u64;
pub(crate) type BlockNumber = u64;
pub(crate) type AskPeriodNum = u64;
pub(crate) const DOLLARS: u64 = 1_000_000_000_000;

// For testing the module, we construct a mock runtime.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		OracleFinance: oracle_finance::<Instance2>::{Pallet, Call, Storage, Event<T>},
		AresReminder: ares_reminder::{Pallet, Call, Storage, Event<T>},
	}
);

impl system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
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
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	// type AccountData = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
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

pub struct TestSymbolInfo ;
impl SymbolInfo<BlockNumber> for TestSymbolInfo {
	fn price(symbol: &PriceKey) -> Result<(u64, FractionLength, BlockNumber), ()> {
		Ok(
			(23164822300, TestSymbolInfo::fraction(symbol).unwrap(), 50)
		)
	}
	fn fraction(symbol: &PriceKey) -> Option<FractionLength> {
		Some(6)
	}
}

impl ares_reminder::Config for Test {
	type OffchainAppCrypto = ares_oracle::ares_crypto::AresCrypto<AuthorityId>;
	type AuthorityAres = AuthorityId;
	type Event = Event;
	type FinanceInstance = ReminderFinanceInstance;
	type OracleFinanceHandler = OracleFinance;
	type PriceProvider = TestSymbolInfo;
	type RequestOrigin = frame_system::EnsureRoot<AccountId>;
	type StashAndAuthorityPort = TestAresAuthority;
}

parameter_types! {
	pub const AresFinancePalletId: PalletId = PalletId(*b"reminder");
	pub const BasicDollars: Balance = DOLLARS;
	pub const AskPerEra: SessionIndex = 2;
	pub const HistoryDepth: u32 = 2;
    pub const TestFinanceLockIdentifier : LockIdentifier = *b"rem/fund";
}

impl oracle_finance::Config<ReminderFinanceInstance> for Test {
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

pub struct StashOf;
impl Convert<AccountId, Option<AccountId>> for StashOf {
	fn convert(controller: AccountId) -> Option<AccountId> {
		Some(controller)
	}
}

pub struct TestAresAuthority;

impl IStashAndAuthority<AccountId, AuthorityId> for TestAresAuthority {
	fn get_auth_id(stash: &AccountId) -> Option<AuthorityId> {
		let list_of_storage = Self::get_list_of_storage();
		for data in list_of_storage {
			if &data.0 == stash {
				return Some(data.1);
			}
		}
		None
	}

	fn get_stash_id(auth: &AuthorityId) -> Option<AccountId> {
		let list_of_storage = Self::get_list_of_storage();
		for data in list_of_storage {
			if &data.1 == auth {
				return Some(data.0);
			}
		}
		None
	}

	fn get_authority_list_of_local() -> Vec<AuthorityId> {
		let list_of_storage = Self::get_list_of_storage();
		list_of_storage.iter().map(|data|{
			data.1.clone()
		}).collect()
	}

	fn get_list_of_storage() -> Vec<(AccountId, AuthorityId)> {
		vec![
			(AccountId::from_raw([1; 32]).try_into().unwrap(), get_account_id_from_seed::<AuthorityId>("hunter1").into()),
			(AccountId::from_raw([2; 32]).try_into().unwrap(), get_account_id_from_seed::<AuthorityId>("hunter2").into()),
			(AccountId::from_raw([3; 32]).try_into().unwrap(), get_account_id_from_seed::<AuthorityId>("hunter3").into()),
			// (AccountId::from_raw([4; 32]).try_into().unwrap(), get_account_id_from_seed::<AuthorityId>("hunter4").into()),
		]
	}

	fn check_block_author_and_sotre_key_the_same(block_author: &AuthorityId) -> bool {
		false
	}
}

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

// 6000000000100
// 2000000000000

// Build genesis storage according to the mock runtime.
pub fn new_test_ext(offchain: Option<TestOffchainExt>, pool: Option<TestTransactionPoolExt> ) -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	// let (offchain, offchain_state) = TestOffchainExt::new();
	// let (pool, pool_state) = TestTransactionPoolExt::new();



	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(AccountId::from_raw([1; 32]), 10 * DOLLARS + 100),
					   (AccountId::from_raw([2; 32]), 20 * DOLLARS + 100),
					   (AccountId::from_raw([3; 32]), 30 * DOLLARS + 100),
					   (AccountId::from_raw([4; 32]), 40 * DOLLARS + 100),
					   (AccountId::from_raw([5; 32]), 50 * DOLLARS + 100),
					   (AccountId::from_raw([6; 32]), 60 * DOLLARS + 100)
					],
	}
		.assimilate_storage(&mut t)
		.unwrap();

	crate::GenesisConfig::<Test> {
		security_deposit: 1 * DOLLARS,
		max_pending_keep_bn: 60,
		max_waiting_keep_bn: 180,
	}
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));

	if offchain.is_some() && pool.is_some() {
		// keystore
		let keystore = KeyStore::new();
		SyncCryptoStore::sr25519_generate_new(&keystore, AuthorityId::ID, Some(&format!("{}/hunter1", PHRASE))).unwrap();
		SyncCryptoStore::sr25519_generate_new(&keystore, AuthorityId::ID, Some(&format!("{}/hunter2", PHRASE))).unwrap();
		SyncCryptoStore::sr25519_generate_new(&keystore, AuthorityId::ID, Some(&format!("{}/hunter3", PHRASE))).unwrap();
		SyncCryptoStore::sr25519_generate_new(&keystore, AuthorityId::ID, Some(&format!("{}/hunter4", PHRASE))).unwrap();
		SyncCryptoStore::sr25519_generate_new(&keystore, AuthorityId::ID, Some(&format!("{}/hunter5", PHRASE))).unwrap();

		ext.register_extension(OffchainWorkerExt::new(offchain.unwrap()));
		ext.register_extension(TransactionPoolExt::new(pool.unwrap()));
		ext.register_extension(KeystoreExt(Arc::new(keystore)));
	}
	ext
}


// From<Vec<u8>>
// pub fn to_bound_vec<BoundVec: Get<BoundedVec<u8, MaxLen>> + From<Vec<u8>> , MaxLen: Get<u32>>(input: &str) -> BoundVec {
// 	input.as_bytes().to_vec().try_into().unwrap()
// }

pub fn to_bound_vec<MaxLen: Get<u32>>(to_str: &str) -> BoundedVec<u8, MaxLen> {
	to_str.as_bytes().to_vec().try_into().unwrap()
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

pub fn payload_response(state: &mut testing::OffchainState, url: &str) {
	state.expect_request(testing::PendingRequest {
		method: "GET".into(),
		uri: url.into(),
		response: Some(br#"{"status": "OK"}"#.to_vec()),
		sent: true,
		..Default::default()
	});
}