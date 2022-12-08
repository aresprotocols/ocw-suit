use super::*;
use codec::Encode;
use frame_support::dispatch::EncodeLike;
use frame_support::log;
use frame_system::offchain::{SendUnsignedTransaction, Signer, SigningTypes, SignMessage};
use sp_runtime::offchain::{http, Duration};
use sp_std::vec;
use ares_oracle::types::HttpError;
use sp_application_crypto::Public;
use sp_std::vec::Vec;
use arrform::{arrform, ArrForm};
use ares_oracle_provider_support::IStashAndAuthority;
use md5::{Md5, Digest};
// use rustc_hex::ToHex;
use sp_core::hexdisplay::HexDisplay;
use sp_runtime::traits::IdentifyAccount;
use crate::types::{CompletePayload, DispatchPayload, ReminderCallBackSign, ReminderCallBackUrl, ReminderRequestOptions};

impl<T: Config> Pallet<T>
    where <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
{
    pub fn call_reminder (signature_request: bool) -> Result<(), &'static str> {
        log::info!( target: DEBUG_TARGET, "Call reminder 5.");

        // Get current authority
        let local_authority_list = T::StashAndAuthorityPort::get_authority_list_of_local();
        let mut validator_list = T::StashAndAuthorityPort::get_list_of_storage();
        validator_list.retain(|(acc,ares)|{
            local_authority_list.iter().any(|local_ares|{
                local_ares == ares
            })
        });

        if validator_list.len() == 0 {
            return Err("Can not find validator.");
        }

        if validator_list.len() > 1 {
            log::warn!( target: DEBUG_TARGET, "Find multiple online validator accounts, only use the first one.");
        }

        let validator: (T::AccountId, T::AuthorityAres) = validator_list[0].clone();

        // Get mission of validator.
        PendingSendList::<T>::iter_prefix(&validator.0).for_each(|((rid, rbn), lbn)|{
            log::info!( target: DEBUG_TARGET, "While PendingSendList rid = {:?}, rbn = {:?}, lbn = {:?}", &rid, &rbn, &lbn);
            // pending_list.push((rid, rbn));
            // Get trigger
            let trigger_opt = ReminderList::<T>::get(&rid);
            if let Some(trigger) = trigger_opt {
                let http_call_opt = match trigger.trigger_receiver {
                    ReminderReceiver::HttpCallBack { url, sign } => {
                        Some((url, sign, rid.clone(), rbn.clone(), lbn.clone(), trigger.owner))
                    },
                    _ => {
                        None
                    }
                };
                if let Some(http_call) = http_call_opt {
                    let call_back_result = Self::request_callback(validator.1.clone(), http_call, signature_request, );
                    if let Ok((_, response_mark)) = call_back_result {
                        Self::save_complete_reminder(
                            validator.1.clone(),
                            (rid, rbn),
                            Some(response_mark),
                            Some(1),
                        );
                    }else{
                        log::error!( target: DEBUG_TARGET, "Request failed. {:?} ", call_back_result.err());
                        Self::save_complete_reminder(
                            validator.1.clone(),
                            (rid, rbn),
                            None,
                            Some(0),
                        );
                    }
                }
            }
        });

        Ok(())
    }

    pub fn request_callback (
        authority: T::AuthorityAres,
        http_call: (ReminderCallBackUrl, ReminderCallBackSign, ReminderIden, T::BlockNumber, T::BlockNumber, T::AccountId),
        signature_request: bool,
    ) -> Result<(Vec<u8>, Vec<u8>), http::Error> {
        // println!("request_callback = {:?} ", &http_call);
        let make_request_url = || -> Option<ReminderRequestOptions> {
            // let acc_param = arrform!(100, "{:?}", &http_call.5);
            // let acc_param = &acc_param.as_str()[10..65];
            let acc_vec = http_call.5.encode();
            let acc_hex = sp_core::hexdisplay::HexDisplay::from(&acc_vec);
            let url_params = arrform!(512, "&_rid_={:?}&_rbn_={:?}&_lbn_={:?}&_acc_={:?}", &http_call.2, &http_call.3, &http_call.4, &acc_hex, );
            //
            let sign_data = Signer::<T, T::OffchainAppCrypto>::any_account()
                .with_filter(Self::make_filter(authority.clone()) )
                .sign_message(url_params.as_bytes());


            if sign_data.is_none() {
                log::warn!("Sign failed on: {:?}", &authority);
                return None;
            }
            let sign_data = sign_data.unwrap();

            let post_data = if signature_request {

                let acc_acc_encode = sign_data.0.public.encode();
                let arr_acc_hex = sp_core::hexdisplay::HexDisplay::from(&acc_acc_encode);
                let arr_acc = arrform!(265, "{:?}", &arr_acc_hex);

                let arr_sign_encode = sign_data.1.encode();
                let arr_sign_hex = sp_core::hexdisplay::HexDisplay::from(&arr_sign_encode);
                let arr_sign = arrform!(265, "{:?}", &arr_sign_hex);
                // let sign_data_vec = &sign_data.clone().0.public.encode().to_hex();
                // let test = sp_std::str::from_utf8(&sign_data_vec.to_vec()).unwrap();
                // println!("test ==== {:?}", test);
                // println!("url_params = {:?}, authority = {:?}, arr_acc = {:?}, arr_sign = {:?}", &url_params.as_str(), &authority, arr_acc.as_str(), arr_sign.as_str());
                // (arr_acc.as_bytes().to_vec(), arr_sign.as_str().as_bytes().to_vec())
                (arr_acc.as_bytes().to_vec(), arr_sign.as_bytes().to_vec())
            }else{
                (Vec::new(), Vec::new())
            };

            if http_call.0.contains(&"?".as_bytes()[0]) {
                Some(ReminderRequestOptions{
                    request_url: [
                        http_call.0.to_vec(),
                        "&_s_=".as_bytes().to_vec(),
                        http_call.1.to_vec(),
                        url_params.as_str().as_bytes().to_vec(),
                    ].concat(),
                    sign_message: post_data
                })
                // Some(([
                //     http_call.0.to_vec(),
                //     "&_s_=".as_bytes().to_vec(),
                //     http_call.1.to_vec(),
                //     url_params.as_str().as_bytes().to_vec(),
                // ].concat(), post_data))
            }else{
                Some(ReminderRequestOptions{
                    request_url: [
                        http_call.0.to_vec(),
                        "?_s_=".as_bytes().to_vec(),
                        http_call.1.to_vec(),
                        url_params.as_str().as_bytes().to_vec(),
                    ].concat(),
                    sign_message: post_data
                })
                // Some(([
                //     http_call.0.to_vec(),
                //     "?_s_=".as_bytes().to_vec(),
                //     http_call.1.to_vec(),
                //     url_params.as_str().as_bytes().to_vec(),
                // ].concat(), post_data))
            }
        };


        let request_url_data = make_request_url();
        if request_url_data.is_none() {
            return Err(http::Error::Unknown);
        }
        let request_url_data = request_url_data.unwrap();

        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(4_000));
        let request_url = sp_std::str::from_utf8(&request_url_data.request_url).unwrap();
        let reminder_acc = sp_std::str::from_utf8(&request_url_data.sign_message.0).unwrap();
        let reminder_sign = sp_std::str::from_utf8(&request_url_data.sign_message.1).unwrap();

        let request =
            http::Request::get(request_url);
        // let request =
        //     http::Request::post(request_url, vec![post_data]); // vec![b"abc"]
        let pending = request.deadline(deadline)
            .add_header("reminder-acc", reminder_acc)
            .add_header("reminder-sign", reminder_sign)
            .send().map_err(|_| http::Error::IoError)?;
        let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
        if response.code != 200 {
            log::warn!("Unexpected status code: {}", response.code);
            return Err(http::Error::Unknown);
        }
        let body = response.body().collect::<Vec<u8>>();
        // Create a str slice from the body.
        let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
            log::warn!("No UTF8 body");
            http::Error::Unknown
        })?;

        let mut hasher = Md5::new();
        hasher.update(body_str.as_bytes());
        let md5_vec = hasher.finalize().to_vec();
        Ok((body, md5_vec))
    }

    pub fn save_dispatch_action () -> Result<(), &'static str> {
        let current_bn = <frame_system::Pallet<T>>::block_number();

        let local_ares_authority_list = T::StashAndAuthorityPort::get_authority_list_of_local();
        if local_ares_authority_list.len() > 0 {
            if(local_ares_authority_list.len() > 1) {
                log::warn!( target: DEBUG_TARGET, "Find multiple online validator accounts, only use the first one.");
            }
            let sign_authority = local_ares_authority_list[0].clone();
            let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
                .with_filter(Self::make_filter(sign_authority.clone()) )
                .send_unsigned_transaction(
                    |account| DispatchPayload::<T::AuthorityAres, T::Public, T::BlockNumber> {
                        auth: sign_authority.clone(),
                        bn: current_bn,
                        public: account.public.clone(),
                    },
                    |payload, signature| Call::submit_dispatch_action {
                        dispatch_payload: payload,
                        signature,
                    },
                )
                .ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;

            result.map_err(|()| "⛔ Unable to submit transaction")?;
        }else{
            log::warn!( target: DEBUG_TARGET, "None of the are-authorities available for signing.");
        }

        Ok(())
    }


    pub fn save_complete_reminder (
        auth: T::AuthorityAres,
        reminder: (ReminderIden, T::BlockNumber),
        response_mark: Option<Vec<u8>>,
        status: Option<u32>
    ) -> Result<(), &'static str> {

        // Get stash account.
        let stash_acc = T::StashAndAuthorityPort::get_stash_id(&auth);
        if stash_acc.is_none() {
            return Err("Not found stash on T::StashAndAuthorityPort.");
        }
        let stash_acc = stash_acc.unwrap();

        // Check pending list
        if !PendingSendList::<T>::contains_key(&stash_acc, &reminder){
            return Err("Not owner with PendingSendList.");
        }

        let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
            .with_filter(Self::make_filter(auth.clone()) )
            .send_unsigned_transaction(
                |account| CompletePayload::<ReminderIden, Vec<u8>, u32, T::AuthorityAres, T::Public, T::BlockNumber> {
                    reminder,
                    response_mark: response_mark.clone(),
                    status,
                    auth: auth.clone(),
                    public: account.public.clone(),
                },
                |payload, signature| Call::submit_complete_reminder {
                    complete_payload: payload,
                    signature,
                },
            )
            .ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;

        result.map_err(|()| "⛔ Unable to submit transaction")?;
        Ok(())
    }

    // TODO:: may be move to follow code to `ares-oracle-provider-support`.
    pub fn make_filter(account_id: T::AuthorityAres) -> Vec<T::Public> {
        let mut sign_public_keys: Vec<<T as SigningTypes>::Public> = Vec::new();
        let encode_data: Vec<u8> = account_id.encode();
        assert!(32 == encode_data.len());
        let raw_data = encode_data.try_into();
        let raw_data = raw_data.unwrap();
        // let new_account = T::AuthorityAres::unchecked_from(raw_data);
        let new_account = sp_core::sr25519::Public::from_raw(raw_data);
        // let new_account = sp_core::sr25519::Public::from_raw(raw_data);
        sign_public_keys.push(new_account.into());
        sign_public_keys
    }
}