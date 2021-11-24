use frame_system::offchain::{SignedPayload, SigningTypes};
use ares_oracle::types::FractionLength;
use sp_runtime::Permill;
use super::*;
// use crypto::{digest::Digest, sha3::Sha3};
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


pub fn eth_checksum(address: &[u8]) -> &str {
    // let address = address.trim_start_matches("0x").to_lowercase();
    // let address_hash = {
    //     let mut hasher = Sha3::keccak256();
    //     hasher.input(address);
    //     hasher.result_str()
    // };
    //
    // let mut acc: Vec<char> = vec![];
    // for (index, x) in address.iter().enumerate() {
    //     let n = u16::from_str_radix(&address_hash[index..index + 1], 16).unwrap();
    //     let c = char::from(*x);
    //     c.to
    //     if n > 7 {
    //         // make char uppercase if ith character is 9..f
    //         acc.push(c.to_ascii_uppercase())
    //     } else {
    //         // already lowercased
    //         acc.push(c)
    //     }
    // }

    return str::from_utf8(b"aaa").unwrap();
}
