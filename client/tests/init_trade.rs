mod lib;

use {
    borsh::BorshDeserialize,
    lib::*,
    solana_program_test::*,
    solana_sdk::{
        instruction::AccountMeta,
        pubkey::Pubkey,
        signature::Signer,
        program_pack::Pack,
    },
    spl_token::state::Account as SPLAccount,
};
use ::trader::state;

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
