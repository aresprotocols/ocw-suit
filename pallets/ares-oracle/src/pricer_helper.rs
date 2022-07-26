use super::*;

impl<T: Config> Pallet<T> {

    /// Calculates the median for the given `prices`.
    pub fn calculation_average_price(mut prices: Vec<u64>, kind: u8) -> Option<u64> {
        if 2 == kind {
            // use median
            prices.sort();
            if prices.len() % 2 == 0 {
                // get 2 mid element then calculation average.
                return Some((prices[prices.len() / 2] + prices[prices.len() / 2 - 1]) / 2);
            } else {
                // get 1 mid element and return.
                return Some(prices[prices.len() / 2]);
            }
        }
        if 1 == kind {
            return Some(prices.iter().fold(0_u64, |a, b| a.saturating_add(*b)) / prices.len() as u64);
        }
        None
    }

    /// Calculate current average price. // fraction_length: FractionLength
    pub(crate) fn average_price(
        // prices_info: Vec<AresPriceData<T::AccountId, T::BlockNumber>>,
        prices_info: PurchasedPriceDataVec<T>,
        kind: u8,
    ) -> Option<(u64, FractionLength, Vec<(T::AccountId, T::BlockNumber)>)> {
        let mut fraction_length_of_pool: FractionLength = 0;
        // Check and get fraction_length.
        prices_info.clone().into_iter().any(|tmp_price_data| {
            if 0 == fraction_length_of_pool {
                fraction_length_of_pool = tmp_price_data.fraction_len;
            }
            if fraction_length_of_pool != tmp_price_data.fraction_len {
                panic!("Inconsistent of fraction lenght.")
            }
            false
        });

        let mut prices: Vec<u64> = Vec::new(); // prices_info.clone().into_iter().map(|price_data| price_data.price).collect();
        let mut account_list: Vec<(T::AccountId, T::BlockNumber)> = Vec::new(); // prices_info.into_iter().map(|price_data| price_data.account_id).collect();

        for price_info in prices_info {
            prices.push(price_info.price);
            account_list.push((price_info.account_id, price_info.create_bn));
        }

        if prices.is_empty() {
            return None;
        }

        match Self::calculation_average_price(prices, kind) {
            Some(price_value) => {
                return Some((price_value, fraction_length_of_pool, account_list));
            }
            _ => {}
        }
        None
    }

    ///
    pub(crate) fn add_purchased_price(
        purchase_id: PurchaseId,
        account_id: T::AccountId,
        create_bn: T::BlockNumber,
        price_list: PricePayloadSubPriceList,
    ) {
        if <PurchasedOrderPool<T>>::contains_key(purchase_id.clone(), account_id.clone()) {
            return ();
        }
        let current_block = <system::Pallet<T>>::block_number();
        price_list.iter().any(|PricePayloadSubPrice(a, b, c, d, timestamp)| {
            let mut price_data_vec = <PurchasedPricePool<T>>::get(purchase_id.clone(), a.clone()).unwrap_or(Default::default());
            price_data_vec.try_push(AresPriceData {
                price: *b,
                account_id: account_id.clone(),
                create_bn: create_bn.clone(),
                fraction_len: *c,
                raw_number: d.clone(),
                timestamp: timestamp.clone(),
                update_bn: current_block
            });
            //
            <PurchasedPricePool<T>>::insert(purchase_id.clone(), a.clone(), price_data_vec);
            false
        });

        <PurchasedOrderPool<T>>::insert(purchase_id.clone(), account_id.clone(), current_block.clone());

        Self::deposit_event(Event::NewPurchasedPrice {
            created_at: create_bn,
            price_list,
            who: account_id,
            finance_era: T::OracleFinanceHandler::current_era_num(),
            purchase_id
        });
    }

