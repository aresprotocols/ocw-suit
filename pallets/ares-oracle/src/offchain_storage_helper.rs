use super::*;

impl<T: Config> Pallet<T> {

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