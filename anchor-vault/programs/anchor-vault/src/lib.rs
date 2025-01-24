use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

declare_id!("7arWQZYX189RjNUCBND9X6NRt9X7x1uijwvM7wDGRMDR");

#[program]
pub mod anchor_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    pub fn deposit(ctx: Context<Payments>, amount: u64) -> Result<()> {
        ctx.accounts.deposit(amount)
    }

    pub fn withdraw(ctx: Context<Payments>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw(amount)
    }

    pub fn close_vault(ctx: Context<CloseVault>) -> Result<()> {
        ctx.accounts.close_vault()
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        seeds = [b"vault", signer.key().as_ref()],
        space = 8 + Vault::INIT_SPACE,
        bump
    )]
    pub vault_state: Account<'info, Vault>,
    #[account(seeds = [vault_state.key().as_ref()], bump)]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bump: &InitializeBumps) -> Result<()> {
        let vault_state = &mut self.vault_state;
        vault_state.state_bump = bump.vault_state;
        vault_state.vault_bump = bump.vault;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Payments<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", signer.key().as_ref()],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, Vault>,
    #[account(mut,seeds = [vault_state.key().as_ref()], bump = vault_state.vault_bump)]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Payments<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        let system_program = self.system_program.to_account_info();
        let from_pubkey = self.signer.to_account_info();
        let to_pubkey = self.vault.to_account_info();
        let cpi = CpiContext::new(system_program, Transfer {
            from: from_pubkey,
            to: to_pubkey,
        });
        transfer(cpi, amount)?;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        let from_pubkey = self.vault.to_account_info();
        let to_pubkey = self.signer.to_account_info();
        let system_program = self.system_program.to_account_info();
        let bump_seed = self.vault_state.vault_bump;
        let vault = self.vault_state.key();
        let signer_seed: &[&[&[u8]]] = &[&[vault.as_ref(), &[bump_seed]]];
        let cpi = CpiContext::new(system_program, Transfer {
            from: from_pubkey,
            to: to_pubkey,
        }).with_signer(signer_seed);
        transfer(cpi, amount)?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CloseVault<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"vault", signer.key().as_ref()],
        bump = vault_state.state_bump,
        close = signer
    )]
    pub vault_state: Account<'info, Vault>,
    #[account(mut,seeds = [vault_state.key().as_ref()], bump = vault_state.vault_bump)]
    pub vault: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> CloseVault<'info> {
    pub fn close_vault(&mut self) -> Result<()> {
        let from_pubkey = self.vault.to_account_info();
        let to_pubkey = self.signer.to_account_info();
        let system_program = self.system_program.to_account_info();
        let amount = self.vault.lamports();
        let bump_seed = self.vault_state.vault_bump;
        let vault = self.vault_state.key();
        let signer_seed: &[&[&[u8]]] = &[&[vault.as_ref(), &[bump_seed]]];
        let cpi = CpiContext::new(system_program, Transfer {
            from: from_pubkey,
            to: to_pubkey,
        }).with_signer(signer_seed);
        transfer(cpi, amount)?;
        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct Vault {
    pub state_bump: u8,
    pub vault_bump: u8,
}