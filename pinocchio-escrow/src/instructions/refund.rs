use pinocchio::{
    account_info::AccountInfo,
    instruction::{Seed, Signer},
    program_error::ProgramError,
    pubkey::find_program_address,
    ProgramResult,
};
use pinocchio_token::instructions::{CloseAccount, Transfer};

// # Accounts
// - `maker`: The creator of the escrow (signer)
// - `mint_x`: The mint account for token X
// - `maker_ata_x`: The maker's associated token account for token X
// - `escrow`: The escrow PDA account
// - `vault_x`: The vault PDA account holding the escrowed tokens
pub struct RefundAccounts<'a> {
    pub maker: &'a AccountInfo,
    pub mint_x: &'a AccountInfo,
    pub maker_ata_x: &'a AccountInfo,
    pub escrow: &'a AccountInfo,
    pub vault_x: &'a AccountInfo,
    pub escrow_bump: u8,
}

impl<'a> TryFrom<&'a [AccountInfo]> for RefundAccounts<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        // destrucutre
        let [maker, mint_x, maker_ata_x, escrow, vault_x] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        // validate the signer
        if !maker.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        };

        // check for pda owner
        if !escrow.is_owned_by(&crate::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // check for escrow pubkey match with derived key
        let (derived_escrow, escrow_bump) =
            find_program_address(&[b"escrow", maker.key().as_ref()], &crate::ID);

        // check for valid pda
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

        // check for account initilization
        if escrow.data_is_empty() || vault_x.data_is_empty() {
            return Err(ProgramError::UninitializedAccount);
        }
        Ok(Self {
            maker,
            mint_x,
            maker_ata_x,
            escrow,
            vault_x,
            escrow_bump,
        })
    }
}

pub struct RefundInstruction<'a> {
    pub accounts: RefundAccounts<'a>,
}

impl<'a> TryFrom<&'a [AccountInfo]> for RefundInstruction<'a> {
    type Error = ProgramError;

    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let accounts = RefundAccounts::try_from(accounts)?;

        Ok(Self { accounts })
    }
}

impl<'a> RefundInstruction<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;

    pub fn process(&mut self) -> ProgramResult {
        let bump = [self.accounts.escrow_bump.to_le()];
        let seed = [
            Seed::from(b"escrow"),
            Seed::from(self.accounts.maker.key()),
            Seed::from(&bump),
        ];
        let seeds = Signer::from(&seed);

        // Transfer tokens from vault back to maker's associated token account
        Transfer {
            from: self.accounts.vault_x,
            to: self.accounts.maker_ata_x,
            authority: self.accounts.mint_x,
            amount: self.accounts.vault_x.lamports(),
        }
        .invoke_signed(&[seeds.clone()])?;

        // Close the vault account and return rent to the maker
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
