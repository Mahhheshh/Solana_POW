use crate::state::escrow::Escrow;
use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    ProgramResult,
};
use pinocchio_token::instructions::{CloseAccount, Transfer};

/// # Account List
/// - `taker` - Signer who completes the swap by providing token Y and receiving token X
/// - `taker_ata_x` - Taker's associated token account for token X (receiving)
/// - `taker_ata_y` - Taker's associated token account for token Y (sending)
/// - `maker` - escrow creator
/// - `maker_ata_y` - Maker's associated token account to receive token Y
/// - `mint_x` - mint for `x` token
/// - `mint_y` - mint of `y` token
/// - `escrow` - Escrow account containing the trade details
/// - `vault_x` - Vault account holding the locked token X funds
pub struct AcceptOfferAccounts<'a> {
    pub taker: &'a AccountInfo,
    pub taker_ata_x: &'a AccountInfo,
    pub taker_ata_y: &'a AccountInfo,
    pub maker: &'a AccountInfo,
    pub maker_ata_y: &'a AccountInfo,
    pub mint_y: &'a AccountInfo,
    pub mint_x: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub vault_x: &'a AccountInfo,
}

impl<'a> TryFrom<&'a [AccountInfo]> for AcceptOfferAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [taker, taker_ata_x, taker_ata_y, maker, maker_ata_y, mint_y, mint_x, escrow, vault_x, _, _] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !taker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        // escrow has to be owned by the pinocchio program
        if !escrow.is_owned_by(&crate::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // check for the bump and key
        let (derived_escrow, _) =
            find_program_address(&[b"escrow", maker.key().as_ref()], &crate::ID);

        // check if the pdas match
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

        // escrow account should be initilized
        if escrow.data_is_empty() || vault_x.data_is_empty() {
            return Err(ProgramError::UninitializedAccount);
        }

        Ok(Self {
            taker,
            taker_ata_x,
            taker_ata_y,
            maker,
            maker_ata_y,
            escrow,
            vault_x,
            mint_y,
            mint_x,
        })
    }
}

pub struct AcceptOfferInstruction<'a> {
    pub accounts: AcceptOfferAccounts<'a>,
}

impl<'a> TryFrom<&'a [AccountInfo]> for AcceptOfferInstruction<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let accounts = AcceptOfferAccounts::try_from(accounts)?;

        Ok(Self { accounts })
    }
}

impl<'a> AcceptOfferInstruction<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;

    pub fn process(&mut self) -> ProgramResult {
        // Borrow a reference to the escrow account data
        let mut escrow_ref = self.accounts.escrow.try_borrow_mut_data()?;
        let escrow = bytemuck::try_from_bytes_mut::<Escrow>(&mut escrow_ref)
            .map_err(|_| ProgramError::InvalidAccountData)?;

        // transfer `y` tokens from taker's ata to maker's ata
        Transfer {
            from: self.accounts.taker_ata_y,
            to: self.accounts.maker_ata_y,
            authority: self.accounts.taker,
            amount: escrow.receive,
        }
        .invoke()?;

        let bump = [escrow.bump.to_le()];
        let seed = [
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.key()),
            Seed::from(&bump),
        ];
        let seeds = Signer::from(&seed);

        // transfer `x` tokens from the vault_x to the taker_ata_x
        Transfer {
            from: self.accounts.vault_x,
            to: self.accounts.taker_ata_x,
            authority: self.accounts.escrow,
            amount: self.accounts.vault_x.lamports(),
        }
        .invoke_signed(&[seeds.clone()])?;

        // close the escrow, and vault, send the money to the maker
        CloseAccount {
            account: self.accounts.vault_x,
            destination: self.accounts.maker,
            authority: self.accounts.escrow,
        }
        .invoke_signed(&[seeds.clone()])?;

        // close escrow
        unsafe {
            *self.accounts.maker.borrow_mut_lamports_unchecked() +=
                *self.accounts.escrow.borrow_lamports_unchecked();
            *self.accounts.escrow.borrow_mut_lamports_unchecked() = 0
        };

        Ok(())
    }
}
