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
};

pub struct Processor {}

impl Processor {
    pub fn process(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> entrypoint::ProgramResult {
        let instruction = Action::try_from_slice(instruction_data)
            .map_err(|_| ProgramError::InvalidInstructionData)?;
        let accounts_iter = &mut accounts.iter();

        match instruction {
            Action::CreateTrade { offer, trade } => {
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

                let mint_ai = next_account_info(accounts_iter)?;
                // TODO: is this check useful?
                if *mint_ai.owner != spl_token::id() {
                    return Err(ProgramError::IncorrectProgramId)?;
                }
                
                let offer_token_ai = next_account_info(accounts_iter)?;
                // TODO: is this check useful?
                if *offer_token_ai.owner != spl_token::id() {
                    return Err(ProgramError::IncorrectProgramId)?;
                }

                let temp_account_ai = next_account_info(accounts_iter)?;

                let trade_program_id = next_account_info(accounts_iter)?;
                // TODO: is this check really needed?
                if !trade_program_id.executable {
                    return Err(TradeError::NotAProgram)?;
                }

                let system_program_id = next_account_info(accounts_iter)?;
                // TODO: is this check really needed?
                if !system_program_id.executable {
                    return Err(TradeError::NotAProgram)?;
                }

                // Create temp account

                let temp_size = state::AccountTemp::size();
                let min_balance = rent.minimum_balance(temp_size);
                
                // TODO: what is the difference between using this one or create_account_with_seed ?
                let owner_change_ix = system_instruction::create_account(
                    authority.key,
                    temp_account_ai.key,
                    min_balance,
                    temp_size as u64,
                    trade_program_id.key,
                );

                invoke_signed(
                    &owner_change_ix,
                    &[
                        authority.clone(),
                        temp_account_ai.clone(),
                        system_program_id.clone(),
                    ],
                    &[&[&authority.key.as_ref(), &trade_ai.key.as_ref(), &[255]]],
                )?;

                trade_account.offer_token_account = *offer_token_ai.key;
                trade_account.authority = *authority.key;
                trade_account.offer_amount = offer;
                trade_account.trade_amount = trade;
                trade_account.initialized = true;
                trade_account.mint_account = *mint_ai.key;
                trade_account.program_id = *trade_program_id.key;
                trade_account.serialize(&mut *trade_ai.data.borrow_mut())?;
            },
        }

        Ok(())
    }
}