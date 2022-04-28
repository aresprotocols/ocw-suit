use std::ops::Range;
use super::*;
use super::*;
use assert_matches::assert_matches;
use frame_support::BoundedVec;
use frame_support::sp_std::convert::TryFrom;
use frame_support::traits::ConstU32;
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
    let request_domain = Bytes(b"http://aresprotocol.io/a".to_vec().encode());

    assert_matches!(
		offchain.set_local_storage(StorageKind::PERSISTENT, request_key, request_domain.clone()),
		Ok(())
	);

    let ware_house = offchain.get_warehouse();
    assert!(ware_house.is_ok());
    let ware_house = ware_house.unwrap();
    assert!(ware_house.is_some());
    let ware_house = ware_house.unwrap();
    assert_eq!(ware_house, "http://aresprotocol.io/a".to_string());

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Encode, Decode)]
struct MyStr {
    ref_u8: Vec<u8>,
}

impl MyStr {
    pub fn new(ref_u8: Vec<u8>) -> Self {
        Self{
            ref_u8
        }
    }
}

// #[test]
// fn test_domain_encode() {
//
//     let request_base_str = "http://www.google.com";
//
//     let my_str = MyStr::new(request_base_str.as_bytes().to_vec());
//     let mut encode_str = my_str.encode();
//     println!("my_str A = {:?}", encode_str);
//
//     pub type MaximumPoolSize = ConstU32<1000>;
//     pub type RequestBaseVecU8 = BoundedVec<u8, MaximumPoolSize>;
//     let mut bound_vec = RequestBaseVecU8::try_from(encode_str).unwrap();
//
//     // let second_str = RequestBaseVecU8::decode(&mut encode_str);
//
//     let second_str = MyStr::decode(&mut &bound_vec[..]);
//     println!("second_str = {:?}", second_str);
//
//
//     let encode_u8 = request_base_str.encode(); // --
//     let encode_u8 = encode_u8.as_slice(); // --
//     println!("encode_u8 ENCODE:: = {:?}", encode_u8); // --
//
//     let decode_u8 = Vec::<u8>::decode(&mut &encode_u8[..]);
//     println!("decode_u8 DECODE:: = {:?}", decode_u8);
//
//     let mut bytes_u8 = request_base_str.encode(); // --
//     // let decode_u8 = String::decode(&mut bytes_u8);
//
//     let range = Range { start: 1, end: 100 };
//     let range_bytes = (1, 100).encode();
//     assert_eq!(range.encode(), range_bytes);
//     assert_eq!(Range::decode(&mut &range_bytes[..]), Ok(range));
//
//     println!("bytes_u8 = {:?}", bytes_u8); // --
//     // println!("dcode_u8 = {:?}", decode_u8); // --
//
//     // let result_str = String::from_utf8();
//     // println!("result_str = {:?}", result_str);
//
// }
