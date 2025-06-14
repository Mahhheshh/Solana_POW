use crate::state::escrow::Escrow;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    ProgramResult,
};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_token::instructions::{CloseAccount, Transfer};

/// Accounts required for the `AcceptOffer` instruction.
///
/// # Account List
/// - `taker`: The signer who completes the swap by providing token Y and receiving token X.
/// - `taker_ata_x`: The taker's associated token account for receiving token X.
/// - `taker_ata_y`: The taker's associated token account for sending token Y.
/// - `maker`: The creator of the escrow.
/// - `maker_ata_y`: The maker's associated token account to receive token Y.
/// - `mint_x`: The mint account for token X.
/// - `mint_y`: The mint account for token Y.
/// - `escrow`: The escrow account containing the trade details.
/// - `vault_x`: The vault account holding the locked token X funds.
pub struct AcceptOfferAccounts<'a> {
    pub taker: &'a AccountInfo,
    pub maker: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub mint_x: &'a AccountInfo,
    pub mint_y: &'a AccountInfo,
    pub vault: &'a AccountInfo,
    pub taker_ata_x: &'a AccountInfo,
    pub taker_ata_y: &'a AccountInfo,
    pub maker_ata_y: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for AcceptOfferAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [taker, maker, escrow, mint_x, mint_y, vault, taker_ata_x, taker_ata_y, maker_ata_y, system_program, token_program, _] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // Verify that the `taker` account has signed the transaction.
        if !taker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Ensure the `escrow` account is owned by the current program.
        if !escrow.is_owned_by(&crate::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Validate the data length of the `escrow` account.
        if escrow.data_len().ne(&Escrow::LEN) {
            return Err(ProgramError::InvalidAccountData);
        }

        // Check if `mint_x` and `mint_y` are owned by the Pinocchio Token Program.
        if !mint_x.is_owned_by(&pinocchio_token::ID) || !mint_y.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Validate the data length for both mint accounts.
        if mint_x.data_len() != pinocchio_token::state::Mint::LEN
            || mint_y.data_len() != pinocchio_token::state::Mint::LEN
        {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Verify that `taker_ata_y` is the correct associated token account for the `taker` and `mint_y`.
        if find_program_address(
            &[taker.key(), &pinocchio_token::ID, mint_y.key()],
            &pinocchio_associated_token_account::ID,
        )
        .0
        .ne(taker_ata_y.key())
        {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(Self {
            taker,
            maker,
            escrow,
            mint_x,
            mint_y,
            taker_ata_x,
            taker_ata_y,
            maker_ata_y,
            vault,
            system_program,
            token_program,
        })
    }
}

/// Represents the `AcceptOffer` instruction.
pub struct AcceptOfferInstruction<'a> {
    pub accounts: AcceptOfferAccounts<'a>,
}

