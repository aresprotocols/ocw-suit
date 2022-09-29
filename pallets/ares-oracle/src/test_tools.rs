use sp_runtime::BoundedVec;
use sp_runtime::traits::Get;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

pub fn to_test_vec(input: &str) -> Vec<u8> {
    input.as_bytes().to_vec()
}

pub fn to_test_bounded_vec<MaxLen: Get<u32>>(to_str: &str) -> BoundedVec<u8, MaxLen> {
    to_str.as_bytes().to_vec().try_into().unwrap()
}

pub fn get_bug_json_of_20220721() -> &'static str {
    "{\"code\":0,\"message\":\"OK\",\"data\":{\"algousdt\":{\"price\":0.3395,\"timestamp\":1658390392,\"infos\":[{\"price\":0.3395,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"axsusdt\":{\"price\":15.294,\"timestamp\":1658390345,\"infos\":[{\"price\":15.302,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":15.3,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":15.28,\"weight\":1,\"exchangeName\":\"coinbase\"}]},\"batusdt\":{\"price\":0.38925,\"timestamp\":1658390376,\"infos\":[{\"price\":0.3894,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":0.3891,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"bntusdt\":{\"price\":0.5016,\"timestamp\":1658390339,\"infos\":[{\"price\":0.502,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":0.5012,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"bttusdt\":{\"price\":8.934e-7,\"timestamp\":1658390367,\"infos\":[{\"price\":8.94e-7,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":8.928e-7,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"celousdt\":{\"price\":0.9415,\"timestamp\":1658390392,\"infos\":[{\"price\":0.9415,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"chzusdt\":{\"price\":0.107811,\"timestamp\":1658390370,\"infos\":[{\"price\":0.107893,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":0.1078,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":0.10774,\"weight\":1,\"exchangeName\":\"bitfinex\"}]},\"crvusdt\":{\"price\":1.163,\"timestamp\":1658390393,\"infos\":[{\"price\":1.163,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":1.163,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"dcrusdt\":{\"price\":24.2783,\"timestamp\":1658390358,\"infos\":[{\"price\":24.3,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":24.2566,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"egldusdt\":{\"price\":54.79,\"timestamp\":1658390393,\"infos\":[{\"price\":54.79,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"enjusdt\":{\"price\":0.590397,\"timestamp\":1658390371,\"infos\":[{\"price\":0.5907,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":0.5903,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":0.59019,\"weight\":1,\"exchangeName\":\"bitfinex\"}]},\"fetusdt\":{\"price\":0.0818,\"timestamp\":1658390345,\"infos\":[{\"price\":0.0818,\"weight\":1,\"exchangeName\":\"coinbase\"}]},\"ftmusdt\":{\"price\":0.30523,\"timestamp\":1658390364,\"infos\":[{\"price\":0.30526,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":0.3052,\"weight\":1,\"exchangeName\":\"binance\"}]},\"fttusdt\":{\"price\":28.2704,\"timestamp\":1658390364,\"infos\":[{\"price\":28.2808,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":28.26,\"weight\":1,\"exchangeName\":\"binance\"}]},\"grtusdt\":{\"price\":0.104105,\"timestamp\":1658390356,\"infos\":[{\"price\":0.104179,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":0.10403,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"hbarusdt\":{\"price\":0.0698,\"timestamp\":1658390392,\"infos\":[{\"price\":0.0698,\"weight\":1,\"exchangeName\":\"binance\"}]},\"icpusdt\":{\"price\":6.7438,\"timestamp\":1658390338,\"infos\":[{\"price\":6.75,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":6.7414,\"weight\":1,\"exchangeName\":\"bitfinex\"},{\"price\":6.74,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"icxusdt\":{\"price\":0.28865,\"timestamp\":1658390360,\"infos\":[{\"price\":0.289,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":0.2883,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"iostusdt\":{\"price\":0.013583,\"timestamp\":1658390393,\"infos\":[{\"price\":0.013583,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"iotausdt\":{\"price\":0.2901,\"timestamp\":1658390391,\"infos\":[{\"price\":0.2901,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"iotxusdt\":{\"price\":0.03337,\"timestamp\":1658390344,\"infos\":[{\"price\":0.03337,\"weight\":1,\"exchangeName\":\"binance\"}]},\"kavausdt\":{\"price\":1.7587,\"timestamp\":1658390392,\"infos\":[{\"price\":1.7587,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"kncusdt\":{\"price\":1.40765,\"timestamp\":1658390348,\"infos\":[{\"price\":1.4083,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":1.407,\"weight\":1,\"exchangeName\":\"binance\"}]},\"ksmusdt\":{\"price\":59.3564,\"timestamp\":1658390365,\"infos\":[{\"price\":59.4047,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":59.3081,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"lrcusdt\":{\"price\":0.417533,\"timestamp\":1658390357,\"infos\":[{\"price\":0.4177,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":0.4176,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":0.4173,\"weight\":1,\"exchangeName\":\"coinbase\"}]},\"manausdt\":{\"price\":0.90396,\"timestamp\":1658390392,\"infos\":[{\"price\":0.90396,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"mkrusdt\":{\"price\":962.05,\"timestamp\":1658390377,\"infos\":[{\"price\":962.1,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":962,\"weight\":1,\"exchangeName\":\"binance\"}]},\"nanousdt\":{\"price\":2.224,\"timestamp\":1658390393,\"infos\":[{\"price\":2.224,\"weight\":1,\"exchangeName\":\"binance\"}]},\"nearusdt\":{\"price\":4.174,\"timestamp\":1658390391,\"infos\":[{\"price\":4.174,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"neousdt\":{\"price\":9.56265,\"timestamp\":1658390345,\"infos\":[{\"price\":9.5653,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":9.56,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"omgusdt\":{\"price\":1.87565,\"timestamp\":1658390354,\"infos\":[{\"price\":1.876,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":1.8753,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"ontusdt\":{\"price\":0.2452,\"timestamp\":1658390392,\"infos\":[{\"price\":0.2452,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"qtumusdt\":{\"price\":3.0403,\"timestamp\":1658390358,\"infos\":[{\"price\":3.0406,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":3.04,\"weight\":1,\"exchangeName\":\"binance\"}]},\"renusdt\":{\"price\":0.146223,\"timestamp\":1658390392,\"infos\":[{\"price\":0.146223,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"sandusdt\":{\"price\":1.32313,\"timestamp\":1658390389,\"infos\":[{\"price\":1.32316,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":1.3231,\"weight\":1,\"exchangeName\":\"binance\"}]},\"scusdt\":{\"price\":0.004251,\"timestamp\":1658390359,\"infos\":[{\"price\":0.004252,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":0.00425,\"weight\":1,\"exchangeName\":\"binance\"}]},\"srmusdt\":{\"price\":0.9985,\"timestamp\":1658390391,\"infos\":[{\"price\":0.999,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":0.998,\"weight\":1,\"exchangeName\":\"binance\"}]},\"stxusdt\":{\"price\":0.423,\"timestamp\":1658390345,\"infos\":[{\"price\":0.423,\"weight\":1,\"exchangeName\":\"coinbase\"}]},\"sushiusdt\":{\"price\":1.3205,\"timestamp\":1658390385,\"infos\":[{\"price\":1.3205,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"thetausdt\":{\"price\":1.2205,\"timestamp\":1658390392,\"infos\":[{\"price\":1.2205,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"umausdt\":{\"price\":2.6182,\"timestamp\":1658390392,\"infos\":[{\"price\":2.6182,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"vetusdt\":{\"price\":0.024886,\"timestamp\":1658390343,\"infos\":[{\"price\":0.02489,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":0.024882,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"wavesusdt\":{\"price\":5.537,\"timestamp\":1658390392,\"infos\":[{\"price\":5.537,\"weight\":1,\"exchangeName\":\"binance\"}]},\"xemusdt\":{\"price\":0.0472,\"timestamp\":1658390392,\"infos\":[{\"price\":0.0472,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"xlmusdt\":{\"price\":0.111627,\"timestamp\":1658390355,\"infos\":[{\"price\":0.11166,\"weight\":1,\"exchangeName\":\"bitfinex\"},{\"price\":0.11162,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":0.1116,\"weight\":1,\"exchangeName\":\"binance\"}]},\"xmrusdt\":{\"price\":151.605,\"timestamp\":1658390364,\"infos\":[{\"price\":151.61,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":151.6,\"weight\":1,\"exchangeName\":\"binance\"}]},\"xtzusdt\":{\"price\":1.604765,\"timestamp\":1658390345,\"infos\":[{\"price\":1.60553,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":1.604,\"weight\":1,\"exchangeName\":\"binance\"}]},\"yfiusdt\":{\"price\":6385.405,\"timestamp\":1658390392,\"infos\":[{\"price\":6387.18,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":6383.63,\"weight\":1,\"exchangeName\":\"kucoin\"}]},\"zecusdt\":{\"price\":60.97,\"timestamp\":1658390394,\"infos\":[{\"price\":61,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":60.94,\"weight\":1,\"exchangeName\":\"huobi\"}]},\"zenusdt\":{\"price\":16.6615,\"timestamp\":1658390343,\"infos\":[{\"price\":16.663,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":16.66,\"weight\":1,\"exchangeName\":\"binance\"}]},\"zilusdt\":{\"price\":0.040255,\"timestamp\":1658390374,\"infos\":[{\"price\":0.04026,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":0.04025,\"weight\":1,\"exchangeName\":\"binance\"}]},\"zrxusdt\":{\"price\":0.3115,\"timestamp\":1658390356,\"infos\":[{\"price\":0.3115,\"weight\":1,\"exchangeName\":\"binance\"}]}}}"
}

