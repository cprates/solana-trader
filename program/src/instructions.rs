use borsh::{BorshDeserialize, BorshSerialize};

// 0. `[signer]` Account of the owner of the trade
// 1. `[writable]` trade account
// 2. `[]` mint account of the offer
// 3. `[]` the account to store the trade amount
// 4. `[writable]` temp account
// 5. `[]` trade program id
// 6. `[]` system program id
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum Action {
    CreateTrade{ 
        bump_seed: u8,
        offer: u64,
        trade: u64,
    },
}
