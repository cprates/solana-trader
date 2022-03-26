use crate::{processor::Processor};
use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    pubkey::Pubkey,
    msg,
};

#[cfg(not(feature = "exclude_entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> entrypoint::ProgramResult {
    msg!(
        "process_instruction: {}: {} accounts, data={:?}",
        program_id,
        accounts.len(),
        instruction_data
    );

    if let Err(error) = Processor::process(program_id, accounts, instruction_data) {
        msg!("{}", error);

        return Err(error);
    }

    Ok(())
}