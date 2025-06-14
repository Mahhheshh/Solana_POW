use crate::state::escrow::Escrow;

use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::find_program_address,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::{InitializeAccount3, Transfer};

/// Accounts required for the Make instruction
///
/// # Account List
/// - `maker` - The signer creating the escrow (must sign)
/// - `mint_x` - The mint of token X to lock
/// - `mint_y` - The mint of token Y to receive
/// - `maker_ata_x` - Maker's associated token account for token X
/// - `escrow` - The escrow account to store the trade details
/// - `vault_x` - The vault account to hold token X
pub struct MakeOfferAccounts<'a> {
    pub maker: &'a AccountInfo,
    pub mint_x: &'a AccountInfo,
    pub mint_y: &'a AccountInfo,
    pub maker_ata_x: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub vault_x: &'a AccountInfo,
    pub escrow_bump: u8,
}

impl<'a> TryFrom<&'a [AccountInfo]> for MakeOfferAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [maker, mint_x, mint_y, maker_ata_x, escrow, vault_x, _, _] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Validate maker's signature
        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        };

        // Validate escrow PDA
        let (derived_escrow, escrow_bump) =
            find_program_address(&[b"escrow", maker.key().as_ref()], &crate::ID);

        if derived_escrow != *escrow.key() {
            return Err(ProgramError::InvalidAccountData);
        }

        // token `x` vault, validation
        let (derived_vault, _) = find_program_address(
            &[b"vault", maker.key().as_ref(), mint_x.key().as_ref()],
            &crate::ID,
        );

        // check for address
        if derived_vault != *vault_x.key() {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check escrow initialization status
        if !escrow.data_is_empty() || !vault_x.data_is_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        };

        Ok(Self {
            maker,
            mint_x,
            mint_y,
            maker_ata_x,
            escrow,
            vault_x,
            escrow_bump,
        })
    }
}

pub struct MakeOfferArgs {
    pub amount: u64, // Amount of token Y to receive in exchange
}

impl<'a> TryFrom<&'a [u8]> for MakeOfferArgs {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() != core::mem::size_of::<u64>() {
            return Err(ProgramError::InvalidInstructionData);
        }

        let amount = u64::from_be_bytes(data.try_into().unwrap());
        if amount == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self { amount })
    }
}

/// Complete Make instruction handler
pub struct MakeOfferInstruction<'a> {
    pub accounts: MakeOfferAccounts<'a>,
    pub data: MakeOfferArgs,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountInfo])> for MakeOfferInstruction<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountInfo])) -> Result<Self, Self::Error> {
        let accounts = MakeOfferAccounts::try_from(accounts)?;
        let data = MakeOfferArgs::try_from(data)?;
        Ok(Self { accounts, data })
    }
}

impl<'a> MakeOfferInstruction<'a> {
    /// Instruction discriminator for Make instruction
    pub const DISCRIMINATOR: &'a u8 = &0;

    /// Process the Make instruction
    ///
    /// Creates an escrow account and transfers token X from maker to vault
    pub fn process(&mut self) -> ProgramResult {
        // Initialize escrow account
        CreateAccount {
            from: self.accounts.maker,
            to: self.accounts.escrow,
            lamports: Rent::get()?.minimum_balance(Escrow::LEN),
            space: Escrow::LEN as u64,
            owner: &crate::ID,
        }
        .invoke()?;

        // initilize the vault
        InitializeAccount3 {
            account: self.accounts.vault_x,
            mint: self.accounts.mint_x,
            owner: &self.accounts.escrow.key(),
        }
        .invoke()?;

        // Set escrow data
        let mut escrow_ref = self.accounts.escrow.try_borrow_mut_data()?;
        let escrow = bytemuck::try_from_bytes_mut::<Escrow>(&mut escrow_ref)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        escrow.maker = *self.accounts.maker.key();
        escrow.mint_x = *self.accounts.mint_x.key();
        escrow.mint_y = *self.accounts.mint_y.key();
        escrow.receive = self.data.amount;
        escrow.bump = self.accounts.escrow_bump;

        // Transfer tokens to vault
        Transfer {
            from: self.accounts.maker_ata_x,
            to: self.accounts.vault_x,
            amount: self.data.amount,
            authority: self.accounts.maker,
        }
        .invoke()?;

        Ok(())
    }
}
