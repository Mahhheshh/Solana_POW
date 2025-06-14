use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    ProgramResult,
};
use pinocchio_token::instructions::{CloseAccount, Transfer};

/// Accounts required for the `Refund` instruction.
///
/// # Accounts
/// - `maker`: The creator of the escrow, who is requesting the refund (must sign).
/// - `mint_x`: The mint account for token X, which was originally locked in the escrow.
/// - `maker_ata_x`: The maker's associated token account for token X, where the refunded tokens will be sent.
/// - `escrow`: The escrow Program Derived Address (PDA) account containing the trade details.
/// - `vault`: The vault PDA account holding the escrowed token X funds.
pub struct RefundAccounts<'a> {
    pub maker: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub mint_x: &'a AccountInfo,
    pub maker_ata_x: &'a AccountInfo,
    pub vault: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for RefundAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        // Destructure the accounts slice into individual account references.
        let [maker, escrow, mint_x, maker_ata_x, vault, system_program, token_program] = accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Validate that the `maker` account has signed the transaction.
        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        };

        // Ensure the `escrow` account is owned by the current program.
        if !escrow.is_owned_by(&crate::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Check if `mint_x` is owned by the Pinocchio Token Program.
        if !mint_x.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Validate the data length of the `mint_x` account.
        if mint_x.data_len() != pinocchio_token::state::Mint::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        // Ensure the `escrow` account is not empty (i.e., it's initialized).
        if escrow.data_is_empty() {
            return Err(ProgramError::UninitializedAccount);
        }

        // Validate that `maker_ata_x` is the correct associated token account for the `maker` and `mint_x`.
        if find_program_address(
            &[maker.key(), &pinocchio_token::ID, mint_x.key()],
            &pinocchio_associated_token_account::ID,
        )
        .0
        .ne(maker_ata_x.key())
        {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate that `vault` is the correct associated token account for the `escrow` and `mint_x`.
        if find_program_address(
            &[escrow.key(), &pinocchio_token::ID, mint_x.key()],
            &pinocchio_associated_token_account::ID,
        )
        .0
        .ne(vault.key())
        {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            maker,
            mint_x,
            maker_ata_x,
            escrow,
            vault,
            system_program,
            token_program,
        })
    }
}

/// Represents the `Refund` instruction.
pub struct RefundInstruction<'a> {
    pub accounts: RefundAccounts<'a>,
    pub bump: u8,
}

impl<'a> TryFrom<&'a [AccountInfo]> for RefundInstruction<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let accounts = RefundAccounts::try_from(accounts)?;

        // Derive the Program Derived Address (PDA) and bump seed for the escrow account.
        let (derived_escrow, bump) = find_program_address(
            &[b"escrow", accounts.maker.key(), accounts.mint_x.key()],
            &crate::ID,
        );

        // Verify that the provided `escrow` account matches the derived PDA.
        if *accounts.escrow.key() != derived_escrow {
            return Err(ProgramError::InvalidAccountData);
        }

        // The escrow account's authenticity is validated by checking its PDA derivation
        // using the maker's key and mint_x, eliminating the need to load its data
        // for further maker account validation.

        Ok(Self { accounts, bump })
    }
}

impl<'a> RefundInstruction<'a> {
    /// Instruction discriminator for the `Refund` instruction.
    pub const DISCRIMINATOR: &'a u8 = &2;

    /// Processes the `Refund` instruction.
    ///
    /// This function handles the logic for refunding tokens from the vault back to the maker
    /// and then closing both the vault and escrow accounts.
    pub fn process(&mut self) -> ProgramResult {
        // Prepare the seeds for signing with the escrow PDA.
        let bump = [self.bump.to_le()];
        let seed = [
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.key()),
            Seed::from(&bump),
        ];
        let seeds = Signer::from(&seed);

        // Transfer all token X from the vault back to the maker's associated token account.
        Transfer {
            from: self.accounts.vault,              // Source: Vault holding token X.
            to: self.accounts.maker_ata_x,          // Destination: Maker's token X account.
            authority: self.accounts.mint_x, // Authority for the transfer: The mint_x account.
            amount: self.accounts.vault.lamports(), // Transfer all lamports (representing tokens) from the vault.
        }
        .invoke_signed(&[seeds.clone()])?;

        // Close the vault account, sending its remaining SOL (rent exemption) back to the maker.
        CloseAccount {
            account: self.accounts.vault,     // The account to close.
            destination: self.accounts.maker, // The account to receive the remaining SOL.
            authority: self.accounts.escrow,  // The authority to close the account (escrow PDA).
        }
        .invoke_signed(&[seeds.clone()])?;

        // Close the escrow account and transfer its remaining SOL (rent exemption) back to the maker.
        unsafe {
            *self.accounts.maker.borrow_mut_lamports_unchecked() +=
                *self.accounts.escrow.borrow_lamports_unchecked();
            *self.accounts.escrow.borrow_mut_lamports_unchecked() = 0
        };

        Ok(())
    }
}
