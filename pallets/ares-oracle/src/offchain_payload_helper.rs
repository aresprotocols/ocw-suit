use super::*;

impl<T: Config> Pallet<T> {

    /// Convert `AuthorityAres` type to `Public` type
    pub fn handler_get_sign_public_keys(account_id: T::AuthorityAres) -> Vec<<T as SigningTypes>::Public>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        let mut sign_public_keys: Vec<<T as SigningTypes>::Public> = Vec::new();
        let encode_data: Vec<u8> = account_id.encode();
        assert!(32 == encode_data.len());
        let raw_data = encode_data.try_into();
        let raw_data = raw_data.unwrap();
        // let new_account = T::AuthorityAres::unchecked_from(raw_data);
        let new_account = sp_core::sr25519::Public::from_raw(raw_data);
        sign_public_keys.push(new_account.into());
        sign_public_keys
    }

    /// Get the list of `purchase-ids` that reached the delay,
    /// and construct the offchain-payload to submit the list to the chain.
    pub fn save_forced_clear_purchased_price_payload_signed(
        block_number: T::BlockNumber,
        account_id: T::AuthorityAres,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        let force_request_list = Self::get_expired_purchased_transactions();

        log::debug!("🚅 Force request list length: {:?} .", &force_request_list.len());
        if force_request_list.len() > 0 {
            let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
            // Singer
            let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
                .with_filter(sign_public_keys)
                .send_unsigned_transaction(
                    |account| PurchasedForceCleanPayload {
                        block_number,
                        purchase_id_list: force_request_list.clone(),
                        auth: account_id.clone(),
                        public: account.public.clone(),
                    },
                    |payload, signature| Call::submit_forced_clear_purchased_price_payload_signed {
                        price_payload: payload,
                        signature,
                    },
                )
                .ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
            result.map_err(|()| "⛔ Unable to submit transaction")?;
        }
        Ok(())
    }

    /// Submit the `price` result of `purchase-id` to the chain.
    pub fn save_fetch_purchased_price_and_send_payload_signed(
        block_number: T::BlockNumber,
        account_id: T::AuthorityAres,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        let mut price_list = Vec::new();
        // Get purchased request by AccountId
        let purchased_key = Self::fetch_purchased_request_keys(account_id.clone());

        if purchased_key.is_none() {
            log::debug!("🚅 Waiting for purchased service.");
            return Ok(());
        }

        let purchased_key = purchased_key.unwrap();
        if 0 == purchased_key.raw_source_keys.len() {
            log::warn!(
				target: "pallet::ocw::save_fetch_purchased_price_and_send_payload_signed",
				"❗ Purchased raw key is empty."
			);
            return Ok(());
        }

        let fetch_http_result = Self::fetch_bulk_price_with_http(purchased_key.clone().raw_source_keys);

        let price_result = fetch_http_result;
        if price_result.is_err() {
            log::error!(
				target: "pallet::ocw::save_fetch_purchased_price_and_send_payload_signed",
				"⛔ Ocw network error."
			);
            // Record http error.
            Self::trace_network_error(
                account_id,
                purchased_key.clone().raw_source_keys,
                price_result.err().unwrap(),
                "purchased_worker".as_bytes().to_vec(),
            );
            return Ok(());
        }

        for (price_key, price_option, fraction_length, json_number_value, timestamp) in price_result.unwrap() {
            if price_option.is_some() {
                // record price to vec!
                if let Some(number_value) =  JsonNumberValue::try_new(json_number_value) {
                    price_list.push(PricePayloadSubPrice(
                        price_key,
                        price_option.unwrap(),
                        fraction_length,
                        number_value,
                        timestamp,
                    ));
                }
            }
        }
        let price_list: PricePayloadSubPriceList = price_list.try_into().expect("price list is too long");
        log::debug!("🚅 fetch purchased price count: {:?}", price_list.len());
        if price_list.len() > 0 {
            let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
            // Singer
            let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
                .with_filter(sign_public_keys)
                .send_unsigned_transaction(
                    |account| PurchasedPricePayload {
                        price: price_list.clone(),
                        block_number,
                        purchase_id: purchased_key.purchase_id.clone(),
                        auth: account_id.clone(),
                        public: account.public.clone(),
                    },
                    |payload, signature| Call::submit_purchased_price_unsigned_with_signed_payload {
                        price_payload: payload,
                        signature,
                    },
                )
                .ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
            result.map_err(|()| "⛔ Unable to submit transaction")?;
        }
        Ok(())
    }

    /// Save the results submitted by pre-check to the chain.
    /// This method is only responsible for submitting data and
    /// does not specifically judge the correctness of the data.
    pub fn save_offchain_pre_check_result (
        stash_id: T::AccountId,
        auth_id: T::AuthorityAres,
        block_number: T::BlockNumber,
        pre_check_list: PreCheckList,
        task_at: T::BlockNumber,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        let sign_public_keys = Self::handler_get_sign_public_keys(auth_id.clone());
        // Singer
        let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
            .with_filter(sign_public_keys)
            .send_unsigned_transaction(
                |account| PreCheckResultPayload {
                    pre_check_stash: stash_id.clone(),
                    pre_check_auth: auth_id.clone(),
                    block_number,
                    pre_check_list: pre_check_list.clone(),
                    task_at,
                    public: account.public.clone(),
                },
                |payload, signature| Call::submit_offchain_pre_check_result {
                    preresult_payload: payload,
                    signature,
                },
            )
            .ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
        result.map_err(|()| "⛔ Unable to submit transaction")?;
        Ok(())
    }

    /// Log http errors that occur when submitting prices
    pub(crate) fn trace_network_error(
        account_id: T::AuthorityAres,
        _format_arr: RawSourceKeys,
        http_err: HttpError,
        tip: Vec<u8>,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
        // let tip: BoundedVec<u8, MaximumErrorTip> = tip.try_into().expect("tip is too long");
        // Singer
        let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
            .with_filter(sign_public_keys)
            .send_unsigned_transaction(
                |account| HttpErrTracePayload {
                    trace_data: HttpErrTraceData {
                        block_number: <system::Pallet<T>>::block_number(),
                        // request_list: format_arr.clone(),
                        err_auth: Self::get_stash_id(&account_id.clone()).unwrap(),
                        err_status: http_err.clone(),
                        tip: DataTipVec::create_on_vec(tip.clone()) ,
                    },
                    auth: account_id.clone(),
                    public: account.public.clone(),
                },
                |payload, signature| Call::submit_offchain_http_err_trace_result {
                    err_payload: payload,
                    signature,
                },
            )
            .ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
        result.map_err(|()| "⛔ Unable to submit transaction")?;
        Ok(())
    }

    /// Submit a pre-checked offchain extrinsics.
    /// - account_id: The `ares-authority` to sign the current commit data.
    /// - stash_id: Stash ID to review.
    /// - auth_id: Authority ID associated with Stash ID.
    /// - block_number: Submit block number.
    pub fn save_create_pre_check_task(
        account_id: T::AuthorityAres,
        stash_id: T::AccountId,
        auth_id: T::AuthorityAres,
        block_number: T::BlockNumber,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());

        let (_, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
            .with_filter(sign_public_keys)
            .send_unsigned_transaction(
                |account| PreCheckPayload {
                    block_number,
                    pre_check_stash: stash_id.clone(),
                    pre_check_auth: auth_id.clone(),
                    auth: account_id.clone(),
                    public: account.public.clone(),
                },
                |payload, signature| Call::submit_create_pre_check_task {
                    precheck_payload: payload,
                    signature,
                },
            )
            .ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
        result.map_err(|()| "⛔ Unable to submit transaction")?;
        Ok(())
    }

    /// Submit a set of prices on-chain via `offchian`.
    pub fn save_fetch_ares_price_and_send_payload_signed(
        block_number: T::BlockNumber,
        account_id: T::AuthorityAres,
    ) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        let mut price_list = Vec::new();
        // Get raw request.
        let format_arr = Self::make_bulk_price_format_data(block_number);
        // Filter jump block info
        let (format_arr, jump_block) = Self::filter_jump_block_data(
            format_arr.clone(),
            Self::get_stash_id(&account_id).unwrap(),
            block_number,
        );
        let price_result = Self::fetch_bulk_price_with_http(format_arr.clone());
        if price_result.is_err() {
            log::error!(
				target: "pallet::ocw::save_fetch_purchased_price_and_send_payload_signed",
				"⛔ Ocw network error."
			);
            Self::trace_network_error(
                account_id,
                format_arr.clone(),
                price_result.err().unwrap(),
                "ares_price_worker".as_bytes().to_vec(),
            );
            return Ok(());
        }
        let price_result = price_result.unwrap();
        for (price_key, price_option, fraction_length, json_number_value, timestamp) in price_result {
            if price_option.is_some() {
                if let Some(number_value) = JsonNumberValue::try_new(json_number_value) {
                    // record price to vec!
                    price_list.push(PricePayloadSubPrice(
                        price_key,
                        price_option.unwrap(),
                        fraction_length,
                        number_value,
                        timestamp,
                    ));
                }
            }
        }
        log::debug!(
			"🚅 fetch price count: {:?}, jump block count: {:?}",
			price_list.len(),
			jump_block.len()
		);
        let price_list: PricePayloadSubPriceList = price_list.try_into().expect("price_list is too long");
        if price_list.len() > 0 || jump_block.len() > 0 {
            let sign_public_keys = Self::handler_get_sign_public_keys(account_id.clone());
            // Singer
            let (acc_id, result) = Signer::<T, T::OffchainAppCrypto>::any_account()
                .with_filter(sign_public_keys)
                .send_unsigned_transaction(
                    |account| PricePayload {
                        price: price_list.clone(),
                        jump_block: jump_block.clone(),
                        block_number,
                        auth: account_id.clone(),
                        public: account.public.clone(),
                    },
                    |payload, signature| Call::submit_price_unsigned_with_signed_payload {
                        price_payload: payload,
                        signature,
                    },
                )
                .ok_or("❗ No local accounts accounts available, `ares` StoreKey needs to be set.")?;
            result.map_err(|()| "⛔ Unable to submit transaction")?;
        }
        Ok(())
    }
}