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


use frame_support::storage_alias;
use frame_support::{Blake2_128Concat, StorageValue};
use frame_support::storage::types::ValueQuery;
use frame_support::traits::{OnRuntimeUpgrade, StorageInstance};
use sp_runtime::traits::Zero;
use ares_oracle_provider_support::{OrderIdEnum, PurchaseId};
use crate::{AskEraPayment, AskEraPoint, Pallet, PaymentTrace, RewardEra, StorageVersion};
use crate::types::{AskPointNum, BalanceOf, EraIndex, MaximumRewardEras, PaidValue, Releases};
use scale_info::prelude::vec::Vec;
use sp_runtime::BoundedVec;
use bound_vec_helper::BoundVecHelper;

pub struct PrefixOfPaymentTrace;
impl StorageInstance for PrefixOfPaymentTrace {
	fn pallet_prefix() -> &'static str {
		"OracleFinance"
	}
	const STORAGE_PREFIX: &'static str = "PaymentTrace";
}

pub type OldPaymentTraceV0<T, I> = frame_support::storage::types::StorageDoubleMap<
	PrefixOfPaymentTrace,
	Blake2_128Concat,
	PurchaseId,
	Blake2_128Concat,
	<T as frame_system::Config>::AccountId,
	PaidValue<<T as frame_system::Config>::BlockNumber, BalanceOf<T, I>, EraIndex>,
	ValueQuery,
>;


pub struct PrefixOfAskEraPayment;
impl StorageInstance for PrefixOfAskEraPayment {
	fn pallet_prefix() -> &'static str {
		"OracleFinance"
	}
	const STORAGE_PREFIX: &'static str = "AskEraPayment";
}

pub type OldAskEraPaymentV0<T, I> = frame_support::storage::types::StorageDoubleMap<
	PrefixOfAskEraPayment,
	Blake2_128Concat,
	EraIndex, // ,
	Blake2_128Concat,
	(<T as frame_system::Config>::AccountId, PurchaseId), // pay or pay to account. ,,
	BalanceOf<T, I>,
	ValueQuery,
>;

// AskEraPoint
pub struct PrefixOfAskEraPoint;
impl StorageInstance for PrefixOfAskEraPoint {
	fn pallet_prefix() -> &'static str {
		"OracleFinance"
	}
	const STORAGE_PREFIX: &'static str = "AskEraPoint";
}

pub type OldAskEraPointV0<T> = frame_support::storage::types::StorageDoubleMap<
	PrefixOfAskEraPoint,
	Blake2_128Concat,
	EraIndex, //,
	Blake2_128Concat,
	(<T as frame_system::Config>::AccountId, PurchaseId), // pay or pay to account. ,,
	AskPointNum,
	ValueQuery,
>;

// RewardEra, Fro test.
pub struct PrefixOfRewardEra;
impl StorageInstance for PrefixOfRewardEra {
	fn pallet_prefix() -> &'static str {
		"OracleFinance"
	}
	const STORAGE_PREFIX: &'static str = "RewardEra";
}

pub type OldRewardEraV0<T> = frame_support::storage::types::StorageMap<
	PrefixOfRewardEra,
	Blake2_128Concat,
	<T as frame_system::Config>::AccountId,                                    //,
	BoundedVec<(EraIndex, AskPointNum, PurchaseId), MaximumRewardEras>, // pay or pay to account. ,
	ValueQuery,
>;

