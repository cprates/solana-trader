use {
    assert_matches::*,
    borsh::{
        BorshSerialize,
        BorshDeserialize,
    },
    solana_program_test::*,
    solana_sdk::{
        message::Message,
        hash::Hash,
        instruction::{
            AccountMeta, 
            Instruction,
        },
        pubkey::Pubkey,
        transaction::Transaction,
        signature::{Keypair, Signer},
        system_instruction,
        sysvar::rent::Rent,
        program_pack::Pack,
    },
    spl_token:: {
        state::{Account as SPLAccount, Mint},
        instruction:: {
            initialize_account,
            initialize_mint,
            mint_to,
        },
    },
};
use ::trader::{
    instructions::Action,
    entrypoint as trader,
    state,
};
use trader_client::utils::get_wallet;
use std::cell::RefCell;

pub fn minimum_balance_rent_exempt(size: usize) -> u64 {
    Rent::default().minimum_balance(size)
}

pub async fn mint_account<'a>(
    authority: &Keypair,
    payer: &Keypair, 
    recent_blockhash: Hash, 
    conn: &RefCell<BanksClient>,
) -> Pubkey {
    let mint_key = Keypair::new();

    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint_key.pubkey(),
        minimum_balance_rent_exempt(Mint::LEN),
        Mint::LEN as u64,
        &spl_token::id(),
    );

    let init_ix = initialize_mint(
        &spl_token::id(), 
        &mint_key.pubkey(), 
        &authority.pubkey(), 
        None, 
        9,
    ).unwrap();

    let message = Message::new(&[create_ix, init_ix], Some(&payer.pubkey()));
    let transaction = Transaction::new(
        &[&payer, &mint_key],
        message, recent_blockhash,
    );
    assert_matches!(conn.borrow_mut().process_transaction(transaction).await, Ok(()));

    mint_key.pubkey()
}

pub async fn token_account(
    authority: &Keypair,
    payer: &Keypair, 
    mint: Pubkey,
    recent_blockhash: Hash,
    conn: &RefCell<BanksClient>,
) -> Pubkey {
    let account_key = Keypair::new();

    let create_ix = system_instruction::create_account(
        &payer.pubkey(),
        &account_key.pubkey(),
        minimum_balance_rent_exempt(SPLAccount::LEN),
        SPLAccount::LEN as u64,
        &spl_token::id(),
    );

    let init_ix = initialize_account(
        &spl_token::id(),
        &account_key.pubkey(),
        &mint,
        &authority.pubkey(),
    ).unwrap();

    let message = Message::new(&[create_ix, init_ix], Some(&payer.pubkey()));
    let transaction = Transaction::new(
        &[&payer, &account_key],
        message, recent_blockhash,
    );
    assert_matches!(conn.borrow_mut().process_transaction(transaction).await, Ok(()));

    account_key.pubkey()
}

pub async fn mint_to_account(
    authority: &Keypair,
    payer: &Keypair,
    mint: Pubkey,
    token_account: Pubkey,
    amount: u64,
    recent_blockhash: Hash,
    conn: &RefCell<BanksClient>,
) {
    let mint_ix = mint_to(
        &spl_token::id(),
        &mint,
        &token_account,
        &authority.pubkey(),
        &[&authority.pubkey(), &payer.pubkey()],
        amount
    ).unwrap();

    let message = Message::new(&[mint_ix], Some(&payer.pubkey()));
    let transaction = Transaction::new(
        &[authority, payer],
        message, recent_blockhash,
    );
    assert_matches!(conn.borrow_mut().process_transaction(transaction).await, Ok(()));
}

pub async fn get_spl_account(addr: Pubkey, conn: &RefCell<BanksClient>) -> SPLAccount {
    let offer_src_ai = conn.borrow_mut().get_account(addr).await.unwrap().unwrap();
    
    SPLAccount::unpack_from_slice(&offer_src_ai.data).unwrap()
}

