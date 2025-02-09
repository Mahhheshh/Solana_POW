use anchor_lang::prelude::*;
use anchor_spl::{metadata::{mpl_token_metadata::instructions::{ThawDelegatedAccountCpi, ThawDelegatedAccountCpiAccounts}, MasterEditionAccount, Metadata}, token::{revoke, Mint, Revoke, Token, TokenAccount}};
use crate::{error::CustomError, StakeAccounts, StakeConfig, UserAccount};

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub nft_mint: Account<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = user
    )]
    pub nft_mint_ata: Account<'info, TokenAccount>,

    #[account(
        seeds = [
            b"metadata",
            metadata_program.key().as_ref(),
            nft_mint.key().as_ref(),
            b"edition"
        ],
        bump,
        seeds::program = metadata_program.key(),    
    )]
    pub edition: Account<'info, MasterEditionAccount>,

    #[account(
        mut,
        close = user,
        seeds = [
            b"stake_account",
            nft_mint.key().as_ref()
        ],
        bump
    )]
    pub stake_account: Account<'info, StakeAccounts>,

    #[account{
        seeds = [b"config".as_ref()],
        bump = config.bump
    }]
    pub config: Account<'info, StakeConfig>,

    #[account(
        mut,
        seeds = [
            b"user".as_ref(),
            user.key().as_ref()
        ],
        bump = user_account.bump,
    )]
    pub user_account : Account<'info, UserAccount>,
    
    pub token_program: Program<'info, Token>, 
    pub metadata_program: Program<'info, Metadata>,
    pub system_program: Program<'info, System>
}


impl <'info> Unstake<'info> { 
    pub fn unstake(&mut self) -> Result<()> {
        let time_elapsed = ((Clock::get()?.unix_timestamp - self.stake_account.staked_at) / 86400) as u32;

        require!(time_elapsed >= self.config.freeze_period, CustomError::FreezePeriodNotPassed);


        let seeds = &[
            b"stake_account", 
            self.config.to_account_info().key.as_ref(),
            self.nft_mint.to_account_info().key.as_ref(),
            &[self.stake_account.bump]
        ];
        let signed_seeds = &[&seeds[..]];
        

        let cpi_program = &self.metadata_program.to_account_info();
        let cpi_accounts = ThawDelegatedAccountCpiAccounts {
                delegate : &self.stake_account.to_account_info(),
                token_account: &self.nft_mint_ata.to_account_info(),    
                edition: &self.edition.to_account_info(),
                mint: &self.nft_mint.to_account_info(),
                token_program: &self.token_program.to_account_info()
        };

        ThawDelegatedAccountCpi::new(cpi_program, cpi_accounts).invoke_signed(signed_seeds)?;

        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = Revoke {
            source: self.nft_mint.to_account_info(),
            authority: self.user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        revoke(cpi_ctx)?;

        self.user_account.amount_staked -= 1;

        Ok(())
    }
}