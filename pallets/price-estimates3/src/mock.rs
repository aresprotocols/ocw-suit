use codec::Encode;
use crate as pallet_price_estimates;
use crate::{Admins, EstimatesType, LockedEstimates, MinimumInitReward, MinimumTicketPrice};
use frame_support::traits::{ConstU32, ConstU64, Everything, ExtrinsicCall, FindAuthor, GenesisBuild, Hooks, OnInitialize};
use frame_support::{PalletId, parameter_types};
use frame_support::{assert_noop, assert_ok};
use bound_vec_helper::BoundVecHelper;
use frame_system as system;
use sp_consensus_aura::AURA_ENGINE_ID;
use sp_core::sr25519::Public;
use sp_core::{
	offchain::{
		testing::{self},
		OffchainWorkerExt, TransactionPoolExt,
	},
	sr25519::Signature,
	H256,
};
use sp_runtime::{testing::{Header, TestXt, UintAuthorityId}, traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify}, Perbill, MultiSignature, DigestItem, Digest, Permill};
use ares_oracle::traits::SymbolInfo;
use ares_oracle::types::FractionLength;

use sp_keystore::{
	testing::KeyStore,
	{KeystoreExt, SyncCryptoStore},
};
use ares_oracle::ares_crypto;
use crate::{BoundedVecOfPreparedEstimates, PreparedEstimates};
use crate::types::{BoundedVecOfConfigRange, BoundedVecOfMultiplierOption, EstimatesState, MultiplierOption, SymbolEstimatesConfig};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
// pub(crate) type Signature = MultiSignature;
pub(crate) type Balance = u64;
pub(crate) type BlockNumber = u64;
pub const DOLLARS: Balance = 1_000_000_000_000;
pub(crate) const TestPalletId: PalletId = PalletId(*b"py/arest");

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		Balances: pallet_balances ,
		System: frame_system ,
		Estimates: pallet_price_estimates,
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
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

pub(crate) type Extrinsic = TestXt<Call, ()>;
pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

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

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(AccountId::from_raw([1; 32]), 1000000000100),
			(AccountId::from_raw([2; 32]), 2000000000100),
			(AccountId::from_raw([3; 32]), 3000000000100),
			(AccountId::from_raw([4; 32]), 4000000000100),
			(AccountId::from_raw([5; 32]), 5000000000100),
			(AccountId::from_raw([6; 32]), 6000000000100)
		],
	}.assimilate_storage(&mut t)
	.unwrap();

	pallet_price_estimates::GenesisConfig::<Test> {
		admins: vec![AccountId::from_raw([1; 32])],
		// white_list: vec![AccountId::from_raw([2; 32])],
		locked_estimates: 2,
		minimum_ticket_price: 100,
		minimum_init_reward: 100,
	}.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

parameter_types! {
	pub const MinimumDeposit: Balance = 100 * DOLLARS ;
	pub const BidderMinimumDeposit: Balance = 1000 * DOLLARS ;
	pub const EstimatesPalletId: PalletId = PalletId(*b"py/arest");
	pub const EstimatesPerSymbol: u32 = 1;
	pub const MaxQuotationDelay: BlockNumber = 20;
	pub const MaxEndDelay: BlockNumber = 10;
	pub const UnsignedPriority: u64 = 1 << 20;
}

pub(crate) type AresId = ares_oracle_provider_support::crypto::sr25519::AuthorityId;

impl pallet_price_estimates::Config for Test {
	type Event = Event;
	type PalletId = EstimatesPalletId;
	type OffchainAppCrypto = ares_oracle::ares_crypto::AresCrypto<AresId>;
	type MaxEstimatesPerSymbol = EstimatesPerSymbol;
	type Currency = Balances;
	type Call = Call;
	type UnsignedPriority = UnsignedPriority;
	type PriceProvider = TestSymbolInfo;
	type MaxEndDelay = MaxEndDelay;
	// type AuthorityId = ares_oracle::ares_crypto::AresCrypto<AresId>;
	type MaxQuotationDelay = MaxQuotationDelay;
}

pub struct TestSymbolInfo ;

impl SymbolInfo<BlockNumber> for TestSymbolInfo {
	fn price(symbol: &Vec<u8>) -> Result<(u64, FractionLength, BlockNumber), ()> {

		Ok(
			(23164822300, TestSymbolInfo::fraction(symbol).unwrap(), 50)
		)
	}
	fn fraction(symbol: &Vec<u8>) -> Option<FractionLength> {
		Some(6)
	}
}


pub(crate) fn run_to_block(n: BlockNumber) {
	println!("System::block_number() {:?} < n {:?}", System::block_number(), n);
	assert!(System::block_number() < n);
	Estimates::on_finalize(System::block_number());
	for b in (System::block_number() + 1)..=n {
		System::set_block_number(b);
		<Estimates as Hooks<u64>>::on_initialize(b);
		<Estimates as Hooks<u64>>::offchain_worker(b);
		if b != n {
			Estimates::on_finalize(System::block_number());
		}
	}
}

