use super::*;
use codec::Encode;
use frame_support::dispatch::EncodeLike;
use frame_support::log;
use frame_system::offchain::{SendUnsignedTransaction, Signer, SigningTypes};
use sp_runtime::offchain::{http, Duration};
use sp_std::vec;
use ares_oracle::types::HttpError;
use sp_application_crypto::Public;

use ares_oracle_provider_support::IStashAndAuthority;
use md5::{Md5, Digest};
use crate::types::{CompletePayload, ReminderCallBackSign, ReminderCallBackUrl};

impl<T: Config> Pallet<T> {
    pub fn call_reminder () -> Result<(), &'static str>
        where <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        // Get current authority
        let local_authority_list = T::StashAndAuthorityPort::get_authority_list_of_local();
        let mut validator_list = T::StashAndAuthorityPort::get_list_of_storage();
        validator_list.retain(|(acc,ares)|{
            local_authority_list.iter().any(|local_ares|{
                local_ares == ares
            })
        });

        // println!("validator_list == {:?}", &validator_list);

        if validator_list.len() == 0 {
            return Ok(());
        }

        if validator_list.len() > 1 {
            log::warn!( target: DEBUG_TARGET, "Find multiple online validator accounts, only use the first one.");
        }

        let validator: (T::AccountId, T::AuthorityAres) = validator_list[0].clone();

        // Get mission of validator.
        PendingSendList::<T>::iter_prefix(&validator.0).for_each(|((rid, rbn), _)|{
            // println!("rid = {:?}, rbn = {:?}", &rid, &rbn);
            // pending_list.push((rid, rbn));
            // Get trigger
            let trigger_opt = ReminderList::<T>::get(&rid);
            if let Some(trigger) = trigger_opt {
                let http_call_opt = match trigger.trigger_receiver {
                    ReminderReceiver::HttpCallBack { url, sign } => {
                        Some((url, sign))
                    },
                    _ => {
                        None
                    }
                };
                if let Some(http_call) = http_call_opt {
                    let call_back_result = Self::request_callback(http_call);
                    if let Ok((_, response_mark)) = call_back_result {
                        Self::save_complete_reminder(
                            validator.1.clone(),
                            (rid, rbn),
                            Some(response_mark),
                            None,
                        );
                    }else{
                        log::error!( target: DEBUG_TARGET, "Request failed. {:?}", call_back_result.err());
                    }
                }
            }
        });

        Ok(())
    }

    pub fn request_callback (http_call: (ReminderCallBackUrl, ReminderCallBackSign)) -> Result<(Vec<u8>, Vec<u8>), http::Error> {

        let make_request_url = || -> Vec<u8> {
            [http_call.0.to_vec(), "&sign=".as_bytes().to_vec(), http_call.1.to_vec()].concat()
        };
        let request_url = make_request_url();

        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(4_000));
        let request_url = sp_std::str::from_utf8(&request_url).unwrap();

        // println!("request_url == {:?}", request_url);

        let request =
            http::Request::get(request_url);
        let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
        let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
        if response.code != 200 {
            log::warn!("Unexpected status code: {}", response.code);
            return Err(http::Error::Unknown)
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


    pub fn save_complete_reminder (
        auth: T::AuthorityAres,
        reminder: (ReminderIden, T::BlockNumber),
        response_mark: Option<Vec<u8>>,
        status: Option<u32>
    ) -> Result<(), &'static str>
        where <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        // let sign_public_keys = Self::handler_get_sign_public_keys(auth_id.clone());
        // Singer

        // println!("save_complete_reminder &auth={:?}, &reminder={:?}, &response_mark={:?}, &status={:?}", &auth, &reminder, &response_mark, &status);

        // TODO:: may be move to follow code to `ares-oracle-provider-support`.
        let make_filter = |account_id: T::AuthorityAres| {
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
        };

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
            .with_filter(make_filter(auth.clone()) )
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
}