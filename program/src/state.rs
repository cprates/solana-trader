use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct AccountTemp {
    pub authority: Pubkey,
    pub offer_amount: u64,
    pub trade_amount: u64,
}

impl AccountTemp {
    pub fn size() -> usize {
        // TODO: could have a const to save compute unites when executed on-chain
        let encoded = AccountTemp::default()
            .try_to_vec().unwrap();

        encoded.len()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, Default)]
pub struct AccountTrade {
    pub bump_seed: u8,
    pub offer_token_account: Pubkey,
    pub authority: Pubkey,
    pub offer_amount: u64,
    pub trade_amount: u64,
    pub initialized: bool,
    pub trade_mint: Pubkey,
    pub program_id: Pubkey,
}

impl AccountTrade {
    pub fn size() -> usize {
        // TODO: could have a const to save compute unites when executed on-chain
        let encoded = AccountTrade::default()
            .try_to_vec().unwrap();

        encoded.len()
    }
}
