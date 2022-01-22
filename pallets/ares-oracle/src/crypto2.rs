// use super::sr25519 as AresSr25519;
use sp_consensus_aura::sr25519 as AuraSr25519;
use sp_runtime::{
	// app_crypto::{app_crypto, sr25519},
	// traits::Verify,
	MultiSignature,
	MultiSigner,
};
// /// the types with this pallet-specific identifier.
// use super::KEY_TYPE;
use sp_core::sr25519::{Public as Sr25519Public, Signature as Sr25519Signature};
// use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};
//
//
// // struct fro production
// pub struct OcwAuthId;
// impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for OcwAuthId {
//     type RuntimeAppPublic = AresSr25519::AuthorityId;
//     type GenericSignature = Sr25519Signature;
//     type GenericPublic = Sr25519Public;
// }
//
// impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
//     for OcwAuthId
// {
//     type RuntimeAppPublic = AresSr25519::AuthorityId;
//     type GenericSignature = Sr25519Signature;
//     type GenericPublic = Sr25519Public;
// }
//
pub struct AuraAuthId;
// pub struct BabeAuthId;

impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for AuraAuthId {
	type RuntimeAppPublic = AuraSr25519::AuthorityId;
	type GenericSignature = Sr25519Signature;
	type GenericPublic = Sr25519Public;
}

//
// impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
//     for AuraAuthId
// {
//     type RuntimeAppPublic = AuraSr25519::AuthorityId;
//     type GenericSignature = Sr25519Signature;
//     type GenericPublic = Sr25519Public;
// }
