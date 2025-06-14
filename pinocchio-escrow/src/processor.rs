use pinocchio::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, ProgramResult,
};

use crate::instructions::{AcceptOfferInstruction, MakeOfferInstruction, RefundInstruction};

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // check for valid program id
    if program_id != &crate::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    // split instruction at first byte and do matching
    let _ = match instruction_data.split_first() {
        Some((MakeOfferInstruction::DISCRIMINATOR, data)) => {
            MakeOfferInstruction::try_from((data, accounts))?.process()
        }
        Some((AcceptOfferInstruction::DISCRIMINATOR, _)) => {
            AcceptOfferInstruction::try_from(accounts)?.process()
        }
        Some((RefundInstruction::DISCRIMINATOR, _)) => {
            RefundInstruction::try_from(accounts)?.process()
        }
        _ => Err(ProgramError::InvalidInstructionData),
    };

    Ok(())
}
