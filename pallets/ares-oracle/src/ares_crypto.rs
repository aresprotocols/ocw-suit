
use frame_support::pallet_prelude::{PhantomData};
use frame_support::sp_runtime::{MultiSignature, MultiSigner, Percent};
use frame_support::sp_runtime::traits::{Verify};
use sp_core::sr25519::Signature as Sr25519Signature;
use sp_runtime::RuntimeAppPublic;

pub struct AresCrypto<AresPublic>(PhantomData<AresPublic>);

impl<AresPublic: RuntimeAppPublic> frame_system::offchain::AppCrypto<MultiSigner, MultiSignature>
for AresCrypto<AresPublic>
    where
        sp_application_crypto::sr25519::Signature: From<<AresPublic as sp_runtime::RuntimeAppPublic>::Signature>,
        <AresPublic as sp_runtime::RuntimeAppPublic>::Signature: From<sp_application_crypto::sr25519::Signature>,
        sp_application_crypto::sr25519::Public: From<AresPublic>,
        AresPublic: From<sp_application_crypto::sr25519::Public>,
{
    type RuntimeAppPublic = AresPublic;
    type GenericSignature = Sr25519Signature;
    type GenericPublic = sp_core::sr25519::Public;
}

impl<AresPublic: RuntimeAppPublic>
frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature> for AresCrypto<AresPublic>
    where
        sp_application_crypto::sr25519::Signature: From<<AresPublic as sp_runtime::RuntimeAppPublic>::Signature>,
        <AresPublic as sp_runtime::RuntimeAppPublic>::Signature: From<sp_application_crypto::sr25519::Signature>,
        sp_application_crypto::sr25519::Public: From<AresPublic>,
        AresPublic: From<sp_application_crypto::sr25519::Public>,
{
    type RuntimeAppPublic = AresPublic;
    type GenericSignature = Sr25519Signature;
    type GenericPublic = sp_core::sr25519::Public;
}