use crate::state::escrow::Escrow;

use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_system::instructions::CreateAccount;
use pinocchio_token::instructions::Transfer;

/// Accounts required for the `Make` instruction.
///
/// # Account List
/// - `maker`: The signer creating the escrow (must sign).
/// - `mint_x`: The mint of token X to be locked in the escrow.
/// - `mint_y`: The mint of token Y that the maker expects to receive.
/// - `maker_ata_x`: The maker's associated token account for token X.
/// - `escrow`: The escrow account where trade details will be stored.
/// - `vault`: The vault account that will temporarily hold token X.
pub struct MakeOfferAccounts<'a> {
    pub maker: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub mint_x: &'a AccountInfo,
    pub mint_y: &'a AccountInfo,
    pub maker_ata_x: &'a AccountInfo,
    pub vault: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for MakeOfferAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [maker, escrow, mint_x, mint_y, maker_ata_x, vault, system_program, token_program, _] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Validate that the `maker` account has signed the transaction.
        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        };

        // Verify that `mint_x` and `mint_y` are owned by the `pinocchio_token` program.
        if !mint_x.is_owned_by(&pinocchio_token::ID) || !mint_y.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Check if the data length for both mint accounts is valid.
        if mint_x.data_len() != pinocchio_token::state::Mint::LEN
            || mint_y.data_len() != pinocchio_token::state::Mint::LEN
        {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Validate that `maker_ata_x` is the correct associated token account for `maker` and `mint_x`.
        if find_program_address(
            &[maker.key(), &pinocchio_token::ID, mint_x.key()],
            &pinocchio_associated_token_account::ID,
        )
        .0
        .ne(maker_ata_x.key())
        {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            maker,
            escrow,
            mint_x,
            mint_y,
            maker_ata_x,
            vault,
            system_program,
            token_program,
        })
    }
}

/// Arguments required for the `Make` instruction.
pub struct MakeOfferArgs {
    pub receive: u64,
    pub amount: u64,
}

impl<'a> TryFrom<&'a [u8]> for MakeOfferArgs {
    type Error = ProgramError;

    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        // Ensure the instruction data has the correct length for two u64 values.
        if data.len() != core::mem::size_of::<u64>() * 2 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let receive = u64::from_le_bytes(data[8..16].try_into().unwrap()); // The amount of token Y the maker expects to receive.
        let amount = u64::from_be_bytes(data[0..8].try_into().unwrap()); // The amount of token X the maker will deposit.

        // Ensure the deposit amount is not zero.
        if amount == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self { receive, amount })
    }
}

/// Complete `Make` instruction handler.
pub struct MakeOfferInstruction<'a> {
    pub accounts: MakeOfferAccounts<'a>,
    pub data: MakeOfferArgs,
    pub bump: u8,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountInfo])> for MakeOfferInstruction<'a> {
    type Error = ProgramError;

    fn try_from((data, accounts): (&'a [u8], &'a [AccountInfo])) -> Result<Self, Self::Error> {
        let accounts = MakeOfferAccounts::try_from(accounts)?;
        let data = MakeOfferArgs::try_from(data)?;

        // Derive the Program Derived Address (PDA) and bump seed for the escrow account.
        let (_, bump) = find_program_address(
            &[b"escrow", accounts.maker.key(), accounts.mint_x.key()],
            &crate::ID,
        );

        // Prepare the seeds for signing the escrow account creation.
        let binding = [bump];
        let seeds = [
            Seed::from(b"escrow"),
            Seed::from(accounts.maker.key().as_ref()),
            Seed::from(accounts.mint_x.key().as_ref()),
            Seed::from(&binding),
        ];

        // Initialize the escrow PDA account.
        CreateAccount {
            from: accounts.maker, // The account funding the creation.
            to: accounts.escrow,  // The escrow account to be created.
            lamports: Rent::get()?.minimum_balance(Escrow::LEN), // Lamports required for rent exemption.
            space: Escrow::LEN as u64, // The data space allocated for the escrow account.
            owner: &crate::ID,         // The program ID that owns the escrow account.
        }
        .invoke_signed(&[Signer::from(&seeds)])?;

        // Initialize the vault Associated Token Account (ATA).
        Create {
            funding_account: accounts.maker, // The account that pays for the creation fee.
            account: accounts.vault,         // The new vault account address.
            wallet: accounts.escrow, // The authority for the vault account (the escrow PDA).
            mint: accounts.mint_x,   // The mint associated with this vault account.
            system_program: accounts.system_program,
            token_program: accounts.token_program,
        }
        .invoke()?;

        Ok(Self {
            accounts,
            data,
            bump,
        })
    }
}

impl<'a> MakeOfferInstruction<'a> {
    /// Instruction discriminator for the `Make` instruction.
    pub const DISCRIMINATOR: &'a u8 = &0;

    /// Processes the `Make` instruction.
    ///
    /// This function creates an escrow account, populates it with trade details,
    /// and transfers the specified amount of token X from the maker's ATA to the vault account.
    pub fn process(&mut self) -> ProgramResult {
        // Mutably borrow the escrow account's data.
        let mut escrow_ref = self.accounts.escrow.try_borrow_mut_data()?;
        // Deserialize the escrow account data into the `Escrow` struct.
        let escrow = bytemuck::try_from_bytes_mut::<Escrow>(&mut escrow_ref)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Set the escrow account's data with the provided details.
        escrow.maker = *self.accounts.maker.key();
        escrow.mint_x = *self.accounts.mint_x.key();
        escrow.mint_y = *self.accounts.mint_y.key();
        escrow.receive = self.data.receive;
        escrow.bump = self.bump;

        // Transfer tokens from the maker's associated token account to the vault.
        Transfer {
            from: self.accounts.maker_ata_x, // The source account for the tokens.
            to: self.accounts.vault,         // The destination vault account.
            amount: self.data.amount,        // The amount of tokens to transfer.
            authority: self.accounts.maker,  // The authority to sign the transfer.
        }
        .invoke()?;

        Ok(())
    }
}
