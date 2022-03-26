use borsh::BorshSerialize;
use solana_program::system_program;
use trader_client::{Error, Result};
use trader_client::utils::{get_wallet, load_config};
use solana_client::rpc_client::RpcClient;
use spl_associated_token_account;
use solana_sdk::{
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

pub fn make_trade(
    offer: u64,
    trade: u64,
    owner: Keypair,
    trade_id: Pubkey,
    trader_program_id: Pubkey,
    trade_dst: Pubkey,
    trade_src: Pubkey, 
    offer_dst: Pubkey,
    original_pda: Pubkey,
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
            AccountMeta::new(pda_pubkey, false),
            AccountMeta::new(trade_dst, false),
            AccountMeta::new(trade_src, false),
            AccountMeta::new(offer_dst, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(trader_program_id, false),
            AccountMeta::new(original_pda, false),
        ],
    );
    let message = Message::new(&[make_trade_ix], Some(&owner.pubkey()));
    let transaction = Transaction::new(&[&owner], message, conn.get_latest_blockhash().unwrap());

    conn.send_and_confirm_transaction(&transaction).unwrap();

    Ok(())
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!(
            "usage: {} <path to solana hello world example program keypair>",
            args[0]
        );
        std::process::exit(-1);
    }
    // program pubkey
    let program_addr = &args[1];
    let program_pubkey = Pubkey::from_str(program_addr).unwrap();

    let cfg = load_config().unwrap();
    let cluster_url = cfg["json_rpc_url"].as_str().unwrap();
    
    let conn = RpcClient::new_with_commitment(
        cluster_url,
        CommitmentConfig::confirmed(),
    );
    println!(
        "Connected to cluster at {}.", cluster_url
    );

    let program_info = conn.get_account(&program_pubkey).unwrap();
    if !program_info.executable {
        println!(
            "program with addr {} is not executable",
            program_addr,
        );
        return;
    }

    let wallet = get_wallet().unwrap();
    //let balance = conn.get_account(&wallet.pubkey()).unwrap().lamports;
    //println!("Account balance: {}", balance); // TODO: this prints a different value from 'solana balance' ?

    // TODO
    let token_pubkey = Pubkey::from_str("J9w7iwsKUdhnPDutBjdYEphbKRpaLUmtpfAGggadupcE").unwrap();

    let trade_id = Pubkey::from_str("H96F9EAuds2Wab2qpzZXrUG16qy1rwQr9ASncoQgoQ1u").unwrap();
    let offer_dst = Pubkey::from_str("5AfQJS5FJkk1SQf5s7c5XFBDeHNbQAzjEXMWwDTbixwa").unwrap();
    let trade_dst = Pubkey::from_str("DscwD8rrC3Ztr4hQqSdtbAhCAubrjADVE4sRYg3U31hi").unwrap();
    let trade_src = Pubkey::from_str("BsP4W1iW1JHA3nAYMTNAWC26knq7ipWgAxTmQSnqZpaf").unwrap();

    //println!("fees = {:?}", rpc.get_fees()?);
    //println!("signature fee = {}", rpc.get_fees()?.fee_calculator.lamports_per_signature);

    match args[2].as_str() {
        "1" => create_trade(2, wallet, token_pubkey, program_pubkey, &conn).unwrap(),
        "2" => make_trade(
            10000000000, // TODO: stop using the account.balance or handle decimals...
            2,
            wallet,
            trade_id,
            program_pubkey,
            trade_dst,
            trade_src,
            offer_dst,
            token_pubkey,
            &conn,
        ).unwrap(),
        op => {
            eprintln!("Unknown operation '{}'", op);
            std::process::exit(-1);
        }
    }
}
