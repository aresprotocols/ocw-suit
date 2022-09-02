use frame_support::sp_runtime::KeyTypeId;
pub const ARES_ORACLE: KeyTypeId = KeyTypeId(*b"ares");
// use scale_info::TypeInfo;

pub mod sr25519 {
	mod app_sr25519 {
		// use sp_std::convert::TryFrom;
		use crate::crypto::ARES_ORACLE;
		use sp_application_crypto::{app_crypto, sr25519};
		use sp_std::convert::TryFrom;
		app_crypto!(sr25519, ARES_ORACLE);
	}

	sp_application_crypto::with_pair! {
		/// An `Ares` authority keypair using S/R 25519 as its crypto.
		pub type AuthorityPair = app_sr25519::Pair;
	}

	/// An `Ares` authority signature using S/R 25519 as its crypto.
	pub type AuthoritySignature = app_sr25519::Signature;

	/// An `Ares` authority identifier using S/R 25519 as its crypto.
	pub type AuthorityId = app_sr25519::Public;
}
