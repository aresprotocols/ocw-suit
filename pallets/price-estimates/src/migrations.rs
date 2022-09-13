// This file is part of Substrate.

// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The migrations of this pallet.

use codec::{Decode, Encode};
use core::marker::PhantomData;
use frame_support::traits::OnRuntimeUpgrade;
use sp_runtime::traits::Zero;
use crate::types::{AccountParticipateEstimates, BoundedVecOfCompletedEstimates, BoundedVecOfSymbol, EstimatesType, MaximumParticipants, MaximumWinners, SymbolEstimatesConfig};
use crate::{CompletedEstimates, EstimatesInitDeposit, Participants, StorageVersion, TARGET, UnresolvedEstimates, Winners};

#[cfg(feature = "try-runtime")]
use frame_support::ensure;
use frame_support::StorageMap;
use crate::{ActiveEstimates, BalanceOf, PreparedEstimates};
use crate::types::Releases;


pub struct UpdateOfV1<T: crate::Config> (sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for UpdateOfV1<T> {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		if StorageVersion::<T>::get() == None {
			// Goto update.
			// <PreparedEstimates<T>>::translate::<BoundedVecOfSymbol, _>(|old_key, old_val| {
			//
			// });

			log::info!( target: TARGET, "PreparedEstimates::<T>::kill()", );
			PreparedEstimates::<T>::clear(u32::MAX, None);

			log::info!( target: TARGET, "ActiveEstimates::<T>::kill()", );
			ActiveEstimates::<T>::clear(u32::MAX, None);

			log::info!( target: TARGET, "UnresolvedEstimates::<T>::kill()", );
			UnresolvedEstimates::<T>::clear(u32::MAX, None);

			log::info!( target: TARGET, "CompletedEstimates::<T>::kill()", );
			CompletedEstimates::<T>::clear(u32::MAX, None);

			log::info!( target: TARGET, "Participants::<T>::kill()", );
			Participants::<T>::clear(u32::MAX, None);

			log::info!( target: TARGET, "EstimatesInitDeposit::<T>::kill()", );
			EstimatesInitDeposit::<T>::clear(u32::MAX, None);

			log::info!( target: TARGET, "Winners::<T>::kill()", );
			Winners::<T>::clear(u32::MAX, None);

			StorageVersion::<T>::put(Releases::V1);
		}

		frame_support::weights::Weight::zero()
	}
}

// mod old {
// 	use super::*;
// 	use frame_support::pallet_prelude::*;
// 	use crate::{BalanceOf, Config};
// 	use crate::types::{AccountParticipateEstimates, BoundedVecOfCompletedEstimates, BoundedVecOfSymbol, EstimatesType, MaximumParticipants, MaximumWinners, SymbolEstimatesConfig};
//
//
// 	#[pallet::storage]
// 	pub type PreparedEstimates<T: Config> = StorageMap<
// 		_,
// 		Blake2_128Concat,
// 		BoundedVecOfSymbol,
// 		// BoundedVec<u8, StringLimit>,                            // symbol
// 		SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>, // config
// 	>;
//
// 	#[pallet::storage]
// 	pub type ActiveEstimates<T: Config> = StorageMap<
// 		_,
// 		Blake2_128Concat,
// 		BoundedVecOfSymbol,                            // symbol
// 		SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>, // config
// 	>;
//
// 	#[pallet::storage]
// 	pub type UnresolvedEstimates<T: Config> = StorageMap<
// 		_,
// 		Blake2_128Concat,
// 		BoundedVecOfSymbol,                            // symbol
// 		SymbolEstimatesConfig<T::BlockNumber, BalanceOf<T>>, // config
// 	>;
//
// 	#[pallet::storage]
// 	pub type CompletedEstimates<T: Config> = StorageMap<
// 		_,
// 		Blake2_128Concat,
// 		BoundedVecOfSymbol,  // symbol btc-sudt => [0, 1, 3]
// 		BoundedVecOfCompletedEstimates<T::BlockNumber, BalanceOf<T>>, // configs
// 		ValueQuery,
// 	>;
//
// 	#[pallet::storage]
// 	pub type Participants<T: Config> = StorageDoubleMap<
// 		_,
// 		Blake2_128Concat,
// 		BoundedVecOfSymbol,  // symbol
// 		Blake2_128Concat,
// 		u64, // id
// 		BoundedVec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>, MaximumParticipants>,
// 		ValueQuery,
// 	>;
//
// 	#[pallet::storage]
// 	pub type EstimatesInitDeposit<T: Config> = StorageDoubleMap<
// 		_,
// 		Blake2_128Concat,
// 		BoundedVecOfSymbol,
// 		Blake2_128Concat,
// 		u64, // id
// 		BalanceOf<T>,
// 		ValueQuery,
// 	>;
//
// 	#[pallet::storage]
// 	pub type Winners<T: Config> = StorageDoubleMap<
// 		_,
// 		Blake2_128Concat,
// 		BoundedVecOfSymbol, // symbol
// 		Blake2_128Concat,
// 		u64, // id
// 		BoundedVec<AccountParticipateEstimates<T::AccountId, T::BlockNumber>, MaximumWinners>,
// 	>;
// }