    /// Save paid price results,The submitted price will be compared with the average price,
    /// and if the offset is too large, it will be recorded in `AresAbnormalPrice<T>`.
    ///
    /// Offset setting parameters are stored in `PriceAllowableOffset<T>`
    pub(crate) fn handler_purchase_avg_price_storage (
        purchase_id: PurchaseId,
        price_key: PriceKey,
        mut prices_info: PurchasedPriceDataVec<T>,
        reached_type: u8,
    ) -> Option<(PriceKey, PurchasedAvgPriceData, Vec<T::AccountId>)> {
        let (average, fraction_length, account_list) =
            Self::average_price(prices_info.clone(), T::CalculationKind::get()).expect("The average is not empty.");

        // Abnormal price index list
        let mut abnormal_price_index_list = Vec::new();
        // Pick abnormal price.
        if 0 < prices_info.len() {
            for (index, check_price) in prices_info.iter().enumerate() {
                // let offset_percent = match check_price.price {
                // 	x if &x > &average => ((x - average) * 100) / average,
                // 	x if &x < &average => ((average - x) * 100) / average,
                // 	_ => 0,
                // };
                let offset_percent = match check_price.price {
                    x if &x > &average => Percent::from_rational((x - average), average),
                    x if &x < &average => Percent::from_rational((average - x), average),
                    _ => Percent::from_percent(0),
                };
                if offset_percent > <PriceAllowableOffset<T>>::get()  {

                    <AresAbnormalPrice<T>>::try_append(
                        price_key.clone(),
                        (
                            check_price,
                            AvgPriceData {
                                integer: average,
                                fraction_len: fraction_length,
                            },
                        ),
                    );
                    // abnormal_price_index_list
                    abnormal_price_index_list.push(index);
                }
            }

            let mut remove_count = 0;
            // has abnormal price.
            if abnormal_price_index_list.len() > 0 {
                // pick out abnormal
                abnormal_price_index_list.iter().any(|remove_index| {
                    prices_info.remove(*remove_index - remove_count);
                    remove_count += 1;
                    false
                });
                // reset price pool
                return Self::handler_purchase_avg_price_storage(
                    purchase_id.clone(),
                    price_key.clone(),
                    prices_info.clone(),
                    reached_type,
                );
            }
            // let current_block = <system::Pallet<T>>::block_number() as u64;
            let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();
            // get valid request price accounts
            let valid_request_account_id_list: Vec<T::AccountId> = prices_info
                .into_iter()
                .map(|x| {
                    // Self::get_stash_id(&x.account_id).unwrap()
                    x.account_id
                })
                .collect();

            let avg_price_data = PurchasedAvgPriceData {
                create_bn: current_block,
                reached_type,
                price_data: (average.clone(), fraction_length.clone()),
            };
            // Update avg price (average, fraction_length)
            <PurchasedAvgPrice<T>>::insert(purchase_id.clone(), price_key.clone(), avg_price_data.clone());
            <PurchasedAvgTrace<T>>::insert(purchase_id.clone(), <system::Pallet<T>>::block_number());
            return Some((price_key.clone(), avg_price_data, valid_request_account_id_list));
        }
        None
    }

    /// Execute the average price aggregation of `ask-price`, and the expired data will be deleted after aggregation
    /// - purchase_id: `ask-price` order id.
    /// - reached_type: Take the value of [PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE](PURCHASED_FINAL_TYPE_IS_ALL_PARTICIPATE)
    /// or [PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE](PURCHASED_FINAL_TYPE_IS_PART_PARTICIPATE).
    pub(crate) fn update_purchase_avg_price_storage(purchase_id: PurchaseId, reached_type: u8) -> usize {
        // Get purchase price pool
        let price_key_list = <PurchasedPricePool<T>>::iter_key_prefix(purchase_id.clone()).collect::<Vec<_>>();
        //
        let mut event_result_list = Vec::new();
        //
        price_key_list.iter().any(|x| {
            let prices_info = <PurchasedPricePool<T>>::get(purchase_id.clone(), x.clone()).unwrap_or(Default::default());
            let result =
                Self::handler_purchase_avg_price_storage(purchase_id.clone(), x.clone(), prices_info, reached_type);
            event_result_list.push(result);
            false
        });

        let result_count =event_result_list.len();
        Self::deposit_event(Event::PurchasedAvgPrice {
            purchase_id: purchase_id.clone(),
            event_results: event_result_list,
            finance_era: T::OracleFinanceHandler::current_era_num(),
        });
        let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();

        Self::check_and_clear_expired_purchased_average_price_storage(current_block);

        result_count
    }

