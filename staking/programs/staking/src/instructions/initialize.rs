use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

use crate::StakeConfig;

#[derive(Accounts)]
pub struct InitializeConfig<'info> {

    #[account(mut)]    
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + StakeConfig::INIT_SPACE,
        seeds = [b"stake_config"], 
        bump
    )]
    pub config: Account<'info, StakeConfig>,
    #[account(
        init, 
        payer = admin,
        seeds = [b"rewards", admin.key().as_ref()],
        bump,
        mint::authority = config,
        mint::decimals = 6
    )]
    pub reward_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>
}

impl <'info> InitializeConfig<'info> {
    pub fn initialize(&mut self, point_per_stake: u8, freeze_period: u32, max_stake: u8, bumps: &InitializeConfigBumps,) -> Result<()> {

        self.config.set_inner(StakeConfig {
            point_per_stake,
            max_stake,
            freeze_period,
            reward_bump: bumps.reward_mint,
            bump: bumps.config,
        });
        Ok(())
    }
}