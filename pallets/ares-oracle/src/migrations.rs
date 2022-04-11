use super::*;
use oracle_finance::types::PurchaseId;
// PurchasedDefaultSetting
// pub mod v1_2_0 {
// 	use super::*;
// 	use frame_support::storage::types::{StorageDoubleMap, ValueQuery};
// 	use frame_support::traits::StorageInstance;
// 	use frame_support::{generate_storage_alias, traits::Get, weights::Weight, Blake2_128Concat};
//
// 	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
// 	pub struct OldPurchasedDefaultData {
// 		pub submit_threshold: u8,
// 		pub max_duration: u64,
// 		pub avg_keep_duration: u64,
// 		pub unit_price: u64,
// 	}
//
// 	pub struct OldPurchasedOrderPoolPrefix<T>(sp_std::marker::PhantomData<T>);
// 	impl<T: Config> StorageInstance for OldPurchasedOrderPoolPrefix<T> {
// 		fn pallet_prefix() -> &'static str {
// 			"AresOracle"
// 		}
// 		const STORAGE_PREFIX: &'static str = "PurchasedOrderPoolPrefix";
// 	}
//
// 	//
// 	// generate_storage_alias!(AresOracle, OldPurchasedOrderPool<T: Config> => Value<T::BlockNumber>);
//
// 	/// check to execute prior to migration.
// 	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
// 		Ok(())
// 	}
//
// 	/// Migrate storage to v1_2_0.
// 	pub fn migrate<T: Config>() -> Weight {
// 		log::info!("Migrating ares-oracle to Releases::v1_2_0.");
// 		let mut write_count: Weight = 0;
// 		let mut read_count: Weight = 0;
// 		// AresPrice
// 		let _ = AresPrice::<T>::translate::<Vec<AresPriceData<T::AuthorityAres, T::BlockNumber>>, _>(
// 			|maybe_old_key, maybe_old_val| {
// 				log::info!(
// 					target: "runtime::ares-oracle",
// 					"migrated AresPrice. update AresPriceData()",
// 				);
// 				read_count += 1;
// 				write_count += 1;
// 				let new_data = maybe_old_val
// 					.into_iter()
// 					.map(|c| AresPriceData {
// 						price: c.price,
// 						account_id: crate::Pallet::<T>::get_stash_id_or_default(&c.account_id),
// 						create_bn: c.create_bn,
// 						fraction_len: c.fraction_len,
// 						raw_number: c.raw_number,
// 						timestamp: c.timestamp,
// 					})
// 					.collect::<Vec<_>>();
// 				Some(new_data)
// 			},
// 		);
//
// 		// LastPriceAuthor
// 		let _ = LastPriceAuthor::<T>::translate::<T::AuthorityAres, _>(|maybe_old_key, maybe_old_val| {
// 			log::info!(
// 				target: "runtime::ares-oracle",
// 				"migrated LastPriceAuthor. Authority to Account",
// 			);
// 			read_count += 1;
// 			write_count += 1;
// 			Some(crate::Pallet::<T>::get_stash_id_or_default(&maybe_old_val))
// 		});
//
// 		pub(super) type OldPurchasedOrderPool<T: Config> = StorageDoubleMap<
// 			OldPurchasedOrderPoolPrefix<T>,
// 			Blake2_128Concat,
// 			PurchaseId, // purchased_id,
// 			Blake2_128Concat,
// 			T::AuthorityAres,
// 			T::BlockNumber,
// 			ValueQuery,
// 		>;
// 		OldPurchasedOrderPool::<T>::iter().map(|(k1, k2, val)| {
// 			// PurchasedOrderPool::<T>::swap(k1.clone(), k2.clone(), k1,
// 			// crate::Pallet::<T>::get_stash_id_or_default(&k2));
// 			log::info!(
// 				target: "runtime::ares-oracle",
// 				"migrated PurchasedOrderPool. k2:Authority to k2:Account : {:?} to {:?}",
// 				&k2, crate::Pallet::<T>::get_stash_id_or_default(&k2)
// 			);
// 			read_count += 1;
// 			write_count += 1;
// 			PurchasedOrderPool::<T>::insert(k1, crate::Pallet::<T>::get_stash_id_or_default(&k2), val);
// 		});
//
// 		// PurchasedPricePool
// 		let _ = PurchasedPricePool::<T>::translate::<Vec<AresPriceData<T::AuthorityAres, T::BlockNumber>>, _>(
// 			|_key1, _key2_, maybe_old_val| {
// 				log::info!(
// 					target: "runtime::ares-oracle",
// 					"migrated PurchasedPricePool. Update AresPriceData()",
// 				);
// 				read_count += 1;
// 				write_count += 1;
// 				let new_data = maybe_old_val
// 					.into_iter()
// 					.map(|c| {
// 						// Vec<AresPriceData<T::AccountId, T::BlockNumber>>
// 						AresPriceData {
// 							price: c.price,
// 							account_id: crate::Pallet::<T>::get_stash_id_or_default(&c.account_id),
// 							create_bn: c.create_bn,
// 							fraction_len: c.fraction_len,
// 							raw_number: c.raw_number,
// 							timestamp: c.timestamp,
// 						}
// 					})
// 					.collect::<Vec<_>>();
//
// 				Some(new_data)
// 			},
// 		);
//
// 		let _ = <PurchasedDefaultSetting<T>>::translate::<OldPurchasedDefaultData, _>(|maybe_old_data| {
// 			read_count += 1;
// 			write_count += 1;
// 			maybe_old_data.map(|old_data| {
// 				log::info!(
// 					target: "runtime::ares-oracle",
// 					"migrated PurchasedDefaultData remove unit_price field.",
// 				);
// 				PurchasedDefaultData {
// 					submit_threshold: old_data.submit_threshold,
// 					max_duration: old_data.max_duration,
// 					avg_keep_duration: old_data.avg_keep_duration,
// 				}
// 			})
// 		});
//
// 		// AresAbnormalPrice
// 		let _ = AresAbnormalPrice::<T>::translate::<Vec<AresPriceData<T::AuthorityAres, T::BlockNumber>>, _>(
// 			|maybe_old_key, maybe_old_val| {
// 				log::info!(
// 					target: "runtime::ares-oracle",
// 					"migrated AresAbnormalPrice. update AresAbnormalPrice()",
// 				);
// 				read_count += 1;
// 				write_count += 1;
// 				let new_data = maybe_old_val
// 					.into_iter()
// 					.map(|c| {
// 						// let price_data: AresPriceData<T::AuthorityAres, T::BlockNumber> = c;
// 						// crate::Pallet::<T>::get_stash_id()
// 						let new_data = AresPriceData {
// 							price: c.price,
// 							account_id: crate::Pallet::<T>::get_stash_id_or_default(&c.account_id),
// 							create_bn: c.create_bn,
// 							fraction_len: c.fraction_len,
// 							raw_number: c.raw_number,
// 							timestamp: c.timestamp,
// 						};
// 						(
// 							new_data,
// 							AvgPriceData {
// 								integer: 0,
// 								fraction_len: 0,
// 							},
// 						)
// 					})
// 					.collect::<Vec<_>>();
//
// 				Some(new_data)
// 			},
// 		);
//
// 		// HttpErrTraceLogV1
// 		let _ = <HttpErrTraceLogV1<T>>::translate::<Vec<HttpErrTraceData<T::BlockNumber, T::AuthorityAres>>, _>(
// 			|maybe_old_trace_log| {
// 				read_count += 1;
// 				write_count += 1;
//
// 				maybe_old_trace_log.map(|old_trace_log| {
// 					log::info!(
// 						target: "runtime::ares-oracle",
// 						"migrated {} HttpErrTraceLogV1 update HttpErrTraceData()",
// 						old_trace_log.len(),
// 					);
// 					old_trace_log
// 						.into_iter()
// 						.map(|c| HttpErrTraceData {
// 							block_number: c.block_number,
// 							err_auth: crate::Pallet::<T>::get_stash_id_or_default(&c.err_auth),
// 							err_status: c.err_status,
// 							tip: c.tip,
// 						})
// 						.collect::<Vec<_>>()
// 				})
// 			},
// 		);
//
// 		// Update version.
// 		StorageVersion::<T>::put(Releases::V1_2_0);
//
// 		log::info!("Migrating ares-oracle done.");
// 		T::DbWeight::get().writes(write_count + read_count)
// 	}
// }

