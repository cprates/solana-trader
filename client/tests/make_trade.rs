mod lib;

use {
    lib::*,
    solana_program_test::*,
    solana_sdk::{
        instruction::AccountMeta,
        pubkey::Pubkey,
        signature::Signer, 
    },
};

#[tokio::test]
async fn test_make_trade() {
    let test = TestData::init().await;

    create_test_trade(&test).await;

    let offer_amount: u64 = spl_token::ui_amount_to_amount(10.0, 9);
    let trade_amount: u64 = spl_token::ui_amount_to_amount(2.0, 9);
    let (ix, _, _) = make_trade_ix(&test, offer_amount, trade_amount, None);
    process_ix(&vec![&test.payer, &test.wallet2], test.payer.pubkey(), ix, &test.conn).await.unwrap();

    // trade fee transfered?
    let fee_account = get_spl_account(test.fee_ata, &test.conn).await;
    let trade_fee = 100000000 as u64;
    assert_eq!(fee_account.amount, trade_fee);

    let offer_src_account = get_spl_account(test.offer_src, &test.conn).await;
    // offer src account back to right owner?
    assert_eq!(offer_src_account.owner, test.payer.pubkey());
    assert_eq!(offer_src_account.amount, 0);

    let offer_dst_account = get_spl_account(test.offer_dst, &test.conn).await;
    assert_eq!(offer_dst_account.amount, offer_amount);

    let trade_src_account = get_spl_account(test.trade_src, &test.conn).await;
    // should have the remaining of the total balance
    assert_eq!(trade_src_account.amount, 3000000000);

    let trade_dst_account = get_spl_account(test.trade_dst, &test.conn).await;
    assert_eq!(trade_dst_account.amount, trade_amount - trade_fee);

    // trade account should be closed in the end
    let trade_account_ai = test.conn.borrow_mut().get_account(test.trade_account_keypair.pubkey()).await.unwrap();
    assert_eq!(trade_account_ai, None);
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: custom program error: 0x0")]
async fn test_make_trade_checks_authority_is_signer() {
    let test = TestData::init().await;

    create_test_trade(&test).await;

    let fake_authority = Pubkey::new_unique();
    let fake_pda = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(fake_authority, false),
        AccountMeta::new(test.trade_account_keypair.pubkey(), false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new(test.offer_src, false),
        AccountMeta::new(test.trade_dst, false),
        AccountMeta::new(test.trade_src, false),
        AccountMeta::new(test.offer_dst, false),
        AccountMeta::new(test.payer.pubkey(), false),
        AccountMeta::new(test.fee_ata, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let (ix, _, _) = make_trade_ix(&test, 2, 2, Some(accounts));
    let panic_on = process_ix(&vec![&test.payer], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: custom program error: 0x4")]
async fn test_make_trade_checks_trade_is_init() {
    let test = TestData::init().await;

    create_test_trade(&test).await;

    let uninitialised_trade = TestData::create_trade_account(
        test.wallet2.pubkey(),
        &test.payer,
        None,
        &test.conn,
    ).await;

    let fake_pda = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(test.wallet2.pubkey(), true),
        AccountMeta::new(uninitialised_trade.pubkey(), false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new(test.offer_src, false),
        AccountMeta::new(test.trade_dst, false),
        AccountMeta::new(test.trade_src, false),
        AccountMeta::new(test.offer_dst, false),
        AccountMeta::new(test.payer.pubkey(), false),
        AccountMeta::new(test.fee_ata, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let (ix, _, _) = make_trade_ix(&test, 2, 2, Some(accounts));
    let panic_on = process_ix(&vec![&test.payer, &test.wallet2], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: custom program error: 0x8")]
async fn test_make_trade_checks_trade_given_trade_dst_is_correct() {
    let test = TestData::init().await;

    create_test_trade(&test).await;

    let fake_trade_dst = Pubkey::new_unique();
    let fake_pda = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(test.wallet2.pubkey(), true),
        AccountMeta::new(test.trade_account_keypair.pubkey(), false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new(test.offer_src, false),
        AccountMeta::new(fake_trade_dst, false),
        AccountMeta::new(test.trade_src, false),
        AccountMeta::new(test.offer_dst, false),
        AccountMeta::new(test.payer.pubkey(), false),
        AccountMeta::new(test.fee_ata, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let (ix, _, _) = make_trade_ix(&test, 2, 2, Some(accounts));
    let panic_on = process_ix(&vec![&test.payer, &test.wallet2], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: custom program error: 0x7")]
async fn test_make_trade_checks_trade_mint() {
    let test = TestData::init().await;

    create_test_trade(&test).await;

    let recent_blockhash = test.conn.borrow_mut().get_latest_blockhash().await.unwrap();
    let fake_trade_mint = mint_account(&test.wallet2, &test.payer, recent_blockhash, &test.conn).await;
    let fake_trade_src = token_account(&test.wallet2, &test.payer, fake_trade_mint, recent_blockhash, &test.conn).await;
    let fake_pda = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(test.wallet2.pubkey(), true),
        AccountMeta::new(test.trade_account_keypair.pubkey(), false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new(test.offer_src, false),
        AccountMeta::new(test.trade_dst, false),
        AccountMeta::new(fake_trade_src, false),
        AccountMeta::new(test.offer_dst, false),
        AccountMeta::new(test.payer.pubkey(), false),
        AccountMeta::new(test.fee_ata, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let (ix, _, _) = make_trade_ix(&test, 2, 2, Some(accounts));
    let panic_on = process_ix(&vec![&test.payer, &test.wallet2], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: custom program error: 0x6")]
async fn test_make_trade_checks_offer_src_match() {
    let test = TestData::init().await;

    create_test_trade(&test).await;

    let fake_offer_src = Pubkey::new_unique();
    let fake_pda = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(test.wallet2.pubkey(), true),
        AccountMeta::new(test.trade_account_keypair.pubkey(), false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new(fake_offer_src, false),
        AccountMeta::new(test.trade_dst, false),
        AccountMeta::new(test.trade_src, false),
        AccountMeta::new(test.offer_dst, false),
        AccountMeta::new(test.payer.pubkey(), false),
        AccountMeta::new(test.fee_ata, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let (ix, _, _) = make_trade_ix(&test, 2, 2, Some(accounts));
    let panic_on = process_ix(&vec![&test.payer, &test.wallet2], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: custom program error: 0x2")]
async fn test_make_trade_checks_offer_amount() {
    let test = TestData::init().await;

    create_test_trade(&test).await;

    let (ix, _, _) = make_trade_ix(&test, 20, 2, None);
    let panic_on = process_ix(&vec![&test.payer, &test.wallet2], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: custom program error: 0x3")]
async fn test_make_trade_checks_trade_amount() {
    let test = TestData::init().await;

    create_test_trade(&test).await;

    let offer: u64 = spl_token::ui_amount_to_amount(10.0, 9);
    let (ix, _, _) = make_trade_ix(&test, offer, 20, None);
    let panic_on = process_ix(&vec![&test.payer, &test.wallet2], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}

#[tokio::test]
#[should_panic(expected = "transport transaction error: Error processing Instruction 0: custom program error: 0x0")]
async fn test_make_trade_checks_fee_account_dst() {
    let test = TestData::init().await;

    create_test_trade(&test).await;

    let fake_fee_ata = Pubkey::new_unique();
    let fake_pda = Pubkey::new_unique();
    let accounts = vec![
        AccountMeta::new_readonly(test.wallet2.pubkey(), true),
        AccountMeta::new(test.trade_account_keypair.pubkey(), false),
        AccountMeta::new_readonly(fake_pda, false),
        AccountMeta::new(test.offer_src, false),
        AccountMeta::new(test.trade_dst, false),
        AccountMeta::new(test.trade_src, false),
        AccountMeta::new(test.offer_dst, false),
        AccountMeta::new(test.payer.pubkey(), false),
        AccountMeta::new(fake_fee_ata, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];
    let offer: u64 = spl_token::ui_amount_to_amount(10.0, 9);
    let trade: u64 = spl_token::ui_amount_to_amount(2.0, 9);
    let (ix, _, _) = make_trade_ix(&test, offer, trade, Some(accounts));
    let panic_on = process_ix(&vec![&test.payer, &test.wallet2], test.payer.pubkey(), ix, &test.conn).await.unwrap_err();
    panic!("{}", panic_on.to_string());
}
