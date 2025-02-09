use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("exceeded max stake")]
    ExceededMaxStake,
    #[msg("unauthorized error")]
    UnauthError,
    #[msg("freeze period have not ended yet!")]
    FreezePeriodNotPassed,
}