    /// Generate an order ID
    pub(crate) fn make_purchase_price_id(who: T::AccountId, add_up: u8) -> PurchaseId {
        // check add up
        if add_up == u8::MAX {
            panic!("⛔ Add up number too large.");
        }
        let mut account_vec: Vec<u8> = who.encode();
        // Get block number to u64
        let current_block_num: T::BlockNumber = <system::Pallet<T>>::block_number();
        let current_blocknumber: u64 = current_block_num.unique_saturated_into();
        let mut current_bn_vec: Vec<u8> = current_blocknumber.encode();
        account_vec.append(&mut current_bn_vec);
        // Get add up number
        let mut add_u8_vec: Vec<u8> = add_up.encode();
        account_vec.append(&mut add_u8_vec);

        let purchase_id: PurchaseId = account_vec.clone().try_into().expect("`PurchaseId` BoundLength Error.");
        // Check id exists.
        if PurchasedRequestPool::<T>::contains_key(&purchase_id) {
            return Self::make_purchase_price_id(who, add_up.saturating_add(1));
        }
        purchase_id
    }

    /// Add an purchase-order
    ///
    /// who: Order submitter
    /// offer: Order deposit
    /// submit_threshold: Minimum submission threshold
    /// max_duration: Order maximum response delay,
    /// purchase_id: Order id,
    /// request_keys: `Trading pairs` list,
    pub(crate) fn ask_price(
        who: T::AccountId,
        offer: BalanceOf<T>,
        submit_threshold: Percent,
        max_duration: u64,
        purchase_id: PurchaseId,
        request_keys: RequestKeys,
    ) -> Result<PurchaseId, Error<T>> {
        // if 0 >= submit_threshold || 100 < submit_threshold {
        if submit_threshold == Zero::zero() {
            return Err(Error::<T>::SubmitThresholdRangeError);
        }
        if 0 >= max_duration {
            return Err(Error::<T>::DruationNumberNotBeZero);
        }
        let current_block: u64 = <system::Pallet<T>>::block_number().unique_saturated_into();
        let request_data = PurchasedRequestData {
            account_id: who,
            offer,
            submit_threshold,
            create_bn: <system::Pallet<T>>::block_number(),
            max_duration: current_block.saturating_add(max_duration),
            request_keys,
        };
        <PurchasedRequestPool<T>>::insert(purchase_id.clone(), request_data.clone());
        Self::deposit_event(Event::NewPurchasedRequest {
            purchase_id: purchase_id.clone(),
            request_data,
            value: offer,
            finance_era: T::OracleFinanceHandler::current_era_num(),
        });
        Ok(purchase_id)
    }

