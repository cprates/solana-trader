use trader_client::client;
use trader_client::utils::{
    get_wallet,
    load_config,
    resolve_mint_info,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
};

use std::str::FromStr;

/*
solana-keygen new --outfile wallet1.json
solana config set --keypair $(pwd)/wallet1.json
solana airdrop 50000
solana-keygen new --outfile wallet2.json
solana config set --keypair $(pwd)/wallet2.json
solana airdrop 50000
 */
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        eprintln!(
            "usage: {} <trader prog addr> <action>",
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

    let wallet = get_wallet(None).unwrap();
    //let balance = conn.get_account(&wallet.pubkey()).unwrap().lamports;
    //println!("Account balance: {}", balance); // TODO: this prints a different value from 'solana balance' ?

    // TODO
    let offer_src = Pubkey::from_str("8GKnb7qGi3iRq59du5VTTQFSwTEhtrKgVywnZ1LcrGZS").unwrap();
    let offer_dst = Pubkey::from_str("3vcMr9AUhK9KcV12CtnTjV7SqJr2nne3nVci2hJ2AqYd").unwrap();
    let trade_src = Pubkey::from_str("9j4dvQFYQE8kAzdM3Ukxfk7obus7vviNVuV7kPWk9bGP").unwrap();
    let trade_dst = Pubkey::from_str("7p3tqYMydkUNzrqT5NFojCxTGfkMLCFr3nAisuSUYYNw").unwrap();

    // TODO: pass in the args
    let wallet1 = Pubkey::from_str("C1G2n2mFb27S3didy9zRc5KCHvgXNVtBmH4DzFfQEaCb").unwrap();
    // generated after step "1"
    let trade_account_id = Pubkey::from_str("ErsEQXQqGgawbCecvt2jXdmZNn63SvLJ7yrk5QtK9JrV").unwrap();
    
    //println!("fees = {:?}", rpc.get_fees()?);
    //println!("signature fee = {}", rpc.get_fees()?.fee_calculator.lamports_per_signature);

    match args[2].as_str() {
        "1" => {
            let decimals = resolve_mint_info(&offer_src, None, &conn).unwrap();
            let ammount = spl_token::ui_amount_to_amount(2.0, decimals);
            client::create_trade(ammount, wallet, offer_src, program_pubkey, &conn).unwrap();
        }
        "2" => {
            let offer_decimals = resolve_mint_info(&offer_src, None, &conn).unwrap();
            let offer_ammount = spl_token::ui_amount_to_amount(10.0, offer_decimals);
            let trade_decimals = resolve_mint_info(&trade_src, None, &conn).unwrap();
            let trade_ammount = spl_token::ui_amount_to_amount(2.0, trade_decimals);

            client::make_trade(
                offer_ammount,
                trade_ammount,
                wallet,
                wallet1,
                trade_account_id,
                program_pubkey,
                None, //Some(trade_dst),
                trade_src,
                None, //Some(offer_dst),
                offer_src,
                &conn,
            ).unwrap();
        }
        "3" => {
            if args.len() != 5 {
                eprintln!(
                    "usage: {} <trader prog addr> 3 <path to wallet 1> <path to wallet 2>",
                    args[0]
                );
                std::process::exit(-1);
            }
            let wallet1 = get_wallet(Some(&args[3])).unwrap();
            let wallet2 = get_wallet(Some(&args[4])).unwrap();
            client::setup_accounts(10, 12, wallet1, wallet2, &conn).unwrap();
        }
        op => {
            eprintln!("Unknown operation '{}'", op);
            std::process::exit(-1);
        }
    }
}
