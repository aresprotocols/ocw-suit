use super::*;
use frame_support::BoundedVec;
use frame_support::traits::{ConstU32, Currency, Get};
use sp_runtime::{RuntimeDebug};
use scale_info::TypeInfo;
use sp_std::fmt::Debug;
use sp_std::vec::Vec;
use codec::{Codec, Decode, Encode, MaxEncodedLen};

#[cfg(feature = "std")]
use serde::{self, Deserialize, Deserializer, Serialize, Serializer};

#[cfg(feature = "std")]
use sp_runtime::traits::Zero;

pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Ident vec.
pub type MaxIdenSize = ConstU32<20>;
pub type Ident = BoundedVec<u8, MaxIdenSize>;

// for cross-chain pending list
pub type MaximumPendingList = ConstU32<127>;
pub type CrossChainInfoList<T: Config> = BoundedVec<CrossChainInfo<Ident, CrossChainKind, BalanceOf<T>>, MaximumPendingList>;

//
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct CrossChainInfo<Iden, Type, Balance> {
    pub iden: Iden,
    pub kind: Type,
    pub amount: Balance
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum CrossChainKind {
    ETH(EthereumAddress),
    BSC(EthereumAddress),
}

impl CrossChainKind {
    pub fn verification_addr (&self) -> bool {
        let verify_eth = |addr| {
            addr != &EthereumAddress::default()
        };
        match self {
            CrossChainKind::ETH(addr) => {
                return verify_eth(addr);
            },
            CrossChainKind::BSC(addr) => {
                return verify_eth(addr);
            }
        }
        false
    }
}

/// An Ethereum address (i.e. 20 bytes, used to represent an Ethereum account).
///
/// This gets serialized to the 0x-prefixed hex representation.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, Default, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct EthereumAddress([u8; 20]);

impl EthereumAddress {
    pub fn new(add: [u8; 20]) -> EthereumAddress {
        Self(add)
    }
}

#[cfg(feature = "std")]
impl Serialize for EthereumAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let hex: String = rustc_hex::ToHex::to_hex(&self.0[..]);
        serializer.serialize_str(&format!("0x{}", hex))
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for EthereumAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
    {
        let base_string = String::deserialize(deserializer)?;
        let offset = if base_string.starts_with("0x") { 2 } else { 0 };
        let s = &base_string[offset..];
        if s.len() != 40 {
            Err(serde::de::Error::custom(
                "Bad length of Ethereum address (should be 42 including '0x')",
            ))?;
        }
        let raw: Vec<u8> = rustc_hex::FromHex::from_hex(s)
            .map_err(|e| serde::de::Error::custom(format!("{:?}", e)))?;
        let mut r = Self::default();
        r.0.copy_from_slice(&raw);
        Ok(r)
    }
}
