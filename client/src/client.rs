use borsh::BorshSerialize;
use solana_program::program_pack::Pack;
use solana_program::system_program;
use crate::{Error, Result};
use crate::utils::{
    get_wallet,
    load_config,
    create_mint_ix,
    create_account_ix,
    get_or_create_token_account,
};
use solana_client::{
    rpc_client::RpcClient,
    client_error::ClientErrorKind,
};
use spl_associated_token_account;
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentConfig,
    message::Message,
    instruction::{
        AccountMeta, 
        Instruction,
    },
    pubkey::Pubkey,
    system_instruction,
    signer::{
        keypair::Keypair,
        Signer,
    },
    transaction::Transaction,
};
use std::borrow::Borrow;
//use std::fmt::Result;
use std::str::FromStr;
use trader::{
    state,
    instructions::Action,
};

/*
 * Sets up all base accounts to test all oprations. Expects two wallets, wallet1 will hold Mint account A, wallet2
 * will hold Mintaccount B.
 * 
 * - Mint A
 * - Mint B
 * 
 * A (offer)
 *   - Token Account 1: Mint A. Offer src account which will be transfered to the trader program, with balance of 1000
 *   - Token account 2: Mint B. Trade dst account. If not provided an ATA will be used
 * 
 * B (trade)
 *   - Token Account 3: Mint A. Offer dst account. If not provided an ATA will be used
 *   - Token account 4: Mint B. Trade src account with a balance of 1000
 * 
*/
pub fn setup_accounts(
    offer: u64,
    trade: u64,
    wallet1: Keypair,
    wallet2: Keypair,
    conn: &RpcClient,
) -> Result<()> {
    let mut ixs = Vec::<Instruction>::new();
    // Mint A
    let mint_a_keypair = &Keypair::new();
    let create_mint_a_ix = create_mint_ix(
        &wallet1.pubkey(),
        &mint_a_keypair.pubkey(),
        conn,
    );
    ixs.extend(create_mint_a_ix);
    
    // Mint B
    let mint_b_keypair = &Keypair::new();
    let create_mint_b_ix = create_mint_ix(
        &wallet1.pubkey(),
        &mint_b_keypair.pubkey(),
        conn,
    );
    ixs.extend(create_mint_b_ix);

    let message = Message::new(ixs.as_ref(), Some(&wallet1.pubkey()));
    let transaction = Transaction::new(&[&wallet1, mint_a_keypair, mint_b_keypair], message, conn.get_latest_blockhash().unwrap());
    conn.send_and_confirm_transaction(&transaction).unwrap();

    println!("Mint accounts setup finished!");

    // Setup offer
    let mut ixs = Vec::<Instruction>::new();
    // Account 1
    let account_1_keypair = &Keypair::new();
    let create_account_1_ix = create_account_ix(
        &wallet1.pubkey(),
        &mint_a_keypair.pubkey(),
        &account_1_keypair.pubkey(),
        conn,
    );
    ixs.extend(create_account_1_ix);

    let offer_balance = offer*10u64.pow(9); //9 decimals
    let mint_offer_ix = spl_token::instruction::mint_to(
        &spl_token::id(), 
        &mint_a_keypair.pubkey(),
        &account_1_keypair.pubkey(),
        &wallet1.pubkey(),
        &[&wallet1.pubkey()],
        offer_balance, 
    ).unwrap();
    ixs.push(mint_offer_ix);

    // Account 2
    let account_2_keypair = &Keypair::new();
    let create_account_2_ix = create_account_ix(
        &wallet1.pubkey(),
        &mint_b_keypair.pubkey(),
        &account_2_keypair.pubkey(),
        conn,
    );
    ixs.extend(create_account_2_ix);

    let message = Message::new(ixs.as_ref(), Some(&wallet1.pubkey()));
    let transaction = Transaction::new(&[&wallet1, account_1_keypair, account_2_keypair], message, conn.get_latest_blockhash().unwrap());
    conn.send_and_confirm_transaction(&transaction).unwrap();

    println!("Offer accounts setup finished!");

    // Setup trade
    // Account 3
    let mut ixs = Vec::<Instruction>::new();
    let account_3_keypair = &Keypair::new();
    let create_account_3_ix = create_account_ix(
        &wallet2.pubkey(),
        &mint_a_keypair.pubkey(),
        &account_3_keypair.pubkey(),
        conn,
    );
    ixs.extend(create_account_3_ix);

    // Account 4
    let account_4_keypair = &Keypair::new();
    let create_account_4_ix = create_account_ix(
        &wallet2.pubkey(),
        &mint_b_keypair.pubkey(),
        &account_4_keypair.pubkey(),
        conn,
    );
    ixs.extend(create_account_4_ix);

    let trade_balance = trade*10u64.pow(9); //9 decimals
    let mint_trade_ix = spl_token::instruction::mint_to(
        &spl_token::id(), 
        &mint_b_keypair.pubkey(),
        &account_4_keypair.pubkey(),
        &wallet1.pubkey(),
        &[&wallet1.pubkey()],
        trade_balance, 
    ).unwrap();
    ixs.push(mint_trade_ix);

    let message = Message::new(ixs.as_ref(), Some(&wallet2.pubkey()));
    let transaction = Transaction::new(&[&wallet1, &wallet2, account_3_keypair, account_4_keypair], message, conn.get_latest_blockhash().unwrap());
    conn.send_and_confirm_transaction(&transaction).unwrap();

    println!("Trade accounts setup finished!");

    // report
    println!("");
    println!("Mint account A: {}", mint_a_keypair.pubkey().to_string());
    println!("Mint account B: {}", mint_b_keypair.pubkey().to_string());
    println!("Offer");
    println!("\t- src: {} with balance {}", account_1_keypair.pubkey().to_string(), offer);
    println!("\t- dst: {}", account_3_keypair.pubkey().to_string());
    println!("Trade");
    println!("\t- src: {} with balance {}", account_4_keypair.pubkey().to_string(), trade);
    println!("\t- dst: {}", account_2_keypair.pubkey().to_string());
    println!(""); 

    Ok(())
}

