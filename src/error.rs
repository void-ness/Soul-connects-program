use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReviewError {
    #[error("Account not initialized")]
    UninitializedAcc,

    #[error("Input data limit exceeded")]
    DataLimit,

    #[error("Provided PDA doesn't match")]
    InvalidPDA,
}

impl From<ReviewError> for ProgramError {
    fn from(e: ReviewError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