pub struct UpdateToV1<T: crate::Config<I>, I: 'static = ()> (sp_std::marker::PhantomData<(T, I)>);
impl<T: crate::Config<I>, I: 'static> OnRuntimeUpgrade for UpdateToV1<T, I> {


	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		if StorageVersion::<T, I>::get() == None || StorageVersion::<T, I>::get() == Some(Releases::V0) {


			// OldPaymentTrace::<T,I>::iter()
			log::info!("OracleFinance::<T, I>::migration() ############## 222");

			// let payment_count = OldPaymentTraceV0::<T, I>::iter().count();
			// println!("payment_count = {:?}", payment_count);

			// // let mut trace_data = Vec::new();
			// OldPaymentTraceV0::<T, I>::iter().for_each(|(k1,k2, v)|{
			// 	// log::info!("Only read OracleFinance::<T, I>::migration(), {:?}, {:?}, {:?}", &k1, &k2, &v);
			// 	println!("Write, OracleFinance::<T, I>::migration(), {:?}, {:?}, {:?}", &k1, &k2, &v);
			// 	// trace_data.push((k1,k2,v));
			// 	// PaymentTrace::<T, I>::insert(OrderIdEnum::String(k1), k2, v);
			// });

			log::info!("While, PaymentTrace");
			let mut payment_trace = Vec::new();
			OldPaymentTraceV0::<T, I>::translate::<
				PaidValue<<T as frame_system::Config>::BlockNumber, BalanceOf<T, I>, EraIndex>, _>(
				|k1, k2, old_value| {
					// let raw_key1 = PurchaseId::create_on_vec( k1.to_vec());
					log::info!("Write, PaymentTrace::<T, I>::migration(), {:?}, {:?}, {:?}", &k1, &k2, &old_value);
					// PaymentTrace::<T, I>::insert(OrderIdEnum::String(raw_key1), k2, old_value);
					payment_trace.push((OrderIdEnum::String(k1), k2, old_value));
					None
				}
			);
			payment_trace.iter().for_each(|x|{
				PaymentTrace::<T, I>::insert(x.0.clone(), x.1.clone(), x.2.clone());
			});


			log::info!("While, AskEraPayment");
			let mut ask_era_payment = Vec::new();
			OldAskEraPaymentV0::<T, I>::translate::<BalanceOf<T, I>, _>(
				|k1, k2, old_value| {
					// let raw_key2_1 = PurchaseId::create_on_vec( k2.1.to_vec());
					log::info!("Write, AskEraPayment::<T, I>::migration(), {:?}, {:?}, {:?}", &k1, &k2, &old_value);
					// AskEraPayment::<T, I>::insert(k1, (k2.0, OrderIdEnum::String(raw_key2_1)), old_value);
					ask_era_payment.push((k1, (k2.0, OrderIdEnum::String(k2.1)), old_value));
					None
				}
			);
			ask_era_payment.iter().for_each(|x|{
				AskEraPayment::<T, I>::insert(x.0.clone(), x.1.clone(), x.2.clone());
			});


			log::info!("While, AskEraPoint");
			let mut ask_era_point = Vec::new();
			OldAskEraPointV0::<T>::translate::<AskPointNum, _>(
				|k1, k2, old_value| {
					// let raw_key2_1 = PurchaseId::create_on_vec( k2.1.to_vec());
					log::info!("Write, AskEraPoint::<T, I>::migration(), {:?}, {:?}, {:?}", &k1, &k2, &old_value);
					// AskEraPoint::<T, I>::insert(k1, (k2.0, OrderIdEnum::String(raw_key2_1)), old_value);
					ask_era_point.push((k1, (k2.0, OrderIdEnum::String(k2.1)), old_value));
					None
				}
			);
			ask_era_point.iter().for_each(|x|{
				AskEraPoint::<T, I>::insert(x.0.clone(), x.1.clone(), x.2.clone());
			});


			log::info!("While, RewardEra");
			RewardEra::<T, I>::translate::<BoundedVec<(EraIndex, AskPointNum, PurchaseId), MaximumRewardEras>, _>(
				|k, old_value| {
					log::info!("Write, RewardEra::<T, I>::migration(), {:?}, {:?}", &k, &old_value);
					let mut new_value: BoundedVec<(EraIndex, AskPointNum, OrderIdEnum), MaximumRewardEras> = Default::default();
					old_value.iter().for_each(|data|{
						new_value.try_push((data.0.clone(), data.1.clone(), OrderIdEnum::String(data.2.clone())));
					});
					Some(new_value)
				}
			);

			StorageVersion::<T, I>::put(Releases::V1);
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
