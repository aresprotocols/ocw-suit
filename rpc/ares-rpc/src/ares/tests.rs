use super::*;
use super::*;
use assert_matches::assert_matches;
// use sc_rpc::offchain::Offchain;
use sp_core::{offchain::storage::InMemOffchainStorage, Bytes};

use serde_json::{Result as JsonResult, Value as JsonValue, Number };

#[test]
fn test_storage_should_work() {
    let storage = InMemOffchainStorage::default();
    let offchain = AresToolsStruct::new(storage, DenyUnsafe::No, Role::Authority);
    let key = Bytes(b"offchain_storage".to_vec());
    let value = Bytes(b"offchain_value".to_vec());

    assert_matches!(
		offchain.set_local_storage(StorageKind::PERSISTENT, key.clone(), value.clone()),
		Ok(())
	);
    assert_matches!(
		offchain.get_local_storage(StorageKind::PERSISTENT, key),
		Ok(Some(ref v)) if *v == value
	);
}

#[test]
fn test_calls_considered_unsafe() {
    let storage = InMemOffchainStorage::default();
    let offchain = AresToolsStruct::new(storage, DenyUnsafe::Yes, Role::Authority);
    let key = Bytes(b"offchain_storage".to_vec());
    let value = Bytes(b"offchain_value".to_vec());

    assert_matches!(
		offchain.set_local_storage(StorageKind::PERSISTENT, key.clone(), value.clone()),
		Err(Error::UnsafeRpcCalled(_))
	);
    assert_matches!(
		offchain.get_local_storage(StorageKind::PERSISTENT, key),
		Err(Error::UnsafeRpcCalled(_))
	);
}

#[test]
fn test_get_warehouse() {
    let storage = InMemOffchainStorage::default();
    let offchain = AresToolsStruct::new(storage, DenyUnsafe::No, Role::Authority);

    let request_key = Bytes(LOCAL_STORAGE_PRICE_REQUEST_DOMAIN.to_vec());
    let request_domain = Bytes(b"http://aresprotocol.io/a".to_vec());

    assert_matches!(
		offchain.set_local_storage(StorageKind::PERSISTENT, request_key, request_domain.clone()),
		Ok(())
	);

    assert_matches!(
		offchain.get_warehouse(),
		Ok(Some(ref v)) if *v == String::from_utf8(request_domain.to_vec()).unwrap()
	);
}

#[test]
fn test_set_warehouse() {
    let storage = InMemOffchainStorage::default();
    let offchain = AresToolsStruct::new(storage, DenyUnsafe::No, Role::Authority);

    let request_key = Bytes(LOCAL_STORAGE_PRICE_REQUEST_DOMAIN.to_vec());
    let request_domain_str =  String::from("http://aresprotocol.io/b");
    let request_domain = Bytes(request_domain_str.as_bytes().to_vec());
    assert_matches!(
		offchain.set_warehouse(request_domain_str.clone()),
		Ok(())
	);

    assert_matches!(
		offchain.get_warehouse(),
		Ok(Some(ref v)) if *v == String::from_utf8(request_domain.to_vec()).unwrap()
	);
}

#[test]
fn test_get_xray() {
    let storage = InMemOffchainStorage::default();
    let offchain = AresToolsStruct::new(storage, DenyUnsafe::No, Role::Authority);

    let request_key = Bytes(LOCAL_HOST_KEY.to_vec());
    let value = Bytes("11111".as_bytes().to_vec());
    assert_matches!(
		offchain.set_local_storage(StorageKind::PERSISTENT, request_key, value.clone()),
		Ok(())
	);

    assert_matches!(
		offchain.get_xray(),
		Ok(Some(ref v)) if *v == value
	);
}

#[test]
fn test_get_infos() {
    let storage = InMemOffchainStorage::default();
    let offchain = AresToolsStruct::new(storage, DenyUnsafe::No, Role::Authority);

    let request_domain_str =  String::from("http://api.aresprotocol.io");
    offchain.set_warehouse(request_domain_str.clone());

    let res = offchain.get_infos();
    // println!("res = {:#?}", res);
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res, AresOracleInfos{
        warehouse: Some(request_domain_str),
        xray: None,
        request_scheme_checked: "Ok".to_string(),
        request_status_checked: "Ok".to_string(),
        request_body_checked: "Ok".to_string(),
        node_role: NodeRole::Authority,
    });
}

#[test]
fn test_try_request() {
    let storage = InMemOffchainStorage::default();
    let offchain = AresToolsStruct::new(storage, DenyUnsafe::No, Role::Authority);

    let request_domain_str =  String::from("http://api.aresprotocol.io");
    offchain.set_warehouse(request_domain_str.clone());

    let res = offchain.try_get_request();
    assert!(res.is_ok());
    let res = res.unwrap();
    assert_eq!(res.get_request_scheme(), "http");
    assert_eq!(res.get_request_status(), "200 OK");
    assert_eq!(res.get_url_path(), "/api/getBulkCurrencyPrices");
    assert_eq!(res.get_url_query(), Some("currency=usdt&symbol=btc_eth".to_string()));
    assert!(res.get_request_body().is_some());
}

#[test]
fn test_json() {
    // let data = r#"
    // {
    //     "code": 0,
    //     "message": "OK",
    //     "data": {
    //         "btcusdt": {
    //             "price": 39706.554286,
    //             "timestamp": 1650769890,
    //             "infos": [{
    //                 "price": 39711.5,
    //                 "weight": 1,
    //                 "exchangeName": "kucoin"
    //             }, {
    //                 "price": 39709.2,
    //                 "weight": 2,
    //                 "exchangeName": "huobi"
    //             }, {
    //                 "price": 39707.34,
    //                 "weight": 1,
    //                 "exchangeName": "binance"
    //             }, {
    //                 "price": 39702.88,
    //                 "weight": 3,
    //                 "exchangeName": "coinbase"
    //             }]
    //         },
    //         "ethusdt": {
    //             "price": 2943.7075,
    //             "timestamp": 1650769886,
    //             "infos": [{
    //                 "price": 2943.96,
    //                 "weight": 1,
    //                 "exchangeName": "kucoin"
    //             }, {
    //                 "price": 2943.84,
    //                 "weight": 1,
    //                 "exchangeName": "huobi"
    //             }, {
    //                 "price": 2943.63,
    //                 "weight": 1,
    //                 "exchangeName": "binance"
    //             }, {
    //                 "price": 2943.4,
    //                 "weight": 1,
    //                 "exchangeName": "coinbase"
    //             }]
    //         }
    //     }
    // }
    // "#;

    let storage = InMemOffchainStorage::default();
    let offchain = AresToolsStruct::new(storage, DenyUnsafe::No, Role::Authority);

    let request_domain_str =  String::from("http://api.aresprotocol.io");
    offchain.set_warehouse(request_domain_str.clone());

    let res = offchain.try_get_request();

    let data = res.unwrap().get_request_body().unwrap();
    let json_obj : JsonValue = serde_json::from_str(data.as_str()).unwrap();

    assert_eq!(json_obj["code"].to_string(), "0");
    assert_eq!(json_obj["message"].as_str(), Some("OK"));
    assert!(json_obj["data"].as_object().unwrap().get("btcusdt").is_some());
    assert!(json_obj["data"].as_object().unwrap().get("ethusdt").is_some());
}