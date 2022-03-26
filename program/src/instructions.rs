//use crate::state::Currency;
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]

// 0. `[signer]` Account of the owner of the trade
// 1. `[writable]` trade account
// 2. `[]` mint account of the offer
// 3. `[]` the account to store the trade amount
// 4. `[]` temp account
// 5. `[]` trade program id
pub enum Action {
    CreateTrade{ 
        offer: u64,
        trade: u64,
    },
}
