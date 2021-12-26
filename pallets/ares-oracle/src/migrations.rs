// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
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

//! Storage migrations for the Staking pallet.

use super::*;

pub mod v1 {
	use super::*;
	use frame_support::{generate_storage_alias, traits::Get, weights::Weight};

	generate_storage_alias!(AresOracle, HttpErrTraceLog => Value<()>);

	/// check to execute prior to migration.
	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
		Ok(())
	}

	/// Migrate storage to v6.
	pub fn migrate<T: Config>() -> Weight {
		log::info!("Migrating ares-oracle to Releases::V1_0_1_HttpErrUpgrade.");

		if ConfPreCheckTokenList::<T>::get().len() == 0 {
			log::debug!("Set per check default value.");
			ConfPreCheckAllowableOffset::<T>::put(Percent::from_percent(10));
			let session_multi: T::BlockNumber= 2u32.into();
			ConfPreCheckSessionMulti::<T>::put(session_multi);

			let mut token_list = Vec::new();
			token_list.push("btc-usdt".as_bytes().to_vec());
			token_list.push("eth-usdt".as_bytes().to_vec());
			token_list.push("dot-usdt".as_bytes().to_vec());
			ConfPreCheckTokenList::<T>::put(token_list);
		}

		// update err log struct.
		HttpErrTraceLog::kill();
		StorageVersion::<T>::put(Releases::V1_0_1_HttpErrUpgrade);

		log::info!("Migrating ares-oracle done.");
		T::DbWeight::get().writes(4 + 5)
	}
}
