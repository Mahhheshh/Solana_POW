#![no_std]
#![allow(unexpected_cfgs)]

use pinocchio::{
    account_info::AccountInfo, entrypoint, nostd_panic_handler, program_error::ProgramError,
    pubkey::Pubkey, ProgramResult,
};

mod instructions;
use instructions::{deposite::Deposit, withdraw::Withdraw};

entrypoint!(process_instruction);
nostd_panic_handler!();

pinocchio_pubkey::declare_id!("7WSfz98HpkeZUatKPuVnkjCWaWfpxKuigyN8JS9Wj3SQ");

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    match instruction_data.split_first() {
        Some((Deposit::DISCRIMINATOR, data)) => Deposit::try_from((data, accounts))?.process(),
        Some((Withdraw::DISCRIMINATOR, _)) => Withdraw::try_from(accounts)?.process(),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
