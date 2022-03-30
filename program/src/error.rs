use solana_program::program_error::ProgramError;
use thiserror::Error;

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum TradeError {
    #[error("Authority missmatch")]
    WrongAuthority,
    
    #[error("Not a program")]
    NotAProgram,

    #[error("Unexpected offer amount")]
    UnexpectedOfferAmount,

    #[error("Unexpected trade amount")]
    UnexpectedTradeAmount,

    #[error("Trade not initialised")]
    TradeNotInitialised,

    #[error("Value overflow")]
    ValueOverflow,

    #[error("Wrong token account")]
    WrongTokenAccount,

    #[error("Trade mint missmatch")]
    TradeMintMissmatch,

    #[error("Unexpected account")]
    UnexpectedAccount,
}

impl From<TradeError> for ProgramError {
    fn from(e: TradeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
