// This file is part of Substrate.

// Copyright (C) 2020-2022 Parity Technologies (UK) Ltd.
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

use crate as example_offchain_worker;
// use crate::*;
// use crate::mock::*;
use crate::mock::{AresId, Call, Estimates, Extrinsic, Origin, Test, };
use codec::Decode;
use frame_support::{
	assert_ok, parameter_types,
	traits::{ConstU32, ConstU64},
};
use sp_core::{
	offchain::{testing, OffchainWorkerExt, TransactionPoolExt},
	sr25519::Signature,
	H256,
};
use std::sync::Arc;
use frame_system::offchain::{SignedPayload, SigningTypes};

use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{
	testing::{Header, TestXt},
	traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
	RuntimeAppPublic,
};
use sp_runtime::traits::AppVerify;
fn test_pub() -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([1u8; 32])
}


