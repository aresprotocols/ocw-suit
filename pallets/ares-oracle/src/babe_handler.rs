
#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
use crate::pallet::*;
use crate::ValidatorCount;
use frame_support::pallet_prelude::{PhantomData, Member};
use frame_support::traits::FindAuthor;
use frame_support::{ConsensusEngineId, Parameter};
use frame_support::sp_runtime::RuntimeAppPublic;
use sp_runtime::traits::MaybeSerializeDeserialize;

/// Config necessary for the historical module.
pub trait Config: pallet_babe::Config
    // where Self::T::AuthorityAres: From<pallet_babe::AuthorityId>
{
    type AuthorityId: Member
    + Parameter
    + RuntimeAppPublic
    + Default
    + MaybeSerializeDeserialize
    + From<pallet_babe::AuthorityId>;
}

pub struct Pallet<T>(PhantomData<T>);

impl <T:Config> ValidatorCount for Pallet<T> {
    fn get_validators_count() -> u64 {
        <pallet_babe::Pallet<T>>::authorities().len() as u64
    }
}

pub struct FindAccountFromAuthorIndex<T, Inner>(sp_std::marker::PhantomData<(T, Inner)>);

impl<T: Config, Inner: FindAuthor<u32>> FindAuthor<T::AuthorityId>
for FindAccountFromAuthorIndex<T, Inner>
{
    fn find_author<'a, I>(digests: I) -> Option<T::AuthorityId>
        where
            I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
            // T::AuthorityId: From<pallet_babe::AuthorityId>
    {
        let i = Inner::find_author(digests)?;

        let validators = <pallet_babe::Pallet<T>>::authorities();
        let validator_opt = validators.get(i as usize).map(|k| k.0.clone());
        if validator_opt.is_none() {
            return None;
        }
        Some(validator_opt.unwrap().into())
    }
}