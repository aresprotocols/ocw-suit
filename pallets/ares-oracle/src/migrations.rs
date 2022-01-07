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

// frame_support::generate_storage_alias!(
// 	PhragmenElection, Candidates<T: V2ToV3> => Value<Vec<(T::AccountId, T::Balance)>>
// );
//
// /// Migrate all candidates to recorded deposit.
// pub fn migrate_candidates_to_recorded_deposit<T: V2ToV3>(old_deposit: T::Balance) {
// 	let _ = <Candidates<T>>::translate::<Vec<T::AccountId>, _>(|maybe_old_candidates| {
// 		maybe_old_candidates.map(|old_candidates| {
// 			log::info!(
// 				target: "runtime::elections-phragmen",
// 				"migrated {} candidate accounts.",
// 				old_candidates.len(),
// 			);
// 			old_candidates.into_iter().map(|c| (c, old_deposit)).collect::<Vec<_>>()
// 		})
// 	});
// }

pub mod v2 {
	use super::*;
	use frame_support::{generate_storage_alias, traits::Get, weights::Weight};

	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
	pub struct OldHttpErrTraceData<BlockNumber, AuthorityId> {
		pub block_number: BlockNumber,
		// pub request_list: Vec<(Vec<u8>, Vec<u8>, u32)>,
		pub err_auth: AuthorityId,
		pub err_status: OldHttpError,
		pub tip: Vec<u8>,
	}
	//
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
	pub enum OldHttpError {
		IoErr(),
		TimeOut(),
		StatusErr(u16),
		ParseErr(),
	}


	// Vec<HttpErrTraceData<T::BlockNumber, T::AuthorityAres>>,
	generate_storage_alias!(AresOracle, HttpErrTraceLogV1<T: Config> => Value<Vec<HttpErrTraceData<T::BlockNumber, T::AuthorityAres>>>);

	/// check to execute prior to migration.
	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
		Ok(())
	}

	/// Migrate storage to v2.
	pub fn migrate<T: Config>() -> Weight {
		log::info!("Migrating ares-oracle to Releases::V1_1_0_HttpErrUpgrade.");


		let _ = <HttpErrTraceLogV1<T>>::translate::<Vec<OldHttpErrTraceData<T::BlockNumber, T::AuthorityAres>>, _>(|maybe_old_trace_log| {
			maybe_old_trace_log.map(|old_trace_log| {
				log::info!(
					target: "runtime::ares-oracle",
					"migrated {} err-trace-logs.",
					old_trace_log.len(),
				);
				old_trace_log.into_iter().map(
					|c| {
						let new_status = |x: OldHttpError| {
							let old_request = "Old value is not".as_bytes().to_vec();
							match x {
								OldHttpError::IoErr() => {
									HttpError::IoErr(old_request.clone())
								}
								OldHttpError::TimeOut() => {
									HttpError::TimeOut(old_request.clone())
								}
								OldHttpError::StatusErr(code) => {
									HttpError::StatusErr(old_request.clone(), code)
								}
								OldHttpError::ParseErr() => {
									HttpError::ParseErr(old_request.clone())
								}
							}
						};

						HttpErrTraceData {
							block_number: c.block_number,
							err_auth: c.err_auth,
							err_status: new_status(c.err_status),
							tip: c.tip,
						}
					}
				).collect::<Vec<_>>()
			})
		});

		// Update version.
		StorageVersion::<T>::put(Releases::V1_1_0_HttpErrUpgrade);

		log::info!("Migrating ares-oracle done.");
		T::DbWeight::get().writes(4 + 5)
	}
}

pub mod v1 {
	use super::*;
	use frame_support::{generate_storage_alias, traits::Get, weights::Weight};

	generate_storage_alias!(AresOracle, HttpErrTraceLog => Value<()>);

	/// check to execute prior to migration.
	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
		Ok(())
	}

	/// Migrate storage to v1.
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
