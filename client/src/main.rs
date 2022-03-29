use clap::{
    Arg,
    Command,
};
use trader_client::client;
use trader_client::utils::{
    get_wallet,
    load_config,
    resolve_mint_info,
    ProgramConfig,
};

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
};
use solana_clap_utils::{
    input_validators::{
        is_valid_pubkey,
        is_amount,
    },
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
    const PROGRAM_CONFIG_PATH: &str = "./program_wallet.json";
    
    let app_matches = Command::new("Trader")
        .about("Create and manage trades")
        .version("v0.0.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("config").about("Manage CLI configs")
            .arg(
                Arg::new("program")
                    .value_name("PROGRAM")
                    .takes_value(true)
                    .short('p')
                    .help("Specify program address."),
            )
            .arg(
                Arg::new("wallet")
                    .value_name("WALLET")
                    .takes_value(true)
                    .short('w')
                    .help("Specify wallet address."),
            )
            .arg_required_else_help(true)
        )
        .subcommand(Command::new("create").about("Create a new trade")
            .arg(
                Arg::new("offer_account")
                    .value_name("OFFER_ACCOUNT")
                    .takes_value(true)
                    .required(true)
                    .index(1)
                    .help("Specify the token account address of the offer. \
                        The offer amount is the account balance."),
            )
            .arg(
                Arg::new("trade_token")
                    .value_name("TRADE_TOKEN")
                    .takes_value(true)
                    .required(true)
                    .index(2)
                    .help("Specify the token address of the token wanted."),
            )
            .arg(
                Arg::new("amount")
                    .value_name("AMOUNT")
                    .takes_value(true)
                    .required(true)
                    //.validator(is_amount)
                    .index(3)
                    .help("Specify the amount of the trade."),
            )
        )
        .subcommand(Command::new("trade").about("Accept a trade")
            .arg(
                Arg::new("id")
                    .value_name("TRADE_ID")
                    .takes_value(true)
                    .required(true)
                    .index(1)
                    .help("Specify the trade id."),
            )
            .arg(
                Arg::new("offersrc")
                    .value_name("OFFER_SRC")
                    .takes_value(true)
                    .required(true)
                    .index(2)
                    .help("Specify token account from where the offer amount will be taken from."),
            )
            .arg(
                Arg::new("offer-amount")
                    .value_name("OFFER_AMOUNT")
                    .takes_value(true)
                    .required(true)
                    //.validator(is_amount)
                    .index(3)
                    .help("Specify the amount of the expected offer."),
            )
            .arg(
                Arg::new("tradesrc")
                    .value_name("TRADE_SRC")
                    .takes_value(true)
                    .required(true)
                    .index(4)
                    .help("Specify token account from where the trade amount will be taken from."),
            )
            .arg(
                Arg::new("trade-amount")
                    .value_name("TRADE_AMOUNT")
                    .takes_value(true)
                    .required(true)
                    //.validator(is_amount)
                    .index(5)
                    .help("Specify the amount of the expected trade."),
            )
            .arg(
                Arg::new("offer-owner")
                    .value_name("OFFER_OWNER")
                    .takes_value(true)
                    .required(true)
                    //.validator(is_amount)
                    .index(6)
                    .help("Specify the wallet public address of the owner of this trade."),
            )
            .arg(
                Arg::new("offerdst")
                    .value_name("OFFER_DST")
                    .takes_value(true)
                    .index(7)
                    .help("Specify token account to where the offer amount will be sent to."),
            )
            .arg(
                Arg::new("tradedst")
                    .value_name("TRADE_DST")
                    .takes_value(true)
                    .index(8)
                    .help("Specify token account to where the trade amount will be sent to."),
            )
        )
        .subcommand(Command::new("bootstrap").about("Create all accounts needed to test the program")
            .arg(
                Arg::new("wallet1")
                    .value_name("WALLET1")
                    .takes_value(true)
                    .required(true)
                    .index(1)
                    .help("Specify the path to the wallet of the user making the offer."),
            )
            .arg(
                Arg::new("wallet2")
                    .value_name("WALLET2")
                    .takes_value(true)
                    .required(true)
                    .index(2)
                    .help("Specify the path to the wallet of the user taking the offer."),
            )
        )
        .get_matches();
    
    let (sub_command, sub_matches) = app_matches.subcommand().unwrap();

    let cfg = load_config().unwrap();
    let cluster_url = cfg["json_rpc_url"].as_str().unwrap();
    
    let conn = RpcClient::new_with_commitment(
        cluster_url,
        CommitmentConfig::confirmed(),
    );
    println!(
        "Connected to cluster at {}.", cluster_url
    );

    let wallet = get_wallet(None).unwrap();
    //let balance = conn.get_account(&wallet.pubkey()).unwrap().lamports;
    //println!("Account balance: {}", balance); // TODO: this prints a different value from 'solana balance' ?
    
    //println!("fees = {:?}", rpc.get_fees()?);
    //println!("signature fee = {}", rpc.get_fees()?.fee_calculator.lamports_per_signature);

    match sub_command {
        "create" => {
            let program_addr = ProgramConfig::load_program_addr(PROGRAM_CONFIG_PATH.into()).unwrap();
            let program_pubkey = Pubkey::from_str(&program_addr).unwrap();

            let src = Pubkey::from_str(sub_matches.value_of("offer_account").unwrap().into()).unwrap();
            let trade_mint = Pubkey::from_str(sub_matches.value_of("trade_token").unwrap().into()).unwrap();
            let amount_arg: f64 = sub_matches.value_of("amount").unwrap().parse().unwrap();

            let decimals = resolve_mint_info(&src, None, &conn).unwrap();
            let ammount = spl_token::ui_amount_to_amount(amount_arg, decimals);
            client::create_trade(ammount, wallet, src, trade_mint, program_pubkey, &conn).unwrap();
        }
        "trade" => {
            let program_addr = ProgramConfig::load_program_addr(PROGRAM_CONFIG_PATH.into()).unwrap();
            let program_pubkey = Pubkey::from_str(&program_addr).unwrap();

            let trade_account_id = Pubkey::from_str(sub_matches.value_of("id").unwrap().into()).unwrap();
            let offer_src = Pubkey::from_str(sub_matches.value_of("offersrc").unwrap().into()).unwrap();
            let trade_src = Pubkey::from_str(sub_matches.value_of("tradesrc").unwrap().into()).unwrap();
            let wallet1 = Pubkey::from_str(sub_matches.value_of("offer-owner").unwrap().into()).unwrap();
            let offer_dst = match sub_matches.value_of("offerdst") {
                Some(addr) => Some(Pubkey::from_str(addr.into()).unwrap()),
                None => None
            };
            let trade_dst = match sub_matches.value_of("tradedst") {
                Some(addr) => Some(Pubkey::from_str(addr.into()).unwrap()),
                None => None
            };

            let offer_decimals = resolve_mint_info(&offer_src, None, &conn).unwrap();
            let amount: f64 = sub_matches.value_of("offer-amount").unwrap().parse().unwrap();
            let offer_ammount = spl_token::ui_amount_to_amount(amount, offer_decimals);
            let trade_decimals = resolve_mint_info(&trade_src, None, &conn).unwrap();
            let amount: f64 = sub_matches.value_of("trade-amount").unwrap().parse().unwrap();
            let trade_ammount = spl_token::ui_amount_to_amount(amount, trade_decimals);

            client::make_trade(
                offer_ammount,
                trade_ammount,
                wallet,
                wallet1,
                trade_account_id,
                program_pubkey,
                trade_dst,
                trade_src,
                offer_dst,
                offer_src,
                &conn,
            ).unwrap();
        }
        "bootstrap" => {
            let wallet1 = get_wallet(sub_matches.value_of("wallet1")).unwrap();
            let wallet2 = get_wallet(sub_matches.value_of("wallet2")).unwrap();
            client::setup_accounts(10, 12, wallet1, wallet2, &conn).unwrap();
        }
        "config" => {
            match sub_matches.value_of("program") {
                Some(addr) => {
                    ProgramConfig::store_program_addr(PROGRAM_CONFIG_PATH.into(), addr.into()).unwrap();
                    ()
                },
                None => ()
            }

            match sub_matches.value_of("wallet") {
                Some(addr) => {
                    // ProgramConfig::store_wallet_addr(program_config_path.into(), addr.into()).unwrap();
                    ()
                },
                None => ()
            }
        }
        op => {
            eprintln!("Unknown operation '{}'", op);
            std::process::exit(-1);
        }
    }
}