    // add price on chain
    pub(crate) fn add_price_and_try_to_agg(
        who: T::AccountId,
        price: u64,
        price_key: PriceKey,
        fraction_length: FractionLength,
        json_number_value: JsonNumberValue,
        max_len: u32,
        timestamp: u64,
        create_bn: T::BlockNumber,
    ) -> Option<(PriceKey, u64, FractionLength, Vec<(T::AccountId, T::BlockNumber)>)> {
        let key_str = price_key;
        let current_block = <system::Pallet<T>>::block_number();
        // 1. Check key exists
        if <AresPrice<T>>::contains_key(key_str.clone()) {
            // get and reset .
            let old_price = <AresPrice<T>>::get(key_str.clone()).unwrap_or(Default::default());

            let mut is_fraction_changed = false;
            // check fraction length inconsistent.
            for (_index, price_data) in old_price.clone().iter().enumerate() {
                if &price_data.fraction_len != &fraction_length {
                    is_fraction_changed = true;
                    break;
                }
            }
            //
            let mut new_price = Vec::new();
            let max_len: usize = max_len.clone() as usize;

            for (index, value) in old_price.iter().enumerate() {
                if old_price.len() >= max_len && 0 == index {
                    continue;
                }

                let mut old_value = (*value).clone();
                if is_fraction_changed {
                    old_value = (*value).clone();
                    old_value.price = old_value.raw_number.to_price(fraction_length.clone());
                    old_value.fraction_len = fraction_length.clone();
                }

                // let new_value = old_value;
                new_price.push(old_value);
            }
            let new_ares_price_data = AresPriceData {
                price: price.clone(),
                account_id: who.clone(),
                create_bn,
                fraction_len: fraction_length,
                raw_number: json_number_value,
                timestamp,
                update_bn: current_block,
            };
            new_price.push(new_ares_price_data.clone());

            let new_price_res = AresPriceDataVecOf::<T>::try_create_on_vec(new_price);
            // Get the last price.
            if let Some((_, last_update_bn)) = Self::get_last_price_author(key_str.clone()) {
                // Compare with the current price.
                if current_block.saturating_sub(last_update_bn) <= ConfDataSubmissionInterval::<T>::get() {
                    // Within the threshold range.
                    if let Ok(new_price) = new_price_res {

                        <AresPrice<T>>::insert(key_str.clone(), new_price);
                        <LastPriceAuthor<T>>::insert(key_str.clone(), (who.clone(), create_bn));
                    }else{
                        log::error!( target: ERROR_MAX_LENGTH_TARGET, "{}, on {}", ERROR_MAX_LENGTH_DESC, "new_price" );
                    }
                }else{
                    log::error!(
                        target: "pallet::ocw::ares_price_worker",
                        "⛔ The data submission interval is too long and data pool will be reset. price_key = {:?}, last = {:?}, current = {:?}",
                        key_str, last_update_bn, current_block
                    );
                    let mut new_price = AresPriceDataVecOf::<T>::default();
                    new_price.try_push(new_ares_price_data);
                    <AresPrice<T>>::insert(key_str.clone(), new_price);
                }
            }else{
                log::error!(
                    target: "pallet::ocw::ares_price_worker",
                    "⛔ No last update was found for the corresponding price and data pool will be reset. price_key = {:?}",
                    key_str
                );
                <AresPrice<T>>::insert(key_str.clone(), AresPriceDataVecOf::<T>::default());
            }
        } else {
            // push a new value.
            let mut new_price = AresPriceDataVecOf::<T>::default();
            new_price.try_push(AresPriceData {
                price: price.clone(),
                account_id: who.clone(),
                create_bn,
                fraction_len: fraction_length,
                raw_number: json_number_value,
                timestamp,
                update_bn: current_block,
            });
            <AresPrice<T>>::insert(key_str.clone(), new_price);
            <LastPriceAuthor<T>>::insert(key_str.clone(), (who.clone(), create_bn));
        }

        let avg_check_result = Self::check_and_update_avg_price_storage(key_str.clone(), max_len);
        // Check if the price request exists, if not, clear storage data.
        let price_request_list = <PricesRequests<T>>::get();
        let has_key = price_request_list
            .iter()
            .any(|(any_price, _, _, _, _)| any_price == &key_str);
        if !has_key {
            Self::clear_price_storage_data(key_str);
        }
        avg_check_result
    }

    pub(crate) fn check_and_update_avg_price_storage(key_str: PriceKey, max_len: u32) -> Option<(PriceKey, u64, FractionLength, Vec<(T::AccountId, T::BlockNumber)>)> {
        let ares_price_list_len = <AresPrice<T>>::get(key_str.clone()).unwrap_or(Default::default()).len();
        if ares_price_list_len >= max_len as usize && ares_price_list_len > 0 {
            return Self::update_avg_price_storage(key_str.clone());
        }
        None
    }

