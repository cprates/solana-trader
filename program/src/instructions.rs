use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum Action {
    // 0. `[signer]` Account of the owner of the trade
    // 1. `[writable]` trade account
    // 2. `[]` token account - the account holding the offer amount
    // 3. `[writable]` pda account
    // 4. `[]` token program account
    CreateTrade{ 
        bump_seed: u8,
        trade: u64,
    },
}
