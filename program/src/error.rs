use solana_program::program_error::ProgramError;
use thiserror::Error;

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum TradeError {
    #[error("Authority missmatch")]
    WrongAuthority,
    #[error("Not a program")]
    NotAProgram,
}

impl From<TradeError> for ProgramError {
    fn from(e: TradeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
