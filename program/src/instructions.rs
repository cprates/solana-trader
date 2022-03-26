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

    // 0. `[signer]` Account of the person accepting the trade (user B)
    // 1. `[writable]` trade account
    // 2. `[writable]` pda account    
    // 3. `[writable]` the token account to store the trade amount in (user A)
    // 4. `[writable]` the token account to get the trade amount from (user B)
    // 5. `[writable]` the token account to store the offer amount in (user B)
    // 6. `[]` token program id
    // 7. `[]` trade program id    
    MakeTrade{ 
        expected_offer: u64,
        expected_trade: u64,
    },
}
