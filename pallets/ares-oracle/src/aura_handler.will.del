#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
use crate::pallet::*;
use crate::ValidatorCount;
use frame_support::pallet_prelude::PhantomData;
// use frame_support::traits::FindAuthor;

/// Config necessary for the historical module.
pub trait Config: pallet_aura::Config {}

pub struct Pallet<T>(PhantomData<T>);

impl<T: Config> ValidatorCount for Pallet<T> {
	fn get_validators_count() -> u64 {
		<pallet_aura::Pallet<T>>::authorities().len() as u64
	}
}
