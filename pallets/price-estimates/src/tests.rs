use codec::Encode;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use sp_core::Public;
use sp_runtime::traits::AccountIdConversion;

#[test]
fn test_subaccount() {

    let acc: AccountId = EstimatesPalletId.into_account();
    println!("ACC: {:?}", acc );

    let uni_usdt: AccountId = EstimatesPalletId.into_sub_account("uni-usdt".as_bytes().to_vec());
    // 6d6f 64 6c70792f61 7265737424 20756e692d75736474 00000000000000000000
    // 6d6f646c70792f61726573742420756e692d7573647400000000000000000000
    println!("uni_usdt 1 = {:?}, {:?}", uni_usdt, "uni-usdt".as_bytes().to_vec());

    let uni_usdt: AccountId = EstimatesPalletId.into_sub_account(b"uni-usdt");
    // 6d6f 64 6c70792f61 7265737424 20756e692d75736474 00000000000000000000
    // 6d6f646c70792f61726573742420756e692d7573647400000000000000000000
    println!("uni_usdt 2 = {:?}, {:?}", uni_usdt, b"uni-usdt");


    println!("---------------------------");

    // let btc_usdc: AccountId = EstimatesPalletId.into_sub_account("btc-usdt".bytes().map(|b|{
    //     println!("b={:?}", b);
    //     b
    // }).collect::<Vec<u8>>());

    println!("{:?}", b"btc-usdt");
    let b_vec = b"btc-usdt".map(|x|{
        println!("-{:?}", x);
        x
    }).to_vec();
    println!("{:?}", b_vec);
    println!("{:?}", "btc-usdt".as_bytes().to_vec());

    // let btc_usdc: AccountId = EstimatesPalletId.into_sub_account(b"btc-usdt");
    let btc_usdc: AccountId = EstimatesPalletId.into_sub_account("btc-usdt".as_bytes());
    // 6d6f646c70792f6172657374206274632d757364740000000000000000000000 ERROR
    // 6d6f646c70792f61726573746274632d75736474000000000000000000000000 RIGTH

    println!("ENCODE A={:?}", "btc-usdt".encode());
    println!("ENCODE A={:?}", "btc-usdt".as_bytes().encode());

    // ---- success
    let left: AccountId = EstimatesPalletId.into_sub_account(b"btc-usdt");

    // // -------- failed
    // let right: AccountId = EstimatesPalletId.into_sub_account("btc-usdt");
    // // -------- failed
    // let right: AccountId = EstimatesPalletId.into_sub_account("btc-usdt".as_bytes());
    // // -------- failed
    // let right: AccountId = EstimatesPalletId.into_sub_account("btc-usdt".as_bytes().to_vec());

    // --- ok
    let mut u8list: [u8; 8] = [0; 8]; /// .. "btc-usdt".as_bytes();
    // u8list.copy_from_slice("btc-usdt".as_bytes());
    u8list.copy_from_slice("btc-usdt".as_bytes().to_vec().as_slice());
    let right: AccountId = EstimatesPalletId.into_sub_account(u8list);
    println!("btc-usdt hex = {:?}", right);
    assert_eq!(
        left,
        right
    );
    // ---------

    // JS:      0x6d6f646c70792f6172657374616176652d757364740000000000000000000000
    // RUST:      6d6f646c70792f6172657374616176652d757364740000000000000000000000
    let left: AccountId = EstimatesPalletId.into_sub_account(b"aave-usdt");
    let mut u8list: [u8; 20] = [0; 20]; /// .. "btc-usdt".as_bytes();
    // u8list.copy_from_slice("btc-usdt".as_bytes());
    u8list[.."aave-usdt".len()].copy_from_slice("aave-usdt".as_bytes().to_vec().as_slice());
    let right: AccountId = EstimatesPalletId.into_sub_account(u8list);
    println!("aave-usdt hex = {:?}", right);
    assert_eq!(
        left,
        right
    );

    println!("btc-usdt = {:?}", btc_usdc);

    let aave_usdt: AccountId = EstimatesPalletId.into_sub_account("aave-usdt".as_bytes().to_vec());
    println!("ERROR : aave_usdt = {:?}", aave_usdt);

    let aave_usdc: AccountId = EstimatesPalletId.into_sub_account("aave-usdc".as_bytes().to_vec());
    println!("ERROR : aave_usdc = {:?}", aave_usdc);

    let aavel_usdc: AccountId = EstimatesPalletId.into_sub_account("aavel-usdc".as_bytes().to_vec());
    println!("ERROR : aavel_usdc = {:?}", aavel_usdc);



    // uni_usdt =  6d6f646c70792f6172657374  2420  756e69  2d    7573647400000000000000000000 (5EYCAe5i...)
    // btc-usdt =  6d6f646c70792f6172657374  2420  627463  2d    7573647400000000000000000000 (5EYCAe5i...)
    // aave_usdt = 6d6f646c70792f6172657374  2824  61617665  2d  75736474000000000000000000 (5EYCAe5i...)
    // aave_usdc = 6d6f646c70792f6172657374  2824  61617665  2d  75736463000000000000000000 (5EYCAe5i...)



    // BTC
    // 0x6d6f646c70792f6172657374  206274632d7573647400000000000000000000 00
    //   6d6f646c70792f617265737424206274632d7573647400000000000000000000
    // 0x6d6f646c70792f6172657374    6274632d75736474000000000000000000000000
}

#[test]
fn test_subaccount2() {

    let acc: AccountId = EstimatesPalletId.into_account();
    println!("ACC: {:?}", acc );

    let symbol="a";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="ab";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abc";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcd";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcde";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdef";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefg";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefgh";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefghi";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefghij";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

    let symbol="abcdefghijk";
    let show: AccountId = EstimatesPalletId.into_sub_account(symbol.as_bytes().to_vec());
    println!("{:?} = {:?}", symbol, show);

}