// /// A struct that migrates all bags lists to contain a score value.
// pub struct AddScore<T: crate::Config<I>, I: 'static = ()>(sp_std::marker::PhantomData<(T, I)>);
// impl<T: crate::Config<I>, I: 'static> OnRuntimeUpgrade for AddScore<T, I> {
// 	#[cfg(feature = "try-runtime")]
// 	fn pre_upgrade() -> Result<(), &'static str> {
// 		// The list node data should be corrupt at this point, so this is zero.
// 		ensure!(crate::ListNodes::<T, I>::iter().count() == 0, "list node data is not corrupt");
// 		// We can use the helper `old::ListNode` to get the existing data.
// 		let iter_node_count: u32 = old::ListNodes::<T, I>::iter().count() as u32;
// 		let tracked_node_count: u32 = old::CounterForListNodes::<T, I>::get();
// 		crate::log!(info, "number of nodes before: {:?} {:?}", iter_node_count, tracked_node_count);
// 		ensure!(iter_node_count == tracked_node_count, "Node count is wrong.");
// 		old::TempStorage::<T, I>::put(iter_node_count);
// 		Ok(())
// 	}
//
// 	fn on_runtime_upgrade() -> frame_support::weights::Weight {
// 		for (_key, node) in old::ListNodes::<T, I>::iter() {
// 			let score = T::ScoreProvider::score(&node.id);
//
// 			let new_node = crate::Node {
// 				id: node.id.clone(),
// 				prev: node.prev,
// 				next: node.next,
// 				bag_upper: node.bag_upper,
// 				score,
// 				_phantom: node._phantom,
// 			};
//
// 			crate::ListNodes::<T, I>::insert(node.id, new_node);
// 		}
//
// 		return frame_support::weights::Weight::MAX
// 	}
//
// 	#[cfg(feature = "try-runtime")]
// 	fn post_upgrade() -> Result<(), &'static str> {
// 		let node_count_before = old::TempStorage::<T, I>::take();
// 		// Now, the list node data is not corrupt anymore.
// 		let iter_node_count_after: u32 = crate::ListNodes::<T, I>::iter().count() as u32;
// 		let tracked_node_count_after: u32 = crate::ListNodes::<T, I>::count();
// 		crate::log!(
// 			info,
// 			"number of nodes after: {:?} {:?}",
// 			iter_node_count_after,
// 			tracked_node_count_after,
// 		);
// 		ensure!(iter_node_count_after == node_count_before, "Not all nodes were migrated.");
// 		ensure!(tracked_node_count_after == iter_node_count_after, "Node count is wrong.");
// 		Ok(())
// 	}
// }