pub fn create_trade(
    trade: u64, 
    owner: Keypair,
    token_account: Pubkey,
    trader_program_id: Pubkey, 
    conn: &RpcClient,
) -> Result<()> {
    println!("Creating trade");

    let trade_account_keypair = &Keypair::new(); // TODO: is this ok?
    println!("New trade account: {}", trade_account_keypair.pubkey().to_string());


    let program_info = conn.get_account(&trader_program_id).unwrap();
    if !program_info.executable {
        println!(
            "program with addr {} is not executable",
            trader_program_id,
        );
        
        Err(Error::InvalidConfig(String::from_str("not a program").unwrap()))?;
    }

    let create_trader_account_ix = system_instruction::create_account(
        &owner.pubkey(),
        &trade_account_keypair.pubkey(),
        conn.get_minimum_balance_for_rent_exemption(state::AccountTrade::size()).unwrap(),
        state::AccountTrade::size() as u64,
        &trader_program_id,
    );

    // this should allow one to have as many trades as they want
    // generate it off-chain to save computation credits
    let (pda_pubkey, bump_seed) = Pubkey::find_program_address(
        &[trade_account_keypair.pubkey().as_ref()],
            &trader_program_id,
    );

    let action = Action::CreateTrade {
        bump_seed: bump_seed,
        trade: trade,
    };
    let buf = &action.try_to_vec().unwrap()[..];

    let init_trade_ix = Instruction::new_with_bytes(
        trader_program_id,
        buf,
        vec![
            AccountMeta::new_readonly(owner.pubkey(), true),
            AccountMeta::new(trade_account_keypair.pubkey(), false),
            AccountMeta::new(token_account, false),
            AccountMeta::new(pda_pubkey, false),          
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
    );
    let message = Message::new(&[create_trader_account_ix, init_trade_ix], Some(&owner.pubkey()));
    let transaction = Transaction::new(&[&owner, &trade_account_keypair], message, conn.get_latest_blockhash().unwrap());

    conn.send_and_confirm_transaction(&transaction).unwrap();

    Ok(())
}

/*
 * owner will be funding ata accounts if any needs to be created.
*/
pub fn make_trade(
    offer: u64,
    trade: u64,
    owner: Keypair,
    wallet1: Pubkey,
    trade_id: Pubkey,
    trader_program_id: Pubkey,
    trade_dst: Option<Pubkey>,
    trade_src: Pubkey, 
    offer_dst: Option<Pubkey>,
    offer_src: Pubkey,
    conn: &RpcClient,
) -> Result<()> {
    println!("Making trade...");

    let (pda_pubkey, _) = Pubkey::find_program_address(
        &[trade_id.as_ref()],
            &trader_program_id,
    );

    let action = Action::MakeTrade { 
        expected_offer: offer,
        expected_trade: trade,
    };
    let buf = &action.try_to_vec().unwrap()[..];

    let make_trade_ix = Instruction::new_with_bytes(
        trader_program_id,
        buf,
        vec![
            AccountMeta::new_readonly(owner.pubkey(), true),
            AccountMeta::new(trade_id, false),
            AccountMeta::new_readonly(pda_pubkey, false),
            AccountMeta::new(offer_src, false),
            AccountMeta::new(trade_dst.unwrap_or_else(|| get_or_create_token_account(&owner, owner.pubkey(), trade_src, conn).unwrap()), false),
            AccountMeta::new(trade_src, false),
            AccountMeta::new(offer_dst.unwrap_or_else(|| get_or_create_token_account(&owner, wallet1, offer_src, conn).unwrap()), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(wallet1, false),
        ],
    );
    let message = Message::new(&[make_trade_ix], Some(&owner.pubkey()));
    let transaction = Transaction::new(&[&owner], message, conn.get_latest_blockhash().unwrap());

    conn.send_and_confirm_transaction(&transaction).unwrap();

    Ok(())
}
