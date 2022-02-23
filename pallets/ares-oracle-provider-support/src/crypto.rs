use frame_support::sp_runtime::KeyTypeId;
pub const ARES_ORACLE: KeyTypeId = KeyTypeId(*b"ares");

pub mod sr25519 {
	mod app_sr25519 {
		use crate::crypto::ARES_ORACLE;
		use sp_application_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, ARES_ORACLE);
	}

	sp_application_crypto::with_pair! {
		/// An Aura authority keypair using S/R 25519 as its crypto.
		pub type AuthorityPair = app_sr25519::Pair;
	}

	/// An Aura authority signature using S/R 25519 as its crypto.
	pub type AuthoritySignature = app_sr25519::Signature;

	/// An Aura authority identifier using S/R 25519 as its crypto.
	pub type AuthorityId = app_sr25519::Public;
}