pub fn init_trade_ix(
    test: &TestData,
    trade_amount: u64,
    accounts: Option<Vec<AccountMeta>>,
) -> (Instruction, Pubkey, u8) {
    let (pda_pubkey, bump_seed) = Pubkey::find_program_address(
        &[test.trade_account_keypair.pubkey().as_ref()],
            &test.program_id,
    );

    let action = Action::CreateTrade {
        bump_seed: bump_seed,
        trade: trade_amount,
    };
    let buf = &action.try_to_vec().unwrap()[..];

    let accounts_list = if let Some(list) = accounts {
        list
    } else {
        vec![
            AccountMeta::new_readonly(test.payer.pubkey(), true),
            AccountMeta::new(test.trade_account_keypair.pubkey(), false),
            AccountMeta::new(test.offer_src, false),
            AccountMeta::new_readonly(test.trade_mint, false),
            AccountMeta::new_readonly(test.trade_dst, false),
            AccountMeta::new_readonly(pda_pubkey, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ]
    };
    let ix = Instruction::new_with_bytes(test.program_id, buf, accounts_list);

    (ix, pda_pubkey, bump_seed)
}

pub async fn process_ix(
    signers: &Vec<&Keypair>,
    payer: Pubkey,
    ix: Instruction,
    conn: &RefCell<BanksClient>,
) -> std::result::Result<(), BanksClientError> {
    let transaction = Transaction::new(
        signers.into(),
        Message::new(&[ix], Some(&payer)),
        conn.borrow_mut().get_latest_blockhash().await.unwrap(),
    );

    match conn.borrow_mut().process_transaction(transaction).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err), 
    }
}

pub async fn create_test_trade(test: &TestData) {
    let trade_balance: u64 = spl_token::ui_amount_to_amount(2.0, 9);
    let (ix, _, _) = init_trade_ix(&test, trade_balance, None);
    process_ix(&vec![&test.payer], test.payer.pubkey(), ix, &test.conn).await.unwrap();
}