// pub mod v2 {
// 	use super::*;
// 	use frame_support::{generate_storage_alias, traits::Get, weights::Weight};
//
// 	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
// 	pub struct OldHttpErrTraceData<BlockNumber, AuthorityId> {
// 		pub block_number: BlockNumber,
// 		// pub request_list: Vec<(Vec<u8>, Vec<u8>, u32)>,
// 		pub err_auth: AuthorityId,
// 		pub err_status: OldHttpError,
// 		pub tip: Vec<u8>,
// 	}
// 	//
// 	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
// 	pub enum OldHttpError {
// 		IoErr(),
// 		TimeOut(),
// 		StatusErr(u16),
// 		ParseErr(),
// 	}
//
//
// 	// Vec<HttpErrTraceData<T::BlockNumber, T::AuthorityAres>>,
// 	// generate_storage_alias!(AresOracle, HttpErrTraceLogV1<T: Config> =>
// Value<Vec<HttpErrTraceData<T::BlockNumber, T::AuthorityAres>>>);
//
// 	/// check to execute prior to migration.
// 	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
// 		Ok(())
// 	}
//
// 	/// Migrate storage to v2.
// 	pub fn migrate<T: Config>() -> Weight {
// 		log::info!("Migrating ares-oracle to Releases::V1_1_0_HttpErrUpgrade.");
//
// 		let _ = <HttpErrTraceLogV1<T>>::translate::<Vec<OldHttpErrTraceData<T::BlockNumber,
// T::AuthorityAres>>, _>(|maybe_old_trace_log| { 			maybe_old_trace_log.map(|old_trace_log| {
// 				log::info!(
// 					target: "runtime::ares-oracle",
// 					"migrated {} err-trace-logs.",
// 					old_trace_log.len(),
// 				);
// 				old_trace_log.into_iter().map(
// 					|c| {
// 						let new_status = |x: OldHttpError| {
// 							let old_request = "Old value is not".as_bytes().to_vec();
// 							match x {
// 								OldHttpError::IoErr() => {
// 									HttpError::IoErr(old_request.clone())
// 								}
// 								OldHttpError::TimeOut() => {
// 									HttpError::TimeOut(old_request.clone())
// 								}
// 								OldHttpError::StatusErr(code) => {
// 									HttpError::StatusErr(old_request.clone(), code)
// 								}
// 								OldHttpError::ParseErr() => {
// 									HttpError::ParseErr(old_request.clone())
// 								}
// 							}
// 						};
//
// 						HttpErrTraceData {
// 							block_number: c.block_number,
// 							err_auth: c.err_auth,
// 							err_status: new_status(c.err_status),
// 							tip: c.tip,
// 						}
// 					}
// 				).collect::<Vec<_>>()
// 			})
// 		});
//
// 		// Update version.
// 		StorageVersion::<T>::put(Releases::V1_1_0_HttpErrUpgrade);
//
// 		log::info!("Migrating ares-oracle done.");
// 		T::DbWeight::get().writes(4 + 5)
// 	}
// }
//
// pub mod v1 {
// 	use super::*;
// 	use frame_support::{generate_storage_alias, traits::Get, weights::Weight};
//
// 	generate_storage_alias!(AresOracle, HttpErrTraceLog => Value<()>);
//
// 	/// check to execute prior to migration.
// 	pub fn pre_migrate<T: Config>() -> Result<(), &'static str> {
// 		Ok(())
// 	}
//
// 	/// Migrate storage to v1.
// 	pub fn migrate<T: Config>() -> Weight {
// 		log::info!("Migrating ares-oracle to Releases::V1_0_1_HttpErrUpgrade.");
//
// 		if ConfPreCheckTokenList::<T>::get().len() == 0 {
// 			log::debug!("Set per check default value.");
// 			ConfPreCheckAllowableOffset::<T>::put(Percent::from_percent(10));
// 			let session_multi: T::BlockNumber= 2u32.into();
// 			ConfPreCheckSessionMulti::<T>::put(session_multi);
//
// 			let mut token_list = Vec::new();
// 			token_list.push("btc-usdt".as_bytes().to_vec());
// 			token_list.push("eth-usdt".as_bytes().to_vec());
// 			token_list.push("dot-usdt".as_bytes().to_vec());
// 			ConfPreCheckTokenList::<T>::put(token_list);
// 		}
//
// 		// update err log struct.
// 		HttpErrTraceLog::kill();
// 		StorageVersion::<T>::put(Releases::V1_0_1_HttpErrUpgrade);
//
// 		log::info!("Migrating ares-oracle done.");
// 		T::DbWeight::get().writes(4 + 5)
// 	}
// }