pub(crate) fn helper_create_new_estimates_with_deviation(
	init_block: BlockNumber,
	deviation: Permill,
	init_reward: Balance,
	price: Balance,
) {
	run_to_block(init_block);

	assert!(Estimates::is_active());

	// Get configuration informations form storage.
	let admins = Admins::<Test>::get();
	// let white_list = Whitelist::<Test>::get();
	// let locked_estimates =LockedEstimates::<Test>::get();
	// let min_ticket_price = MinimumTicketPrice::<Test>::get();
	// let min_init_reward = MinimumInitReward::<Test>::get();

	assert_eq!(Balances::free_balance(&admins[0]), 1000000000100);

	let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
	let start: BlockNumber = init_block+5; //     start: T::BlockNumber,
	let end: BlockNumber = init_block+10; //     end: T::BlockNumber,
	let distribute: BlockNumber = init_block+15; //     distribute: T::BlockNumber,
	let estimates_type =  EstimatesType::DEVIATION; //     estimates_type: EstimatesType,
	let deviation = Some(deviation); // Some(Permill::from_percent(10)); //     deviation: Option<Permill>,
	let range: Option<Vec<u64>>=None; //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
	let multiplier: Vec<MultiplierOption> = vec![
		MultiplierOption::Base(1),
		MultiplierOption::Base(3),
		MultiplierOption::Base(5),
	]; //     multiplier: Vec<MultiplierOption>,
	// let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
	// let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

	println!("start={:?}, end={:?}, distribute={:?}, estimates_type={:?}", start, end, distribute, estimates_type);

	// Create new estimate.
	assert_ok!(Estimates::new_estimates(
            Origin::signed(admins[0].clone()),
            symbol.clone(),
            start.clone(),
            end.clone(),
            distribute.clone(),
            estimates_type.clone(),
            deviation,
            range.clone(),
			None,
            multiplier.clone(),
            init_reward,
            price.clone(),
        ));

	// Check estimate.
	let estimate = PreparedEstimates::<Test>::get(
		BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
	);
	assert_eq!(estimate, Some(
		SymbolEstimatesConfig{
			symbol: BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone()),
			estimates_type,
			id: 0,
			ticket_price: price,
			symbol_completed_price: 0,
			symbol_fraction: TestSymbolInfo::fraction(&symbol.clone()).unwrap(),
			start,
			end,
			distribute,
			multiplier: BoundedVecOfMultiplierOption::create_on_vec(multiplier),
			deviation,
			range: None, //BoundedVecOfConfigRange::create_on_vec(range),
			total_reward: init_reward,
			state: EstimatesState::InActive,
		}
	));
}

pub(crate) fn helper_create_new_estimates_with_range(
	init_block: BlockNumber,
	// deviation: Permill,
	range: Vec<u64>,
	range_fraction_length: u32,
	init_reward: Balance,
	price: Balance,
) {
	run_to_block(init_block);

	assert!(Estimates::is_active());

	// Get configuration informations form storage.
	let admins = Admins::<Test>::get();
	// let white_list = Whitelist::<Test>::get();
	// let locked_estimates =LockedEstimates::<Test>::get();
	// let min_ticket_price = MinimumTicketPrice::<Test>::get();
	// let min_init_reward = MinimumInitReward::<Test>::get();

	assert_eq!(Balances::free_balance(&admins[0]), 1000000000100);

	let symbol = "btc-usdt".as_bytes().to_vec(); //     symbol: Vec<u8>,
	let start: BlockNumber = init_block+5; //     start: T::BlockNumber,
	let end: BlockNumber = init_block+10; //     end: T::BlockNumber,
	let distribute: BlockNumber = init_block+15; //     distribute: T::BlockNumber,
	let estimates_type =  EstimatesType::RANGE; //     estimates_type: EstimatesType,
	let deviation = None; // Some(Permill::from_percent(10)); //     deviation: Option<Permill>,
	// let mut _range_vec: Vec<u64> = vec![];
	// _range_vec.push(21481_3055u64);
	// _range_vec.push(23481_3055u64);
	// _range_vec.push(27481_3055u64);
	// _range_vec.push(29481_3055u64);
	// let range = Some(_range_vec); //     range: Option<Vec<u64>>, // [{ 'Base': 1 }, { 'Base': 3 }, { 'Base': 5 }]
	// let range = Some(vec![21481_3055u64, 23481_3055u64, 27481_3055u64, 29481_3055u64]);
	let range = Some(range);
	let multiplier: Vec<MultiplierOption> = vec![
		MultiplierOption::Base(1),
		MultiplierOption::Base(3),
		MultiplierOption::Base(5),
	]; //     multiplier: Vec<MultiplierOption>,
	// let init_reward: BalanceOf<Test> = 1000; //     #[pallet::compact] init_reward: BalanceOf<T>,
	// let price: BalanceOf<Test> = 500; //     #[pallet::compact] price: BalanceOf<T>,

	println!("start={:?}, end={:?}, distribute={:?}, estimates_type={:?}, range={:?}", start, end, distribute, estimates_type, range);

	// Create new estimate.
	assert_ok!(Estimates::new_estimates(
            Origin::signed(admins[0].clone()),
            symbol.clone(),
            start.clone(),
            end.clone(),
            distribute.clone(),
            estimates_type.clone(),
            deviation,
            range.clone(),
			Some(range_fraction_length),
            multiplier.clone(),
            init_reward,
            price.clone(),
        ));

	// Check estimate.
	let estimate = PreparedEstimates::<Test>::get(
		BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone())
	);
	// assert_eq!(estimate, Some(
	// 	SymbolEstimatesConfig{
	// 		symbol: BoundedVecOfPreparedEstimates::create_on_vec(symbol.clone()),
	// 		estimates_type,
	// 		id: 0,
	// 		ticket_price: price,
	// 		symbol_completed_price: 0,
	// 		symbol_fraction: TestSymbolInfo::fraction(&symbol.clone()).unwrap(),
	// 		start,
	// 		end,
	// 		distribute,
	// 		multiplier: BoundedVecOfMultiplierOption::create_on_vec(multiplier),
	// 		deviation,
	// 		range: Some(BoundedVecOfConfigRange::create_on_vec(range.unwrap())),
	// 		total_reward: init_reward,
	// 		state: EstimatesState::InActive,
	// 	}
	// ));
}

