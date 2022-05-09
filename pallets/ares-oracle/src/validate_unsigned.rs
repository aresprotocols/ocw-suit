use super::*;
//
// #[pallet::validate_unsigned]
// impl<T: Config> ValidateUnsigned for Pallet<T> {
//     type Call = Call<T>;
//     fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
//         if let Call::submit_price_unsigned_with_signed_payload {
//             price_payload: ref payload,
//             signature: ref signature,
//         } = call
//         {
//
//             let mut block_author_trace = BlockAuthorTrace::<T>::get().unwrap_or(Default::default());
//             let mut pop_arr = AuthorTraceData::<T>::default();
//
//             while block_author_trace.len() > 0 && pop_arr.len() < 5 {
//                 pop_arr.try_push(block_author_trace.pop().unwrap());
//             }
//
//             log::debug!(
// 					"🚅 Validate price payload data, on block: {:?}/{:?}, author: {:?} ship auth: {:?}, pop_arr: {:?}",
// 					payload.block_number.clone(), // 236
// 					<system::Pallet<T>>::block_number(), // 237 (next)
// 					payload.public.clone(),
// 					BlockAuthor::<T>::get(), // 237(236)
// 					// block_trace_author, // 236
// 					pop_arr,
// 				);
//
//             if !Self::is_validator_member(&payload.auth) {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on price "
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//
//
//             //TODO:: Need to check is block author.
//             // ---
//
//             let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
//             if !signature_valid {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Signature invalid. `InvalidTransaction` on price"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//
//             Self::validate_transaction_parameters_of_ares(&payload.block_number, &payload.auth)
//         } else if let Call::submit_purchased_price_unsigned_with_signed_payload {
//             price_payload: ref payload,
//             ref signature,
//         } = call
//         {
//             log::debug!(
// 					"🚅 Validate purchased price payload data, on block: {:?} ",
// 					<system::Pallet<T>>::block_number()
// 				);
//
//             if !Self::is_validator_member(&payload.auth) {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on purchased price"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//
//             let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
//
//             if !signature_valid {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Signature invalid. `InvalidTransaction` on purchased price"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//
//             let priority_num: u64 = T::UnsignedPriority::get();
//
//             ValidTransaction::with_tag_prefix("ares-oracle::validate_transaction_parameters_of_purchased_price")
//                 .priority(priority_num.saturating_add(1))
//                 .and_provides(payload.public.clone())
//                 .longevity(5)
//                 .propagate(true)
//                 .build()
//         } else if let Call::submit_forced_clear_purchased_price_payload_signed {
//             price_payload: ref payload,
//             ref signature,
//         } = call
//         {
//             // submit_forced_clear_purchased_price_payload_signed
//             log::debug!(
// 					"🚅 Validate forced clear purchased price payload data, on block: {:?} ",
// 					<system::Pallet<T>>::block_number()
// 				);
//             if !Self::is_validator_member(&payload.auth) {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on force clear purchased"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//             let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
//             if !signature_valid {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Signature invalid. `InvalidTransaction` on force clear purchased"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//
//             let priority_num: u64 = T::UnsignedPriority::get();
//             ValidTransaction::with_tag_prefix(
//                 "ares-oracle::validate_transaction_parameters_of_force_clear_purchased",
//             )
//                 .priority(priority_num.saturating_add(2))
//                 .and_provides(&payload.block_number) // next_unsigned_at
//                 .longevity(5)
//                 .propagate(true)
//                 .build()
//         } else if let Call::submit_create_pre_check_task {
//             precheck_payload: ref payload,
//             ref signature,
//         } = call
//         {
//             log::debug!(
// 					"🚅 Validate submit_create_pre_check_task, on block: {:?}/{:?}, author: {:?} ",
// 					payload.block_number.clone(),
// 					<system::Pallet<T>>::block_number(),
// 					payload.public.clone()
// 				);
//             if !Self::is_validator_member(&payload.auth) {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on price "
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//             let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
//             if !signature_valid {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Signature invalid. `InvalidTransaction` on price"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//
//             ValidTransaction::with_tag_prefix("ares-oracle::submit_create_pre_check_task")
//                 .priority(T::UnsignedPriority::get())
//                 .and_provides(payload.pre_check_stash.clone()) // next_unsigned_at
//                 .longevity(5)
//                 .propagate(true)
//                 .build()
//         } else if let Call::submit_offchain_http_err_trace_result {
//             err_payload: ref payload,
//             ref signature,
//         } = call
//         {
//             // submit_offchain_http_err_trace_result
//             log::debug!(
// 					"🚅 Validate purchased price payload data, on block: {:?} ",
// 					<system::Pallet<T>>::block_number()
// 				);
//
//             if !Self::is_validator_member(&payload.auth) {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Payload public id is no longer in the members. `InvalidTransaction` on purchased price"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//
//             let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
//
//             if !signature_valid {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned",
// 						"⛔️ Signature invalid. `InvalidTransaction` on purchased price"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//
//             let priority_num: u64 = T::UnsignedPriority::get();
//
//             ValidTransaction::with_tag_prefix("ares-oracle::submit_offchain_http_err_trace_result")
//                 .priority(priority_num.saturating_add(1))
//                 .and_provides(payload.public.clone())
//                 .longevity(5)
//                 .propagate(true)
//                 .build()
//         } else if let Call::submit_offchain_pre_check_result {
//             preresult_payload: ref payload,
//             ref signature,
//         } = call
//         {
//             log::debug!(
//                     "🚅 Validate submit_offchain_pre_check_result, on block: {:?}/{:?}, stash: {:?}, auth: {:?}, pub: {:?}, pre_check_list: {:?}",
//                     payload.block_number.clone(),
//                     <system::Pallet<T>>::block_number(),
//                     payload.pre_check_stash.clone(),
//                     payload.pre_check_auth.clone(),
//                     payload.public.clone(),
//                     payload.pre_check_list.clone(),
//                 );
//             if <system::Pallet<T>>::block_number() - payload.block_number.clone() > 5u32.into() {
//                 return InvalidTransaction::BadProof.into();
//             }
//             // check stash status
//             let mut auth_list = Vec::new();
//             auth_list.push(payload.pre_check_auth.clone());
//             if let Some((s_stash, _, _)) = Self::get_pre_task_by_authority_set(auth_list) {
//                 if &s_stash != &payload.pre_check_stash {
//                     log::error!(
// 							target: "pallet::ocw::validate_unsigned - submit_offchain_pre_check_result",
// 							"⛔️ Stash account is inconsistent!"
// 						);
//                     return InvalidTransaction::BadProof.into();
//                 }
//             } else {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned - submit_offchain_pre_check_result",
// 						"⛔️ Could not find the pre-check task!"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//             //
//             let signature_valid = SignedPayload::<T>::verify::<T::OffchainAppCrypto>(payload, signature.clone());
//             if !signature_valid {
//                 log::error!(
// 						target: "pallet::ocw::validate_unsigned - submit_offchain_pre_check_result",
// 						"⛔️ Signature invalid. `InvalidTransaction` on price"
// 					);
//                 return InvalidTransaction::BadProof.into();
//             }
//             ValidTransaction::with_tag_prefix("ares-oracle::submit_offchain_pre_check_result")
//                 .priority(T::UnsignedPriority::get())
//                 .and_provides(&payload.block_number) // next_unsigned_at
//                 // .and_provides(payload.public.clone()) // next_unsigned_at
//                 .longevity(5)
//                 .propagate(true)
//                 .build()
//         } else if let Call::submit_local_xray {
//             ref host_key,
//             ref request_domain,
//             ref authority_list,
//             ref network_is_validator,
//         } = call
//         {
//             log::debug!("*** XRay ValidTransaction: {:?} ", <system::Pallet<T>>::block_number());
//             let current_blocknumber: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();
//             ValidTransaction::with_tag_prefix("ares-oracle::submit_local_xray")
//                 .priority(T::UnsignedPriority::get())
//                 .and_provides(&current_blocknumber.saturating_add((*host_key).into()))
//                 .longevity(5)
//                 .propagate(true)
//                 .build()
//         } else {
//             InvalidTransaction::Call.into()
//         }
//     }
// }