// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use sp_runtime::Perbill;
use std::str;
use sp_runtime::traits::{Keccak256, Hash};
use sp_std::str as sp_str;
use crypto::{digest::Digest, sha3::Sha3};
use sp_std::convert::TryInto;
use sp_core::hexdisplay::HexDisplay;
use std::array::TryFromSliceError;
use codec::Encode;

#[test]
fn test_a() {
    let a = b"e0fc04fa2d34a66b779fd5cee748268032a146c0";
    let result = checksum(a);
    let _tmp = str::from_utf8(a).unwrap();
    println!("{:?}", result);
    let t = Keccak256::hash(a);

    // println!("substrate: {:?}", t);
    assert!(result.eq_ignore_ascii_case(_tmp));
    checksum2(a);
}

pub fn checksum(address: &[u8]) -> String {
    // let address = address.trim_start_matches("0x").to_lowercase();
    let address_hash = {
        let mut hasher = Sha3::keccak256();
        hasher.input(address);
        hasher.result_str()
    };
    println!("address_hash {:?}", &address_hash);
    println!("address_hash bytes {:?}", &address_hash.as_bytes());

    let mut acc: String = String::from("");
    for (index, x) in address.iter().enumerate() {
        let n = u16::from_str_radix(&address_hash[index..index + 1], 16).unwrap();
        // let _a: u8 = *x;
        let c = char::from(*x);
        // println!("test_index: {:?}, n: {:?}, original: {:?}", index, n, x);
        if n > 7 {
            // make char uppercase if ith character is 9..f
            acc.push_str(&c.to_uppercase().to_string())
        } else {
            // already lowercased
            acc.push(c)
        }
    }
    let a = acc.as_bytes();
    // println!("a: {:?}", a);
    return acc;
}

pub fn checksum2(address: &[u8]) -> &str{

    let address_hash  = Keccak256::hash(address);
    println!("checksum2 address_hash {:?}", &address_hash);
    // use hex::ToHex;
    println!("checksum2 address_hash_bytes {:?}", &address_hash.as_bytes());
    // println!("b checksum2 address_hash_bytes {:?}", sp_str::from_utf8(&address_hash.as_bytes()).unwrap().as_bytes());
    // println!("c checksum2 address_hash_bytes {:?}", b"1fdff09c3722676b8b90748d7bd13d9a8352431127fc881637b5b561b537a57d");

    // let address_bytes = address_hash.as_bytes();
    // println!("bytes address_hash {:?}", &address_bytes);
    let mut acc: Vec<u8> = vec![];
    // let a = &mut acc;
    let address_hash = b"1fdff09c3722676b8b90748d7bd13d9a8352431127fc881637b5b561b537a57d";
    for (index, x) in address.iter().enumerate() {
        // let n = u16::from_str_radix(&address_hash[index..index + 1], 16).unwrap();
        //let k: &[u8] = &address_bytes[index..index + 1];
        // println!("test1: {:?}", k);
        // let r:Result<[u8;2],TryFromSliceError> = k.try_into();
        // println!("test: {:?}", r);
        // let _tmp: [u8; 2] = r.unwrap();
        let n = u16::from(*x);
        // let c = char::from(*x);
        // println!("test_index: {:?}, n: {:?}, original: {:?}", index, n, x);
        if n > 7{

            let a = *x;
            let mut _tmp = a.clone();
            _tmp.make_ascii_uppercase();
            acc.push(_tmp);
        }else{
            acc.push(*x)
        }
    }
    let a = acc.as_slice();
    // println!("a: {:?}", a);
    // println!("hex Display: {:?}", HexDisplay::from(acc.as_slice()));
    return sp_str::from_utf8(b"a").unwrap();
}