pub fn get_are_json_of_btc() -> &'static str {
    "{\"code\":0,\"message\":\"OK\",\"data\":{\"price\":50261.372,\"timestamp\":1629699168,\"infos\":[{\"price\":50244.79,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":50243.16,\"weight\":1,\"exchangeName\":\"cryptocompare\"},{\"price\":50274,\"weight\":1,\"exchangeName\":\"bitfinex\"},{\"price\":50301.59,\"weight\":1,\"exchangeName\":\"bitstamp\"},{\"price\":50243.32,\"weight\":1,\"exchangeName\":\"huobi\"}]}}"
}

pub fn get_are_json_of_eth() -> &'static str {
    "{\"code\":0,\"message\":\"OK\",\"data\":{\"price\":3107.71,\"timestamp\":1630055777,\"infos\":[{\"price\":3107,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":3106.56,\"weight\":1,\"exchangeName\":\"cryptocompare\"},{\"price\":3106.68,\"weight\":1,\"exchangeName\":\"ok\"},{\"price\":3107,\"weight\":1,\"exchangeName\":\"bitfinex\"},{\"price\":3111.31,\"weight\":1,\"exchangeName\":\"bitstamp\"}]}}"
}

pub fn get_are_json_of_dot() -> &'static str {
    "{\"code\":0,\"message\":\"OK\",\"data\":{\"price\":35.9921,\"timestamp\":1631497660,\"infos\":[{\"price\":36.0173,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":36.012,\"weight\":1,\"exchangeName\":\"coinbase\"},{\"price\":35.947,\"weight\":1,\"exchangeName\":\"bitfinex\"}]}}"
}