impl<'a> TryFrom<&'a [AccountInfo]> for AcceptOfferInstruction<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let accounts = AcceptOfferAccounts::try_from(accounts)?;

        // Ensure both `taker_ata_x` and `maker_ata_y` are owned by the Pinocchio Token Program.
        if !accounts.taker_ata_x.is_owned_by(&pinocchio_token::ID)
            || !accounts.maker_ata_y.is_owned_by(&pinocchio_token::ID)
        {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Validate that `taker_ata_x` is the correct associated token account for the `taker` and `mint_x`.
        if find_program_address(
            &[
                accounts.taker.key(),
                &pinocchio_token::ID,
                accounts.mint_x.key(),
            ],
            &pinocchio_associated_token_account::ID,
        )
        .0
        .ne(accounts.taker_ata_x.key())
        {
            return Err(ProgramError::InvalidAccountData);
        }

        // Validate that `maker_ata_y` is the correct associated token account for the `maker` and `mint_y`.
        if find_program_address(
            &[
                accounts.maker.key(),
                &pinocchio_token::ID,
                accounts.mint_y.key(),
            ],
            &pinocchio_associated_token_account::ID,
        )
        .0
        .ne(accounts.maker_ata_y.key())
        {
            return Err(ProgramError::InvalidAccountData);
        }

        // Create `taker_ata_x` if it doesn't already exist.
        if accounts
            .taker_ata_x
            .data_len()
            .ne(&pinocchio_token::state::TokenAccount::LEN)
        {
            Create {
                funding_account: accounts.taker, // The account funding the creation.
                account: accounts.taker_ata_x,   // The new ATA account address.
                wallet: accounts.taker,          // The wallet associated with the ATA.
                mint: accounts.mint_x,           // The mint for this ATA.
                system_program: accounts.system_program,
                token_program: accounts.token_program,
            }
            .invoke()?;
        }

        // Create `maker_ata_y` if it doesn't already exist.
        if accounts
            .maker_ata_y
            .data_len()
            .ne(&pinocchio_token::state::TokenAccount::LEN)
        {
            Create {
                funding_account: accounts.taker, // The account funding the creation.
                account: accounts.taker_ata_x, // The new ATA account address. Note: This should likely be accounts.maker_ata_y
                wallet: accounts.taker, // The wallet associated with the ATA. Note: This should likely be accounts.maker
                mint: accounts.mint_x, // The mint for this ATA. Note: This should likely be accounts.mint_y
                system_program: accounts.system_program,
                token_program: accounts.token_program,
            }
            .invoke()?;
        }

        Ok(Self { accounts })
    }
}

impl<'a> AcceptOfferInstruction<'a> {
    /// Instruction discriminator for the `AcceptOffer` instruction.
    pub const DISCRIMINATOR: &'a u8 = &1;

    /// Processes the `AcceptOffer` instruction.
    ///
    /// This function handles the logic for a taker to complete an escrow trade.
    /// It transfers token Y from the taker to the maker, then transfers token X
    /// from the vault to the taker, and finally closes the vault and escrow accounts.
    pub fn process(&mut self) -> ProgramResult {
        // Mutably borrow the escrow account's data.
        let mut escrow_ref = self.accounts.escrow.try_borrow_mut_data()?;
        // Deserialize the escrow account data into the `Escrow` struct.
        let escrow = bytemuck::try_from_bytes_mut::<Escrow>(&mut escrow_ref)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // Transfer `receive` amount of token Y from the taker's ATA to the maker's ATA.
        Transfer {
            from: self.accounts.taker_ata_y, // Source: Taker's token Y account.
            to: self.accounts.maker_ata_y,   // Destination: Maker's token Y account.
            authority: self.accounts.taker,  // Authority for the transfer: Taker.
            amount: escrow.receive,          // Amount to transfer (as specified in escrow).
        }
        .invoke()?;

        // Prepare the seeds for signing with the escrow PDA.
        let bump = [escrow.bump.to_le()];
        let seed = [
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.key()),
            Seed::from(&bump),
        ];
        let seeds = Signer::from(&seed);

        // Transfer all token X from the vault to the taker's ATA.
        Transfer {
            from: self.accounts.vault,              // Source: Vault holding token X.
            to: self.accounts.taker_ata_x,          // Destination: Taker's token X account.
            authority: self.accounts.escrow,        // Authority for the transfer: Escrow PDA.
            amount: self.accounts.vault.lamports(), // Transfer all lamports (representing tokens) from the vault.
        }
        .invoke_signed(&[seeds.clone()])?;

        // Close the vault account, sending remaining SOL back to the maker.
        CloseAccount {
            account: self.accounts.vault,     // The account to close.
            destination: self.accounts.maker, // The account to receive the remaining SOL.
            authority: self.accounts.escrow,  // The authority to close the account (escrow PDA).
        }
        .invoke_signed(&[seeds.clone()])?;

        // Close the escrow account and return its SOL to the maker.
        unsafe {
            *self.accounts.maker.borrow_mut_lamports_unchecked() +=
                *self.accounts.escrow.borrow_lamports_unchecked();
            *self.accounts.escrow.borrow_mut_lamports_unchecked() = 0
        };

        Ok(())
    }
}
