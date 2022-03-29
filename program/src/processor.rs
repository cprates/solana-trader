use crate::{instructions::Action, error::TradeError};
use crate::state;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::sysvar::Sysvar;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    program::{invoke, invoke_signed},
    msg,
    pubkey::{Pubkey, PUBKEY_BYTES},
    program_error::ProgramError,
    program_memory::sol_memcmp,
    rent::Rent,
    program_pack::Pack,
};
use spl_token::state::Account;
use std::ops::{Mul, Sub};
use std::str::FromStr;

// Public address of the wallet that has the authority on the program
const PROGRAM_AUTHORITY: &str = "8VtktqchqCSowPhdiZfuMez8HqrRf2LRPcdwjvGNiumX";
const FEE_PERCENTAGE: f64 = 0.01; // 1%


pub struct Processor {}

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> entrypoint::ProgramResult {
        let instruction = Action::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        let accounts_iter = &mut accounts.iter();

        match instruction {
            Action::CreateTrade { trade, bump_seed } => {
                msg!("Creating trade...");
                
                let authority = next_account_info(accounts_iter)?;
                if !authority.is_signer {
                    Err(TradeError::WrongAuthority)?;
                }

                let trade_ai = next_account_info(accounts_iter)?;
                let mut trade_account = state::AccountTrade::try_from_slice(&trade_ai.data.borrow())?;
                if trade_account.initialized {
                    return Err(ProgramError::AccountAlreadyInitialized)?;
                }

                let rent = Rent::get()?;
                if !rent.is_exempt(trade_ai.lamports(), trade_ai.data_len()) {
                    return Err(ProgramError::AccountNotRentExempt)?;
                }
                
                let offer_token_ai = next_account_info(accounts_iter)?;
                // TODO: is this check useful?
                if *offer_token_ai.owner != spl_token::id() {
                    return Err(ProgramError::IncorrectProgramId)?;
                }
                
                let offer_token = Account::unpack_from_slice(&offer_token_ai.data.borrow())?;
                if offer_token.amount == 0 {
                    return Err(ProgramError::InsufficientFunds)?;
                }

                let trade_mint_ai = next_account_info(accounts_iter)?;

                trade_account.bump_seed = bump_seed;
                trade_account.offer_token_account = *offer_token_ai.key;
                trade_account.authority = *authority.key;
                trade_account.offer_amount = offer_token.amount;
                trade_account.trade_amount = trade;
                trade_account.initialized = true;
                trade_account.trade_mint = *trade_mint_ai.key;
                trade_account.program_id = *program_id;
                trade_account.serialize(&mut *trade_ai.data.borrow_mut())?;

                msg!("Trade account initialised...");

                // transfer authority of the token account to trader program - this will avoid
                // part A having to somehow sign the transfer when making the deal

                let pda_pubkey = next_account_info(accounts_iter)?;
                let token_prog_ai = next_account_info(accounts_iter)?;

                let owner_change_ix = spl_token::instruction::set_authority(
                    &spl_token::id(),
                    offer_token_ai.key,
                    Some(&pda_pubkey.key),
                    spl_token::instruction::AuthorityType::AccountOwner,
                    authority.key,
                    &[&authority.key],
                )?;
        
                invoke(
                    &owner_change_ix,
                    &[
                        offer_token_ai.clone(),
                        authority.clone(),
                        token_prog_ai.clone(),
                    ],
                )?;

                msg!("Transfered authority..");
            },

            Action::MakeTrade{ expected_offer, expected_trade} => {
                msg!("Making trade...");
                
                let authority_ai = next_account_info(accounts_iter)?;
                if !authority_ai.is_signer {
                    Err(TradeError::WrongAuthority)?;
                }

                let trade_account_ai = next_account_info(accounts_iter)?;
                let trade_account = state::AccountTrade::try_from_slice(&trade_account_ai.data.borrow())?;
                if !trade_account.initialized {
                    return Err(TradeError::TradeNotInitialised)?;
                }

                let pda_ai = next_account_info(accounts_iter)?;
                // to avoid the need of this account, I could create a temporary PDA that was deleted afterwards
                // instead of using the original token account.
                // Another detail about this implementation is that this account should not be an ATA because at some
                // point its authority is moved to the program which changes its data
                let original_pda_addr_ai = next_account_info(accounts_iter)?;
                let trade_dst_ai = next_account_info(accounts_iter)?;
                let trade_src_ai = next_account_info(accounts_iter)?;
                let trade_src = Account::unpack_from_slice(&trade_src_ai.data.borrow())?;
                if sol_memcmp(trade_src.mint.as_ref(), trade_account.trade_mint.as_ref(), PUBKEY_BYTES) != 0 {
                    return Err(TradeError::TradeMintMissmatch)?;
                }
                let offer_dst_ai = next_account_info(accounts_iter)?;
                let offer_owner_ai = next_account_info(accounts_iter)?;
                let fee_account_ai = next_account_info(accounts_iter)?;
                let token_program_ai = next_account_info(accounts_iter)?;

                if sol_memcmp(trade_account.offer_token_account.as_ref(), original_pda_addr_ai.key.as_ref(), PUBKEY_BYTES) != 0 {
                    Err(TradeError::WrongTokenAccount)?
                }

                if sol_memcmp(program_id.as_ref(), trade_account.program_id.as_ref(), PUBKEY_BYTES) != 0 {
                    Err(ProgramError::IncorrectProgramId)?
                }

                // I'll leave the checks agains the account balance to the spl-token program
                if expected_offer != trade_account.offer_amount {
                    msg!("Expected offer of {}, but got {}", expected_offer, trade_account.offer_amount);
                    return Err(TradeError::UnexpectedOfferAmount)?;
                }

                if expected_trade != trade_account.trade_amount {
                    msg!("Expected trade of {}, but got {}", expected_trade, trade_account.trade_amount);
                    return Err(TradeError::UnexpectedTradeAmount)?;
                }

                // transfer fee

                let prog_authority = Pubkey::from_str(PROGRAM_AUTHORITY).unwrap();
                let fee_ata = spl_associated_token_account::get_associated_token_address(&prog_authority, &trade_src.mint);
                // make sure the fee account passed is the correct one
                if sol_memcmp(fee_account_ai.key.as_ref(), fee_ata.as_ref(), PUBKEY_BYTES) != 0 {
                    Err(TradeError::WrongAuthority)?
                }
                
                // Not sure on how to do the rouding
                // using floor or ceil instead of rount result in missing symbols in the ELF file...
                let lamports_fee = (trade_account.offer_amount as f64).mul(FEE_PERCENTAGE).round() as u64;
                msg!("Applying a transfer fee of {} lamports", lamports_fee);

                let fee_transfer_ix = spl_token::instruction::transfer(
                    &spl_token::id(),
                    trade_src_ai.key,
                    &fee_account_ai.key,
                    &authority_ai.key,
                    &[&authority_ai.key],
                    lamports_fee,
                )?;
                
                invoke(
                    &fee_transfer_ix,
                    &[
                        trade_src_ai.clone(),
                        fee_account_ai.clone(),
                        authority_ai.clone(),
                        token_program_ai.clone(),
                    ],
                )?;

                msg!("Fee transfered to {}...", fee_account_ai.key.to_string());

                // transfer offer from pda to destination 

                let transfer_offer_ix = spl_token::instruction::transfer(
                    &spl_token::id(),
                    original_pda_addr_ai.key,
                    offer_dst_ai.key,
                    &pda_ai.key,
                    &[&pda_ai.key],
                    expected_offer,
                )?;
                
                invoke_signed(
                    &transfer_offer_ix,
                    &[
                        original_pda_addr_ai.clone(),
                        offer_dst_ai.clone(),
                        pda_ai.clone(),
                        token_program_ai.clone(),
                    ],
                    &[&[trade_account_ai.key.as_ref(), &[trade_account.bump_seed]]],
                )?;

                msg!(
                    "Offer amount transfered from {} with PDA {} to {}...",
                    original_pda_addr_ai.key.to_string(), pda_ai.key.to_string(), offer_dst_ai.key.to_string(),
                );

                // transfer trade amount

                let transfer_trade_ix = spl_token::instruction::transfer(
                    &spl_token::id(),
                    trade_src_ai.key,
                    trade_dst_ai.key,
                    &authority_ai.key,
                    &[&authority_ai.key],
                    expected_trade.sub(lamports_fee),
                )?;
                
                invoke(
                    &transfer_trade_ix,
                    &[
                        trade_src_ai.clone(),
                        trade_dst_ai.clone(),
                        authority_ai.clone(),
                        token_program_ai.clone(),
                    ],
                )?;

                msg!(
                    "Trade amount transfered from {} to {}...",
                    trade_src_ai.key.to_string(), trade_dst_ai.key.to_string(),
                );

                let trade_account_balance = trade_account_ai.lamports();
                // TODO: Store this in the trade account and make sure they match, to prevent the taker from using a different account to return lamports
                **offer_owner_ai.try_borrow_mut_lamports()? = offer_owner_ai
                    .lamports()
                    .checked_add(trade_account_ai.lamports())
                    .ok_or(TradeError::ValueOverflow)?;
                // close account
                **trade_account_ai.try_borrow_mut_lamports()? = 0;
                let bump_seed = trade_account.bump_seed;
                let offer_authority = trade_account.authority;
                // clean data for security reasons
                *trade_account_ai.try_borrow_mut_data()? = &mut [];

                msg!("Trade account closed. Returned {} lamports to account {}", trade_account_balance, offer_owner_ai.key.to_string());

                // return authotiry of the offer token account to the original owner

                // makes sure it's returning authority to the right owner
                if sol_memcmp(offer_authority.as_ref(), offer_owner_ai.key.as_ref(), PUBKEY_BYTES) != 0 {
                    Err(TradeError::WrongAuthority)?
                }

                let owner_change_ix = spl_token::instruction::set_authority(
                    &spl_token::id(),
                    original_pda_addr_ai.key,
                    Some(&offer_authority),
                    spl_token::instruction::AuthorityType::AccountOwner,
                    &pda_ai.key,
                    &[&pda_ai.key],
                )?;
        
                invoke_signed(
                    &owner_change_ix,
                    &[
                        original_pda_addr_ai.clone(),
                        pda_ai.clone(),
                    ],
                    &[&[trade_account_ai.key.as_ref(), &[bump_seed]]],
                )?;

                msg!("Returned authority of {} back to {}", 
                    original_pda_addr_ai.key.to_string(), offer_authority.to_string());
            }
        }

        Ok(())
    }
}