pub fn make_trade_ix(
    test: &TestData,
    offer_amount: u64,
    trade_amount: u64,
    accounts: Option<Vec<AccountMeta>>,
) -> (Instruction, Pubkey, u8) {
    let (pda_pubkey, bump_seed) = Pubkey::find_program_address(
        &[test.trade_account_keypair.pubkey().as_ref()],
            &test.program_id,
    );

    let action = Action::MakeTrade {
        expected_offer: offer_amount,
        expected_trade: trade_amount,
    };
    let buf = &action.try_to_vec().unwrap()[..];

    let accounts_list = if let Some(list) = accounts {
        list
    } else {
        vec![
            AccountMeta::new_readonly(test.wallet2.pubkey(), true),
            AccountMeta::new(test.trade_account_keypair.pubkey(), false),
            AccountMeta::new_readonly(pda_pubkey, false),
            AccountMeta::new(test.offer_src, false),
            AccountMeta::new(test.trade_dst, false),
            AccountMeta::new(test.trade_src, false),
            AccountMeta::new(test.offer_dst, false),
            AccountMeta::new(test.payer.pubkey(), false),
            AccountMeta::new(test.fee_ata, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ]
    };
    let ix = Instruction::new_with_bytes(test.program_id, buf, accounts_list);

    (ix, pda_pubkey, bump_seed)
}

pub struct TestData {
    pub conn: RefCell<BanksClient>,
    pub program_id: Pubkey,
    pub payer: Keypair, // wallet1 / User A
    pub wallet2: Keypair, // User B
    pub trade_account_keypair: Keypair,
    pub offer_mint: Pubkey,
    pub offer_src: Pubkey,
    pub offer_dst: Pubkey,    
    pub trade_mint: Pubkey,
    pub trade_src: Pubkey,
    pub trade_dst: Pubkey,
    pub fee_ata: Pubkey,
}

impl TestData {
    pub async fn create_account(
        authority: Pubkey,
        payer: &Keypair,
        rent: Option<u64>,
        size: u64,
        conn: &RefCell<BanksClient>,
    ) -> Keypair {
        let kp = Keypair::new();
        let r = if let Some(s) = rent {
            s
        } else {
            minimum_balance_rent_exempt(state::AccountTrade::size())
        };

        let ix = system_instruction::create_account(&payer.pubkey(), &kp.pubkey(), r, size, &authority);
        let message = Message::new(&[ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(
            &[&payer, &kp],
            message, conn.borrow_mut().get_latest_blockhash().await.unwrap(),
        );
        conn.borrow_mut().process_transaction(transaction).await.unwrap();

        kp
    }

    pub async fn create_trade_account(
        authority: Pubkey,
        payer: &Keypair,
        rent: Option<u64>,
        conn: &RefCell<BanksClient>,
    ) -> Keypair {
        TestData::create_account(authority, payer, rent, state::AccountTrade::size() as u64, conn).await
    }

    pub async fn init_with_conn(
        conn: RefCell<BanksClient>,
        program_id: Pubkey,
        payer: Keypair, 
        recent_blockhash: Hash,
    ) -> TestData {
        // for offer, the account balance is the offer amount
        let offer_amount: u64 = spl_token::ui_amount_to_amount(10.0, 9);
        let trade_balance: u64 = spl_token::ui_amount_to_amount(5.0, 9);

        // User B
        let wallet2 = get_wallet(None).unwrap();

        let offer_mint = mint_account(&payer, &payer, recent_blockhash, &conn).await;
        let trade_mint = mint_account(&wallet2, &payer, recent_blockhash, &conn).await;
        
        let offer_dst = token_account(&wallet2, &payer, offer_mint, recent_blockhash, &conn).await;
        let offer_src = token_account(&payer, &payer, offer_mint, recent_blockhash, &conn).await;
        mint_to_account(
            &payer,
            &payer,
            offer_mint,
            offer_src,
            offer_amount,
            recent_blockhash,
            &conn,
        ).await;
        
        let trade_dst = token_account(&payer, &payer, trade_mint, recent_blockhash, &conn).await;
        let trade_src = token_account(&wallet2, &payer, trade_mint, recent_blockhash, &conn).await;
        mint_to_account(
            &wallet2,
            &payer,
            trade_mint,
            trade_src,
            trade_balance,
            recent_blockhash,
            &conn,
        ).await;

        let trade_account_keypair = TestData::create_trade_account(program_id, &payer, None, &conn).await;
        let fee_ata = spl_associated_token_account::get_associated_token_address(&wallet2.pubkey(), &trade_mint);
        let fee_ata_ix = spl_associated_token_account::create_associated_token_account(
            &payer.pubkey(),
            &wallet2.pubkey(),
            &trade_mint,
        );
        process_ix(&vec![&payer], payer.pubkey(), fee_ata_ix, &conn).await.unwrap();

        TestData{
            conn,
            program_id, 
            payer,
            wallet2,
            trade_account_keypair,
            offer_mint,
            offer_dst,
            offer_src,
            trade_mint,
            trade_src,
            trade_dst,
            fee_ata,
        }
    }

    pub async fn init() -> TestData {
        let program_id = Pubkey::new_unique();
        let (conn, payer, recent_blockhash) = ProgramTest::new(
            "trader_program",
            program_id,
            processor!(trader::process_instruction),
        )
        .start()
        .await;

        TestData::init_with_conn(RefCell::new(conn), program_id, payer, recent_blockhash).await
    }
}



#[tokio::test]
async fn test_init_trade() {
    let test = TestData::init().await;

    let expected_trade_amount: u64 = spl_token::ui_amount_to_amount(2.0, 9);
    let (ix, pad_account, bump_seed) = init_trade_ix(&test, expected_trade_amount, None);
    process_ix(&vec![&test.payer], test.payer.pubkey(), ix, &test.conn).await.unwrap();

    let trade_ai = test.conn.borrow_mut().get_account(test.trade_account_keypair.pubkey()).await.unwrap().unwrap();
    let trade_account = state::AccountTrade::try_from_slice(&trade_ai.data).unwrap();

    let offer_src_ai = test.conn.borrow_mut().get_account(test.offer_src).await.unwrap().unwrap();
    let offer_src_token = SPLAccount::unpack_from_slice(&offer_src_ai.data).unwrap();

    let expected_offer_amount: u64 = spl_token::ui_amount_to_amount(10.0, 9);
    assert_eq!(offer_src_token.owner, pad_account);
    assert_eq!(trade_account.bump_seed, bump_seed);
    assert_eq!(trade_account.offer_token_account, test.offer_src);
    assert_eq!(trade_account.trade_dst_account, test.trade_dst);
    assert_eq!(trade_account.authority, test.payer.pubkey());
    assert_eq!(trade_account.offer_amount, expected_offer_amount);
    assert_eq!(trade_account.trade_amount, expected_trade_amount);
    assert_eq!(trade_account.initialized, true);
    assert_eq!(trade_account.trade_mint, test.trade_mint);
    assert_eq!(trade_account.program_id, test.program_id);
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: custom program error: 0x0")]
async fn test_init_trade_check_authority_is_signer() {
    let test = TestData::init().await;

    let fake_authority = Pubkey::new_unique();
    let fake_pda = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(fake_authority, false),
        AccountMeta::new(test.trade_account_keypair.pubkey(), false),
        AccountMeta::new(test.offer_src, false),
        AccountMeta::new_readonly(test.trade_mint, false),
        AccountMeta::new_readonly(test.trade_dst, false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let (ix, _, _) = init_trade_ix(&test, 2, Some(accounts));
    let panic_on = process_ix(&vec![&test.payer], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: instruction requires an uninitialized account")]
async fn test_init_trade_check_account_not_initialised() {
    // tests if the program rejects creating a trad on an account marked as initialised

    let test1 = TestData::init().await;
    let fake_pda = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(test1.payer.pubkey(), false),
        AccountMeta::new(test1.trade_account_keypair.pubkey(), false),
        AccountMeta::new(test1.offer_src, false),
        AccountMeta::new_readonly(test1.trade_mint, false),
        AccountMeta::new_readonly(test1.trade_dst, false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let (ix, _, _) = init_trade_ix(&test1, 2, Some(accounts));
    process_ix(&vec![&test1.payer], test1.payer.pubkey(), ix, &test1.conn).await.unwrap();

    // tries to init a new trade with the trade account of the operation above
    let recent_blockhash = test1.conn.borrow_mut().get_latest_blockhash().await.unwrap();
    let test2 = TestData::init_with_conn(test1.conn, test1.program_id, test1.payer, recent_blockhash).await;
    let accounts = vec![
        AccountMeta::new_readonly(test2.payer.pubkey(), false),
        AccountMeta::new(test1.trade_account_keypair.pubkey(), false),
        AccountMeta::new(test2.offer_src, false),
        AccountMeta::new_readonly(test2.trade_mint, false),
        AccountMeta::new_readonly(test2.trade_dst, false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let (ix, _, _) = init_trade_ix(&test2, 2, Some(accounts));
    let panic_on = process_ix(&vec![&test2.payer], test2.payer.pubkey(), ix, &test2.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}

// this scenario does not seem possible
// #[tokio::test]
// #[should_panic(expected = "-------")]
// async fn test_trade_account_is_rejected_if_not_rent_exempt() {
//     let test = TestData::init().await;

//     // the runtime won't allow to create an account that is not rent exempt so, create it as usual
//     // and now transfer enought to not being rent exempt enymore
//     let ix = system_instruction::transfer(&test.trade_account_keypair.pubkey(), &test.payer.pubkey(), 1000);
//     process_ix(&vec![&test.payer, &test.trade_account_keypair], test.payer.pubkey(), ix, &test.conn).await.unwrap();

//     // replace trade account
//     let fake_pda = Pubkey::new_unique();
//     let accounts = vec![
//         AccountMeta::new_readonly(test.payer.pubkey(), false),
//         AccountMeta::new(test.trade_account_keypair.pubkey(), false),
//         AccountMeta::new(test.offer_src, false),
//         AccountMeta::new_readonly(test.trade_mint, false),
//         AccountMeta::new_readonly(test.trade_dst, false),
//         AccountMeta::new_readonly(fake_pda, false),
//         AccountMeta::new_readonly(spl_token::id(), false),
//     ];
//     let (ix, _, _) = init_trade_ix(&test, 2, Some(accounts));
//     let panic_on = process_ix(&vec![&test.payer], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
//     panic!("{}", panic_on.to_string());
// }

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: insufficient funds for instruction")]
async fn test_offer_account_must_have_a_balance_equal_to_the_trade_offer() {
    let test = TestData::init().await;

    let hash = test.conn.borrow_mut().get_latest_blockhash().await.unwrap();
    // replace the offer src account with a new one with balance zero
    let offer_src = token_account(
        &test.wallet2,
        &test.payer,
        test.offer_mint,
        hash,
        &test.conn,
    ).await;

    // replace trade account
    let fake_pda = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(test.payer.pubkey(), false),
        AccountMeta::new(test.trade_account_keypair.pubkey(), false),
        AccountMeta::new(offer_src, false),
        AccountMeta::new_readonly(test.trade_mint, false),
        AccountMeta::new_readonly(test.trade_dst, false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let (ix, _, _) = init_trade_ix(&test, 2, Some(accounts));
    let panic_on = process_ix(&vec![&test.payer], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}