pub fn get_are_json_of_xrp() -> &'static str {
    "{\"code\":0,\"message\":\"OK\",\"data\":{\"price\":1.09272,\"timestamp\":1631497987,\"infos\":[{\"price\":1.09319,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":1.0922,\"weight\":1,\"exchangeName\":\"bitfinex\"},{\"price\":1.09277,\"weight\":1,\"exchangeName\":\"ok\"}]}}"
}

// {"code":0,"message":"OK","data":{"btcusdt":{"price":50261.372,"timestamp":1629699168},"ethusdt":
// {"price":3107.71,"timestamp":1630055777},"dotusdt":{"price":35.9921,"timestamp":1631497660},"
// xrpusdt":{"price":1.09272,"timestamp":1631497987}}}
pub fn get_are_json_of_bulk() -> &'static str {
    "{\"code\":0,\"message\":\"OK\",\"data\":{\"btcusdt\":{\"price\":50261.372,\"timestamp\":1629699168},\"ethusdt\":{\"price\":3107.71,\"timestamp\":1630055777},\"dotusdt\":{\"price\":35.9921,\"timestamp\":1631497660},\"xrpusdt\":{\"price\":1.09272,\"timestamp\":1631497987}}}"
}

pub fn get_are_dot_eth_btc() -> &'static str {
    "{\"code\":0,\"message\":\"OK\",\"data\":{\"btcusdt\":{\"price\":23286.141429,\"timestamp\":1658479119,\"infos\":[{\"price\":23289.23,\"weight\":2,\"exchangeName\":\"huobi\"},{\"price\":23287.4,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":23285.01,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":23284.04,\"weight\":3,\"exchangeName\":\"coinbase\"}]},\"dotusdt\":{\"price\":7.741333,\"timestamp\":1658479124,\"infos\":[{\"price\":7.7443,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":7.7417,\"weight\":1,\"exchangeName\":\"kucoin\"},{\"price\":7.738,\"weight\":1,\"exchangeName\":\"bitfinex\"}]},\"ethusdt\":{\"price\":1609.2625,\"timestamp\":1658479144,\"infos\":[{\"price\":1609.45,\"weight\":1,\"exchangeName\":\"huobi\"},{\"price\":1609.38,\"weight\":1,\"exchangeName\":\"binance\"},{\"price\":1609.18,\"weight\":1,\"exchangeName\":\"bitstamp\"},{\"price\":1609.04,\"weight\":1,\"exchangeName\":\"coinbase\"}]}}}"
}

// {"code":0,"message":"OK","data":{"btcusdt":{"price":50261.372,"timestamp":1629699168},"ethusdt":
// {"price":3107.71,"timestamp":1630055777},"dotusdt":{"price":35.9921,"timestamp":1631497660},"
// xrpusdt":{"price":1.09272,"timestamp":1631497987},"xxxusdt":{"price":1.09272,"timestamp":
// 1631497987}}}
pub fn get_are_json_of_bulk_of_xxxusdt_is_0() -> &'static str {
    "{\"code\":0,\"message\":\"OK\",\"data\":{\"btcusdt\":{\"price\":50261.372,\"timestamp\":1629699168},\"ethusdt\":{\"price\":3107.71,\"timestamp\":1630055777},\"dotusdt\":{\"price\":35.9921,\"timestamp\":1631497660},\"xrpusdt\":{\"price\":1.09272,\"timestamp\":1631497987},\"xxxusdt\":{\"price\":0,\"timestamp\":1631497987}}}"
}
