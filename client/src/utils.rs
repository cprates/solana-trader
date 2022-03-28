use crate::{Error, Result};
use solana_client::rpc_client::RpcClient;
use solana_program::program_pack::Pack;
use solana_sdk::signer::keypair::{read_keypair_file, Keypair};
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    system_instruction,
};
use spl_token::state::{
    Account,
    Mint,
};
use std::str::FromStr;
use yaml_rust::YamlLoader;

pub fn load_config() -> Result<yaml_rust::Yaml> {
    let path = match home::home_dir() {
        Some(mut path) => {
            path.push(".config/solana/cli/config.yml");
            path
        }
        None => {
            return Err(Error::ConfigReadError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "failed to locate homedir and thus can not locoate solana config",
            )));
        }
    };
    let config = std::fs::read_to_string(path).map_err(|e| Error::ConfigReadError(e))?;
    let mut config = YamlLoader::load_from_str(&config).unwrap();
    match config.len() {
        1 => Ok(config.remove(0)),
        l => Err(Error::InvalidConfig(format!(
            "expected one yaml document got ({})",
            l
        ))),
    }
}

pub fn get_wallet(maybe_path: Option<&String>) -> Result<Keypair> {
    let config = load_config()?;

    let path = match maybe_path {
        Some(s) => s,
        None => {
            let p = match config["keypair_path"].as_str() {
                Some(s1) => s1,
                None => {
                    return Err(Error::InvalidConfig(
                        "missing `keypair_path` field".to_string(),
                    ))
                }
            };
            p
        }
    };
    
    read_keypair_file(path).map_err(|e| {
        Error::InvalidConfig(format!("failed to read keypair file ({}): ({})", path, e))
    })
}

pub fn create_mint_ix(authority: &Pubkey, mint_pubkey: &Pubkey, conn: &RpcClient) -> Vec<Instruction> {
    let create_ix = system_instruction::create_account(
        authority,
        mint_pubkey,
        conn.get_minimum_balance_for_rent_exemption(Mint::LEN).unwrap(),
        Mint::LEN as u64,
        &spl_token::id(),
    );

    let init_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(), 
        mint_pubkey,
        authority, 
        None, 
        9,
    ).unwrap();

    Vec::from([create_ix, init_ix])
}

pub fn create_account_ix(
    authority: &Pubkey, 
    mint_pubkey: &Pubkey,
    account_pubkey: &Pubkey, 
    conn: &RpcClient,
) -> Vec<Instruction> {
    let create_ix = system_instruction::create_account(
        authority,
        account_pubkey,
        conn.get_minimum_balance_for_rent_exemption(Account::LEN).unwrap(),
        Account::LEN as u64,
        &spl_token::id(),
    );

    let init_ix = spl_token::instruction::initialize_account(
        &spl_token::id(), 
        account_pubkey,
        mint_pubkey,
        authority, 
    ).unwrap();

    Vec::from([create_ix, init_ix])
}

// Copied  from https://github.com/solana-labs/solana-program-library/blob/b7a3fc62431fcd00001df625aaa61a29ce7d1e29/token/cli/src/main.rs#L599
// and adapted
pub fn resolve_mint_info(
    token_account: &Pubkey,
    mint_address: Option<Pubkey>,
    conn: &RpcClient,
) -> Result<u8> {
    let source_account = conn
        .get_token_account(token_account).unwrap()
        .ok_or_else(|| format!("Could not find token account {}", token_account)).unwrap();

    let source_mint = Pubkey::from_str(&source_account.mint).unwrap();
    if let Some(mint) = mint_address {
        if source_mint != mint {
            return Err(Error::InvalidConfig(format!(
                "Source {:?} does not contain {:?} tokens",
                token_account, mint
            )));
        }
    }
    Ok(source_account.token_amount.decimals)
}
