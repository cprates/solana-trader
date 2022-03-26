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
    system_instruction,
    program_pack::{IsInitialized, Pack, Sealed},
};
use spl_token::state::Account;

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

                trade_account.bump_seed = bump_seed;
                trade_account.offer_token_account = *offer_token_ai.key;
                trade_account.authority = *authority.key;
                trade_account.offer_amount = offer_token.amount;
                trade_account.trade_amount = trade;
                trade_account.initialized = true;
                trade_account.program_id = *program_id;
                trade_account.serialize(&mut *trade_ai.data.borrow_mut())?;

                // Create temp account

                msg!("Trade account initialised...");

                // transfer authority of the token account to trader program - this will avoid
                // part A having to somehow sign the transfer when making the deal

                let pda_pubkey = next_account_info(accounts_iter)?;
                let token_prog_ai = next_account_info(accounts_iter)?;

                msg!("Transfering token account authority to {}", pda_pubkey.key.to_string());

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
                let mut trade_account = state::AccountTrade::try_from_slice(&trade_account_ai.data.borrow())?;
                if !trade_account.initialized {
                    return Err(TradeError::TradeNotInitialised)?;
                }

                let pda_ai = next_account_info(accounts_iter)?;
                let trade_dst_ai = next_account_info(accounts_iter)?;
                let trade_src_ai = next_account_info(accounts_iter)?;
                let offer_dst_ai = next_account_info(accounts_iter)?;
                let token_program_ai = next_account_info(accounts_iter)?;
                let trader_program_ai = next_account_info(accounts_iter)?;

                // TODO
                let original_pda_ai = next_account_info(accounts_iter)?;

                if expected_offer != trade_account.offer_amount {
                    msg!("Expected offer of {}, but got {}", expected_offer, trade_account.offer_amount);
                    return Err(TradeError::UnexpectedOfferAmount)?;
                }

                if expected_trade != trade_account.trade_amount {
                    msg!("Expected trade of {}, but got {}", expected_trade, trade_account.trade_amount);
                    return Err(TradeError::UnexpectedTradeAmount)?;
                }

                // transfer offer from pda to destination 

                let transfer_pda_ix = spl_token::instruction::transfer(
                    token_program_ai.key,
                    original_pda_ai.key,
                    offer_dst_ai.key,
                    &pda_ai.key,
                    &[&pda_ai.key],
                    expected_offer,
                )?;
                
                invoke_signed(
                    &transfer_pda_ix,
                    &[
                        original_pda_ai.clone(),
                        offer_dst_ai.clone(),
                        pda_ai.clone(),
                        token_program_ai.clone(),
                    ],
                    &[&[trade_account_ai.key.as_ref(), &[trade_account.bump_seed]]],
                )?;
                msg!("Offer amount transfered...");

                // TODO:
                //   - transfer trade amount
                //   - transfer pda's and trade lamports back to owner
                //   - close pda and trade account
            }
        }

        Ok(())
    }
}