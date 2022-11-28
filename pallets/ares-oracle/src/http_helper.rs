use frame_support::dispatch::EncodeLike;
use super::*;

impl<'a, T: Config> Pallet<T>
{

    /// Given a set of `PriceKey, PriceToken`, the corresponding offchain-http request address is generated.
    pub fn make_bulk_price_request_url(format: RawSourceKeys) -> (Vec<u8>, Vec<u8>) {
        let raw_request_url = Self::make_local_storage_request_uri_by_vec_u8(
            "/api/getBulkCurrencyPrices?currency=usdt".as_bytes().to_vec(),
        );

        let mut request_url = Vec::new();
        for (_, extract_key, _) in format {
            if request_url.len() == 0 {
                request_url = [
                    raw_request_url.clone().to_vec(),
                    "&symbol=".as_bytes().to_vec(),
                    extract_key.to_vec(),
                ]
                    .concat();
            } else {
                request_url = [request_url, "_".as_bytes().to_vec(), extract_key.to_vec()].concat();
            }
        }
        (request_url, "usdt".as_bytes().to_vec())
    }

    /// Get the price list corresponding to `PriceKey`, `PriceToken` through offchain-http
    pub fn fetch_bulk_price_with_http(
        format_arr: RawSourceKeys,
    ) -> Result<Vec<(PriceKey, Option<u64>, FractionLength, NumberValue, u64)>, HttpError> {
        // make request url
        let (request_url_vu8, base_coin) = Self::make_bulk_price_request_url(format_arr.clone());
        let request_url = sp_std::str::from_utf8(&request_url_vu8).unwrap();
        // request and return http body.
        if "" == request_url {
            log::info!(target: "pallet::ocw::fetch_bulk_price_with_http", "❗ No requests to process.");
            return Ok(Vec::new());
        }
        log::debug!("🚅 Batch price request address: {:?}", request_url);
        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(4_000));
        let request = http::Request::get(request_url.clone());
        let pending = request
            .deadline(deadline)
            .send()
            .map_err(|_| HttpError::IoErr(DataTipVec::try_create_on_vec(request_url_vu8.clone()).unwrap_or(Default::default())))?;
        let response = pending.try_wait(deadline).map_err(|e| {
            log::warn!(
				target: "pallet::ocw::fetch_bulk_price_with_http",
				"❗ The network cannot connect. http::Error::DeadlineReached error = {:?}",
				e
			);
        });

        if response.is_err() {
            return Err(HttpError::TimeOut(DataTipVec::try_create_on_vec(request_url_vu8.clone()).unwrap_or(Default::default())));
        }

        let response = response.unwrap();

        if let Err(e) = response {
            log::warn!(
				target: "pallet::ocw::fetch_bulk_price_with_http",
				"❗ Https is not currently supported. msg = {:#?}", e
			);
            return Err(HttpError::IoErr(DataTipVec::try_create_on_vec(request_url_vu8.clone()).unwrap_or(Default::default())));
        }

        let response = response.unwrap();

        if response.code != 200 {
            log::warn!(
				target: "pallet::ocw::fetch_bulk_price_with_http",
				"❗ Unexpected http status code: {}",
				response.code
			);
            // return Err(http::Error::Unknown);
            return Err(HttpError::StatusErr(DataTipVec::try_create_on_vec(request_url_vu8.clone()).unwrap_or(Default::default()), response.code));
        }
        let body = response.body().collect::<Vec<u8>>();
        // Create a str slice from the body.
        let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
            log::warn!(
				target: "pallet::ocw::fetch_bulk_price_with_http",
				"❗ Extracting body error, No UTF8 body!"
			);
            // http::Error::IoError
            HttpError::ParseErr(DataTipVec::try_create_on_vec(request_url_vu8.clone()).unwrap_or(Default::default()))
        })?;

        log::debug!("Batch price request body length: {:?}", body_str);

        Ok(Self::bulk_parse_price_of_ares(body_str, base_coin, format_arr))
    }
}