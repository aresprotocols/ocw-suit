use super::*;

impl<T: Config> Pallet<T> {

    /// Link `sub_path` behind `request_domain`.
    pub fn make_local_storage_request_uri_by_vec_u8(sub_path: Vec<u8>) -> RequestBaseVecU8 {
        let domain = Self::get_local_storage_request_domain(); //.as_bytes().to_vec();
        let local_storage_request_url_res = RequestBaseVecU8::try_create_on_vec([domain.to_vec(), sub_path].concat());
        if let Ok(local_storage_request_url) = local_storage_request_url_res {
            return local_storage_request_url;
        }else{
            log::error!( target: ERROR_MAX_LENGTH_TARGET, "{}, on {}", ERROR_MAX_LENGTH_DESC, "local_storage_request_url" );
        }
        Default::default()
    }

    /// Calculate a random integer and store it in `local-storage`,
    /// the key is [LOCAL_HOST_KEY](ares_oracle_provider_support::LOCAL_HOST_KEY)
    pub fn get_local_host_key() -> u32 {
        let storage_local_host_key = StorageValueRef::persistent(LOCAL_HOST_KEY);

        let old_key = storage_local_host_key.get::<u32>();
        if let Some(storage_key) = old_key.unwrap() {
            return storage_key;
        }

        let seed = sp_io::offchain::random_seed();
        let random = <u32>::decode(&mut TrailingZeroInput::new(seed.as_ref()));
        if random.is_ok() {
            let random = random.unwrap();
            storage_local_host_key.set(&random);
            return random;
        }
        0
    }

    /// If `local-xray` data has exceeded the length defined by the `maximum_due` parameter, it is removed.
    pub(crate) fn check_and_clean_hostkey_list(maximum_due: T::BlockNumber) -> Weight {
        let current_block_num: T::BlockNumber = <system::Pallet<T>>::block_number();
        let mut weight :Weight = 1;
        <LocalXRay<T>>::iter().any(|(host_key,(create_bn, _, _, _))|{
            if current_block_num.saturating_sub(create_bn) > maximum_due {
                <LocalXRay<T>>::remove(host_key);
                weight +=1;
            }
            false
        });
        weight
    }

    /// Get the request address set by werahouse.
    pub fn get_local_storage_request_domain() -> RequestBaseVecU8 {

        let request_base_onchain = RequestBaseOnchain::<T>::get();
        if request_base_onchain.len() > 0 {
            return request_base_onchain;
        }

        let storage_request_base = StorageValueRef::persistent(LOCAL_STORAGE_PRICE_REQUEST_DOMAIN);

        if let Some(request_base) = storage_request_base.get::<Vec<u8>>().unwrap_or(Some(Vec::new())) {

            if let Ok(result_base_str) = sp_std::str::from_utf8(&request_base) {
                log::debug!("🚅 Ares local request base: {:?}.", &result_base_str);
            }
            let request_base_vec_res = RequestBaseVecU8::try_create_on_vec(request_base) ;
            if let Ok(request_base_vec) = request_base_vec_res {
                return request_base_vec;
            }else{
                log::error!( target: ERROR_MAX_LENGTH_TARGET, "{}, on {}", ERROR_MAX_LENGTH_DESC, "request_base_vec" );
            }
        }
        Default::default()
    }
}