    pub(crate) fn update_avg_price_storage(key_str: PriceKey) -> Option<(PriceKey, u64, FractionLength, Vec<(T::AccountId, T::BlockNumber)>)> {
        let prices_info = <AresPrice<T>>::get(key_str.clone()).unwrap_or(Default::default());
        let average_price_result = Self::average_price(prices_info, T::CalculationKind::get());
        if let Some((average, fraction_length, account_list)) = average_price_result {
            let mut price_list_of_pool = <AresPrice<T>>::get(key_str.clone()).unwrap_or(Default::default());
            // Abnormal price index list
            let mut abnormal_price_index_list = Vec::new();
            // Pick abnormal price.
            if 0 < price_list_of_pool.len() {
                for (index, check_price) in price_list_of_pool.iter().enumerate() {

                    let offset_percent = match check_price.price {
                        x if &x > &average => Percent::from_rational((x - average), average),
                        x if &x < &average => Percent::from_rational((average - x), average),
                        _ => Percent::from_percent(0),
                    };

                    if offset_percent > <PriceAllowableOffset<T>>::get() {
                        // Set price to abnormal list and pick out check_price
                        <AresAbnormalPrice<T>>::try_append(
                            key_str.clone(),
                            (
                                check_price,
                                AvgPriceData {
                                    integer: average,
                                    fraction_len: fraction_length,
                                },
                            ),
                        );
                        // abnormal_price_index_list
                        abnormal_price_index_list.push(index);
                    }
                }

                let mut remove_count = 0;
                // has abnormal price.
                if abnormal_price_index_list.len() > 0 {
                    // pick out abnormal
                    abnormal_price_index_list.iter().any(|remove_index| {
                        price_list_of_pool.remove(*remove_index - remove_count);
                        remove_count += 1;
                        false
                    });
                    // reset price pool
                    <AresPrice<T>>::insert(key_str.clone(), price_list_of_pool);
                    return Self::update_avg_price_storage(key_str.clone());
                }
                // Get current bn
                let current_bn = <system::Pallet<T>>::block_number();
                // Update avg price
                <AresAvgPrice<T>>::insert(key_str.clone(), (average, fraction_length, current_bn));
                // Clear price pool.
                <AresPrice<T>>::remove(key_str.clone());
                //
                return Some((key_str.clone(), average, fraction_length, account_list));
            }
        }
        None
    }

    /// Get a list of purchase-order tasks to be performed by the validator
    pub(crate) fn fetch_purchased_request_keys(who: T::AuthorityAres) -> Option<PurchasedSourceRawKeys> // Vec<(Vec<u8>, Vec<u8>, FractionLength, RequestInterval)>
    {
        let mut raw_source_keys = Vec::new();
        let mut raw_purchase_id: PurchaseId = PurchaseId::default();

        let stash_id = Self::get_stash_id(&who).unwrap();
        // Iter
        <PurchasedRequestPool<T>>::iter().any(|(purchase_id, request_data)| {
            if false == <PurchasedOrderPool<T>>::contains_key(purchase_id.clone(), stash_id.clone()) {
                raw_purchase_id = purchase_id.clone();
                raw_source_keys = Self::filter_raw_price_source_list(request_data.request_keys)
                    .iter()
                    .map(|(price_key, parse_key, fraction_len, _)| {
                        (price_key.clone(), parse_key.clone(), *fraction_len)
                    })
                    .collect();
                return true;
            }
            false
        });

        if raw_source_keys.len() == 0 {
            return None;
        }
        let raw_source_keys: RawSourceKeys = raw_source_keys.clone().try_into().expect("raw_source_keys is too long");
        Some(PurchasedSourceRawKeys {
            purchase_id: raw_purchase_id,
            raw_source_keys,
        })
    }

    /// Update the last committer of `trading-pairs` in `price_list` list to `author`.
    pub(crate) fn update_last_price_list_for_author(price_list: Vec<PriceKey>, author: T::AccountId, create_bn: T::BlockNumber) {
        price_list.iter().any(|price_key| {
            // find_author()
            <LastPriceAuthor<T>>::insert(price_key.clone(), (author.clone(), create_bn));
            false
        });
    }

    /// Get the last committer of a `trading-pairs`
    pub fn get_last_price_author(price_key: PriceKey) -> Option<(T::AccountId,T::BlockNumber)> {
        if <LastPriceAuthor<T>>::contains_key(&price_key) {
            return Some(<LastPriceAuthor<T>>::get(price_key).unwrap().into());
        }
        None
    }

