use super::*;
use frame_system::offchain::{SignedPayload, SigningTypes};
use hex::ToHex;
use pallet_ocw::types::FractionLength;
use sp_runtime::{
    traits::{Hash, Keccak256},
    Permill,
};
use sp_std::str;

#[derive(PartialEq, Eq, Clone, RuntimeDebug, Encode, Decode)]
pub enum EstimatesState {
    InActive,

    Active,

    WaitingPayout,

    Completed,
}

impl Default for EstimatesState {
    fn default() -> Self {
        EstimatesState::InActive
    }
}

#[derive(Encode, Decode, Clone, Default, Eq, PartialEq, RuntimeDebug)]
pub struct SymbolEstimatesConfig<BlockNumber, Balance> {
    pub symbol: Vec<u8>,

    ///
    pub id: u64,

    /// Price per entry.
    pub price: Balance,
    /// Starting block of the estimates.
    pub start: BlockNumber,
    /// Length of the estimates (start + length = end).
    pub length: BlockNumber,
    /// Delay for payout the winner of the estimates. (start + length + delay = payout).
    pub delay: BlockNumber,

    pub deviation: Permill,

    pub state: EstimatesState,

    pub total_reward: Balance,
}

#[derive(Encode, Decode, Clone, Default, Eq, PartialEq, RuntimeDebug)]
pub struct AccountParticipateEstimates<Account> {
    pub account: Account,

    pub estimates: u64,

    pub eth_address: Option<Vec<u8>>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct ChooseWinnersPayload<Public, AccountId, BlockNumber> {
    pub block_number: BlockNumber,
    pub winners: Vec<AccountParticipateEstimates<AccountId>>,
    pub public: Public,
    pub estimates_id: u64,
    pub symbol: Vec<u8>,
    pub price: (u64, FractionLength),
}

impl<T: SigningTypes> SignedPayload<T>
    for ChooseWinnersPayload<T::Public, T::AccountId, T::BlockNumber>
{
    fn public(&self) -> T::Public {
        self.public.clone()
    }
}

pub fn is_eth_address(address: &[u8]) -> bool {
    let _address = str::from_utf8(address).unwrap();

    // let basic = Regex::new(r"^(0x)?(?i)([0-9a-f]{40})$").unwrap();
    // let lowercase = Regex::new(r"^(0x|0X)?[0-9a-f]{40}$").unwrap();
    // let uppercase = Regex::new(r"^(0x|0X)?[0-9A-F]{40}$").unwrap();

    // check if it has the basic requirements of an address( case-insensitive )
    // if basic.find(address).is_none() {
    //     false
    //     // If it's ALL lowercase or ALL uppercase
    // } else if lowercase.find(address).is_some() || uppercase.find(address).is_some() {
    //     true
    // } else {
    //     eth_checksum(address)
    // }
    eth_checksum(address)
}

pub fn is_hex_address(address: &[u8]) -> bool {
    // log::info!("test-hex: {:?} ,length: {}", address, address.len());
    if address.len() != 40 {
        return false;
    }
    for (i, x) in address.iter().enumerate() {
        let c: char = char::from(*x);
        /*if i < 2 {
            // check 0x prefix
            if !((i == 0 && c == '0') || (i == 1 && c == 'x') || (i == 1 && c == 'X')) {
                return false;
            }
        } else {*/
        if !(('0' <= c && c <= '9') || ('a' <= c && c <= 'f') || ('A' <= c && c <= 'F')) {
            return false;
        }
        //}
    }
    true
}

fn eth_checksum(address: &[u8]) -> bool {
    let _address = address.to_ascii_lowercase();
    let address_hash = Keccak256::hash(_address.as_slice());
    let address_hash_bytes: Vec<char> = address_hash.encode_hex();
    let address_hash_bytes = address_hash_bytes.as_slice();
    // println!("checksum2 address_hash {:?}", &address_hash);
    // println!("checksum2 address_hash_bytes {:?}", address_hash_bytes);

    for (index, x) in address.iter().enumerate() {
        let c = address_hash_bytes[index];
        let n = c.to_digit(16).unwrap();

        let a = *x;
        let mut _tmp = a.clone();
        if n > 7 {
            _tmp.make_ascii_uppercase();
            if _tmp != a {
                return false;
            }
        } else {
            _tmp.make_ascii_lowercase();
            if _tmp != a {
                return false;
            }
        }
    }
    // println!("true");
    return true;
    // return str::from_utf8(b"aaa").unwrap();
}
