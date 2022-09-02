#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
pub mod tests {
	use crate::ChallengeValidator;
	use frame_support::traits::ConstU32;
	use frame_support::weights::DispatchClass;
	use frame_support::{assert_ok, PalletId};
	use frame_support::{parameter_types,weights::{RuntimeDbWeight}};
	use frame_system::ChainContext;
	use pallet_collective::EnsureProportionAtLeast;
	use pallet_transaction_payment::CurrencyAdapter;
	use sp_runtime::BuildStorage;
	use sp_runtime::traits::Hash;
	use sp_runtime::{ generic::Era, testing::{Block, Header}, traits::{BlakeTwo256, IdentityLookup}};
	use sp_std::convert::TryFrom;
	use sp_std::convert::TryInto;

	frame_support::construct_runtime!(
		pub enum Runtime where
			Block = TestBlock,
			NodeBlock = TestBlock,
			UncheckedExtrinsic = TestUncheckedExtrinsic
		{
			System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
			// TransactionPayment: pallet_transaction_payment::{Pallet, Storage},
			Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Event<T>, Origin<T>, Config<T>},
			TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
			AresChallenge: crate::<Instance1>::{Pallet, Call, Storage, Event<T>},
		}
	);


	// frame_support::construct_runtime!(
	// 	pub enum Test where
	// 		Block = Block,
	// 		NodeBlock = Block,
	// 		UncheckedExtrinsic = UncheckedExtrinsic,
	// 	{
	// 		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
	// 		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	// 		Aura: pallet_aura::{Pallet, Storage, Config<T>},
	// 	}
	// );

	type AccountId = u64;
	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub BlockWeights: frame_system::limits::BlockWeights =
			frame_system::limits::BlockWeights::builder()
				.base_block(10)
				.for_class(DispatchClass::all(), |weights| weights.base_extrinsic = 5)
				.for_class(DispatchClass::non_mandatory(), |weights| weights.max_total = 1024.into())
				.build_or_panic();
		pub const DbWeight: RuntimeDbWeight = RuntimeDbWeight {
			read: 10,
			write: 100,
		};
	}
	impl frame_system::Config for Runtime {
		type BaseCallFilter = frame_support::traits::Everything;
		type BlockWeights = BlockWeights;
		type BlockLength = ();
		type DbWeight = ();
		type Origin = Origin;
		type Index = u64;
		type Call = Call;
		type BlockNumber = u64;
		type Hash = sp_core::H256;
		type Hashing = BlakeTwo256;
		type AccountId = AccountId;
		type Lookup = IdentityLookup<u64>;
		type Header = Header;
		type Event = Event;
		type BlockHashCount = BlockHashCount;
		type Version = RuntimeVersion;
		type PalletInfo = PalletInfo;
		type AccountData = pallet_balances::AccountData<Balance>;
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
		type SS58Prefix = ();
		type OnSetCode = ();
		type MaxConsumers = ConstU32<16>;
	}

	type Balance = u64;
	parameter_types! {
		pub const ExistentialDeposit: Balance = 1;
		pub const MaxReserves: u32 = 100;
	}
	impl pallet_balances::Config for Runtime {
		type Balance = Balance;
		type Event = Event;
		type DustRemoval = ();
		type ExistentialDeposit = ExistentialDeposit;
		type AccountStore = System;
		type MaxLocks = ();
		type MaxReserves = MaxReserves;
		type ReserveIdentifier = [u8; 8];
		type WeightInfo = ();
	}

	parameter_types! {
		pub const TransactionByteFee: Balance = 0;
		pub const OperationalFeeMultiplier: u8 = 5;
	}

	// impl pallet_transaction_payment::Config for Runtime {
	// 	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	// 	type TransactionByteFee = TransactionByteFee;
	// 	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	// 	type WeightToFee = IdentityFee<Balance>;
	// 	type FeeMultiplierUpdate = ();
	// }

	// impl pallet_transaction_payment::Config for Runtime {
	// 	type Event = Event;
	// 	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	// 	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	// 	type WeightToFee = IdentityFee<Balance>;
	// 	type LengthToFee = IdentityFee<Balance>;
	// 	type FeeMultiplierUpdate = ();
	// }

	// impl pallet_transaction_payment::Config for Test {
	// 	type Event = Event;
	// 	type OnChargeTransaction = CurrencyAdapter<Pallet<Test>, ()>;
	// 	type OperationalFeeMultiplier = ConstU8<5>;
	// 	type WeightToFee = IdentityFee<u64>;
	// 	type LengthToFee = IdentityFee<u64>;
	// 	type FeeMultiplierUpdate = ();
	// }

	parameter_types! {
		pub const MotionDuration: u64 = 3;
		pub const MaxProposals: u32 = 100;
		pub const MaxMembers: u32 = 100;
	}

	type CouncilCollective = pallet_collective::Instance1;

	impl pallet_collective::Config<CouncilCollective> for Runtime {
		type Origin = Origin;
		type Proposal = Call;
		type Event = Event;
		type MotionDuration = MotionDuration;
		type MaxProposals = MaxProposals;
		type MaxMembers = MaxMembers;
		type DefaultVote = pallet_collective::PrimeDefaultVote;
		type WeightInfo = ();
	}

	pub type TechnicalCollective = pallet_collective::Instance2;

	impl pallet_collective::Config<TechnicalCollective> for Runtime {
		type Origin = Origin;
		type Proposal = Call;
		type Event = Event;
		type MotionDuration = MotionDuration;
		type MaxProposals = MaxProposals;
		type MaxMembers = MaxMembers;
		type DefaultVote = pallet_collective::PrimeDefaultVote;
		type WeightInfo = ();
	}

	parameter_types! {
		pub const MinimumDeposit: Balance = 1;
		pub const ChallengePalletId: PalletId = PalletId(*b"py/ardem");
		pub const MinimumThreshold: u32 = 2;
	}

	impl sp_runtime::traits::IsMember<pallet_babe::AuthorityId> for Runtime {
		fn is_member(member_id: &pallet_babe::AuthorityId) -> bool {
			true
		}
	}

	pub type Challenge1 = crate::Instance1;

	impl crate::Config<Challenge1> for Runtime {
		type Event = Event;
		type Proposal = Call;
		type MinimumDeposit = MinimumDeposit;
		type PalletId = ChallengePalletId;
		// type CouncilMajorityOrigin =
		// 	EnsureProportionAtLeast<sp_core::u32_trait::_3, sp_core::u32_trait::_4, AccountId, CouncilCollective>;
		type CouncilMajorityOrigin = EnsureProportionAtLeast<u64, Challenge1, 3, 4>;
		type Currency = Balances;
		type SlashProposer = AresChallenge;
		type IsAuthority = Self;
		type AuthorityId = pallet_babe::AuthorityId;
		type MinimumThreshold = MinimumThreshold;
	}

	pub struct RuntimeVersion;

	impl frame_support::traits::Get<sp_version::RuntimeVersion> for RuntimeVersion {
		fn get() -> sp_version::RuntimeVersion {
			RUNTIME_VERSION.with(|v| v.borrow().clone())
		}
	}

	thread_local! {
		pub static RUNTIME_VERSION: std::cell::RefCell<sp_version::RuntimeVersion> =
			Default::default();
	}

	type SignedExtra = (
		frame_system::CheckEra<Runtime>,
		frame_system::CheckNonce<Runtime>,
		frame_system::CheckWeight<Runtime>,
	);
	type TestXt = sp_runtime::testing::TestXt<Call, SignedExtra>;
	type TestBlock = Block<TestXt>;
	type TestUncheckedExtrinsic = TestXt;

	type Executive =
		frame_executive::Executive<Runtime, Block<TestXt>, ChainContext<Runtime>, Runtime, AllPalletsWithSystem>;

	fn extra(nonce: u64, fee: Balance) -> SignedExtra {
		(
			frame_system::CheckEra::from(Era::Immortal),
			frame_system::CheckNonce::from(nonce),
			frame_system::CheckWeight::new(),
			/* crate::CheckCall::<Challenge1>::new(),
			 * pallet_transaction_payment::ChargeTransactionPayment::from(fee), */
		)
	}

	fn sign_extra(who: u64, nonce: u64, fee: Balance) -> Option<(u64, SignedExtra)> {
		Some((who, extra(nonce, fee)))
	}

	fn make_proposal(delegatee: AccountId, deposit: Balance, validator: AccountId, block_hash: sp_core::H256) -> Call {
		Call::AresChallenge(crate::Call::new_challenge {
			delegatee: 2,
			validator: 2,
			block_hash: block_hash.clone(),
			deposit: 2,
		})
	}

	fn new_test_ext() -> sp_io::TestExternalities {
		// Runtime::GenesisConfig https://docs.substrate.io/rustdocs/latest/node_runtime/struct.GenesisConfig.html
		let mut t: sp_io::TestExternalities = GenesisConfig {
			system: frame_system::GenesisConfig::default(),
			balances: pallet_balances::GenesisConfig {
				balances: vec![(1, 211)],
			},
			council: pallet_collective::GenesisConfig {
				members: vec![3, 4],
				phantom: Default::default(),
			},
			technical_committee: pallet_collective::GenesisConfig {
				members: vec![1],
				phantom: Default::default(),
			},
		}
		.build_storage()
		.unwrap()
		.into();
		return t;
	}

	#[test]
	fn close_challage_success() {
		// let fee: Balance = <Runtime as pallet_transaction_payment::Config>::WeightToFee::calc(&weight);
		// let mut t = sp_io::TestExternalities::new(t);

		new_test_ext().execute_with(|| {
			// Set 1 as prime voter
			pallet_collective::Prime::<Runtime, CouncilCollective>::set(Some(1));
			System::set_block_number(1);
			let block_hash = frame_system::BlockHash::<Runtime>::get(1);

			// let xt = TestXt::new(call, sign_extra(1, 0, 0));
			// let weight = xt.get_dispatch_info().weight
			// 	+ <Runtime as frame_system::Config>::BlockWeights::get()
			// 		.get(DispatchClass::Normal)
			// 		.base_extrinsic;
			frame_support::assert_ok!(AresChallenge::new_challenge(
				Origin::signed(1),
				3,                  // delegatee
				1,                  // validator
				block_hash.clone(), // block_hash
				1,                  // deposit
			));
			System::set_block_number(2);
			let challenge_validator = ChallengeValidator {
				validator: 1 as AccountId,
				block_hash: block_hash.clone(),
			};
			let challenge_validator_hash = <Runtime as frame_system::Config>::Hashing::hash_of(&challenge_validator);
			crate::Proposals::<Runtime, Challenge1>::iter_keys().for_each(|key| {
				println!("{:?}", key);
			});
			use frame_support::traits::OnFinalize;
			// if let Some(challenge_info) = AresChallenge::proposals(&challenge_validator_hash) {
			if let Some(challenge_info) = crate::Proposals::<Runtime, Challenge1>::get(&challenge_validator_hash) {
				println!("challenge_info: {:?}", challenge_info);
				let proposal_hash = challenge_info.proposal;
				// pallet_collective::ProposalOf::<Runtime, CouncilCollective>::iter_keys()
				// 	.for_each(|key| println!("key:{:?}", key));
				let a = pallet_collective::ProposalOf::<Runtime, CouncilCollective>::get(&proposal_hash);
				if a.is_none() {
					assert!(false, "not found")
				}
				assert_ok!(Council::vote(Origin::signed(3), proposal_hash, 0, true));
				assert_ok!(Council::vote(Origin::signed(4), proposal_hash, 0, true));
				System::set_block_number(3);
				assert_ok!(Council::close(Origin::signed(4), proposal_hash, 0, 10000, 100));
				System::set_block_number(100);
				AresChallenge::on_finalize(100);
				crate::Proposals::<Runtime, Challenge1>::iter_keys().for_each(|key| {
					assert!(false, "not empty");
				});
				// assert_eq!(
				// 	<pallet_balances::Pallet<Runtime>>::total_balance(&AresChallenge::account_id()),
				// 	1
				// );
			}
			// println!(
			// 	"test .... challenge_validator {:?}, xxxxx:{:?}",
			// 	&challenge_validator, &challenge_validator_hash
			// );
			// frame_support::assert_ok!();

			// println!("test: {}", r.is_ok());
			// println!("test: {}", r.is_err());

			// assert!();
			// assert_eq!(
			// 	<pallet_collective::ProposalOf<Runtime, CouncilCollective>>::get(hash),
			// 	Some(proposal)
			// );
			// assert_eq!(<pallet_balances::Pallet<Runtime>>::total_balance(&2), 69);
		});
	}
}