    /// Sign the `free-price data` by some `ares-authority `and submit it to the chain
    ///
    /// - block_number: Incoming committed block
    /// - account_id: The ares-authority account to be signed,
    ///
    /// Tip: this account must have a private key stored in the keystore on the local node to sign the data.
    pub(crate) fn ares_price_worker(block_number: T::BlockNumber, account_id: T::AuthorityAres) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        log::debug!("ares_price_worker: save ares price bn: {:?} who: {:?}", &block_number, &account_id);
        let res = Self::save_fetch_ares_price_and_send_payload_signed(block_number.clone(), account_id.clone());
        if let Err(e) = res {
            log::error!(
				target: "pallet::ocw::ares_price_worker",
				"⛔ block number = {:?}, account = {:?}, error = {:?}",
				block_number, account_id, e
			);
        }
        Ok(())
    }

    /// Sign the `purchased-price data` by some `ares-authority `and submit it to the chain
    ///
    /// - block_number: Incoming committed block
    /// - account_id: The ares-authority account to be signed,
    ///
    /// Tip: this account must have a private key stored in the keystore on the local node to sign the data.
    pub(crate) fn ares_purchased_worker(block_number: T::BlockNumber, account_id: T::AuthorityAres) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        let res = Self::save_fetch_purchased_price_and_send_payload_signed(block_number.clone(), account_id.clone());
        if let Err(e) = res {
            log::error!(
				target: "pallet::ocw::purchased_price_worker",
				"⛔ block number = {:?}, account = {:?}, error = {:?}",
				block_number, account_id, e
			);
        }
        Ok(())
    }

    /// Check and clear purchase-price data as these data are only retained for a specific period of time.
    /// - block_number: The block height at which to check.
    /// - account_id: The ares-authority to the offchain submission data signature.
    pub(crate) fn ares_purchased_checker(block_number: T::BlockNumber, account_id: T::AuthorityAres) -> Result<(), &'static str>
        where
            <T as frame_system::offchain::SigningTypes>::Public: From<sp_application_crypto::sr25519::Public>,
    {
        let res = Self::save_forced_clear_purchased_price_payload_signed(block_number.clone(), account_id.clone());
        if let Err(e) = res {
            log::error!(
				target: "pallet::ocw::ares_purchased_checker",
				"⛔ block number = {:?}, account = {:?}, error = {:?}",
				block_number, account_id, e
			);
        }
        Ok(())
    }



    /// Get a list of `Trading pairs`, this list also contains specific ParseVersion, FractionLength, RequestInterval.
    ///
    /// `Vec<(PriceKey, PriceToken, ParseVersion, FractionLength, RequestInterval)>`
    ///
    pub fn get_raw_price_source_list() -> Vec<(PriceKey, PriceToken, u32, FractionLength, RequestInterval)> {
        let result: Vec<(PriceKey, PriceToken, u32, FractionLength, RequestInterval)> = <PricesRequests<T>>::get()
            .into_iter()
            .map(
                |(price_key, price_token, parse_version, fraction_length, request_interval)| {
                    (
                        price_key,
                        price_token,
                        parse_version,
                        fraction_length,
                        request_interval,
                    )
                },
            )
            .collect();
        result
    }

    /// Filter out the list of `trading-pairs` that a block needs to submit
    pub(crate) fn make_bulk_price_format_data(
        block_number: T::BlockNumber,
    ) -> Vec<(PriceKey, PriceToken, FractionLength, RequestInterval)> {
        let mut format = Vec::new();

        // price_key, request_url, parse_version, fraction_length
        let source_list = Self::get_raw_price_source_list();

        // In the new version, it is more important to control the request interval here.
        for (price_key, extract_key, parse_version, fraction_length, request_interval) in source_list {
            if 2 == parse_version {
                let mut round_number: u64 = block_number.unique_saturated_into();
                round_number += Self::get_jump_block_number(price_key.clone());
                let remainder: u64 = (round_number % request_interval as u64).into();
                if 0 == remainder {
                    format.push((price_key, extract_key, fraction_length, request_interval));
                }
            }
        }
        log::debug!("🚅 Ares will be request list count {:?}", format.len());
        format
    }

    /// Extract the price data corresponding to `find_key` from `json_val`,
    /// If found return Option<(BitInt, NumberValue, Timestamp)>
    pub fn extract_bulk_price_by_json_value(
        json_val: JsonValue,
        find_key: &str,
        param_length: FractionLength,
    ) -> Option<(u64, NumberValue, u64)> {
        assert!(param_length <= 6, "Fraction length must be less than or equal to 6");
        let price_value = match json_val {
            JsonValue::Object(obj) => {
                // TODO:: need test again.
                let find_result = obj.into_iter().find(|(k, _)| {
                    // let tmp_k = k.iter().copied();
                    k.iter().copied().eq(find_key.chars())
                });

                match find_result {
                    None => {
                        return None;
                    }
                    Some((_, sub_val)) => match sub_val {
                        JsonValue::Object(price_obj) => {
                            let (_, price) = price_obj
                                .clone()
                                .into_iter()
                                .find(|(k, _)| k.iter().copied().eq("price".chars()))
                                .unwrap();
                            let match_number = match price {
                                JsonValue::Number(number) => number,
                                _ => return None,
                            };
                            let (_, timestamp) = price_obj
                                .into_iter()
                                .find(|(k, _)| k.iter().copied().eq("timestamp".chars()))
                                .unwrap();
                            let match_timestamp = match timestamp {
                                JsonValue::Number(timestamp) => timestamp.integer as u64,
                                _ => return None,
                            };
                            Some((match_number, match_timestamp))
                        }
                        _ => {
                            return None;
                        }
                    },
                }
            }
            _ => {
                return None;
            }
        };

        if price_value.is_none() {
            return None;
        }
        let price_value = price_value.unwrap();

        // Make u64 with fraction length
        let result_opt = JsonNumberValue::try_new(price_value.0.clone());
        if result_opt.is_none() {
            return None;
        }
        let result_numer = result_opt.unwrap();
        let result_price= result_numer.to_price(param_length);

        // A price of 0 means that the correct result of the data is not obtained.
        if result_price == 0 {
            return None;
        }
        Some((result_price, price_value.0, price_value.1))
    }



    /// Filter out supported `trading-pairs` from `request_data`.
    pub fn filter_raw_price_source_list(
        request_data: RequestKeys,
    ) -> Vec<(PriceKey, PriceToken, FractionLength, RequestInterval)> {
        let source_list = Self::get_raw_price_source_list();
        let mut result = Vec::new();
        source_list
            .iter()
            .any(|(price_key, price_token, _version, fraction, interval)| {
                if request_data.iter().any(|x| x == price_key) {
                    result.push((price_key.clone(), price_token.clone(), *fraction, *interval));
                }
                false
            });
        result
    }

    pub fn bulk_parse_price_of_ares(
        price_str: &str,
        base_coin: Vec<u8>,
        format: RawSourceKeys,
    ) -> Vec<(PriceKey, Option<u64>, FractionLength, NumberValue, u64)> {
        let val = lite_json::parse_json(price_str);

        let mut result_vec = Vec::new();
        match val.ok() {
            None => {
                return Vec::new();
            }
            Some(obj) => {
                match obj {
                    JsonValue::Object(obj) => {
                        // find code root.
                        let (_, v_data) = obj
                            .into_iter()
                            .find(|(k, _)| {
                                let _tmp_k = k.iter().copied();
                                k.iter().copied().eq("data".chars())
                            })
                            .unwrap();

                        for (price_key, extract_key, fraction_length) in format {
                            let new_extract_key = [extract_key.to_vec(), base_coin.clone()].concat();
                            let extract_key = sp_std::str::from_utf8(&new_extract_key).unwrap();
                            // println!("str::extract_key = {:?}", str::from_utf8(&price_key.clone().to_vec()));
                            // println!("str::extract_key = {:?}", extract_key);
                            let extract_price_grp =
                                Self::extract_bulk_price_by_json_value(v_data.clone(), extract_key, fraction_length);
                            if extract_price_grp.is_some() {
                                let (extract_price, json_number_value, timestamp) = extract_price_grp.unwrap();
                                // TODO::need recheck why use Some(extract_price)
                                result_vec.push((
                                    price_key,
                                    Some(extract_price),
                                    fraction_length,
                                    json_number_value,
                                    timestamp,
                                ));
                            }
                        }
                    }
                    _ => return Vec::new(),
                }
            }
        }
        result_vec
    }
}

