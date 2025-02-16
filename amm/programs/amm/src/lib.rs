use anchor_lang::prelude::*;
mod instructions;
mod state;
declare_id!("3PqMs13NWeiZM8TU6mmSN6vkYqoWDPg1CHnHTa569ZM6");
use crate::instructions::*;
#[program]
pub mod amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee: u16, seeds: u64) -> Result<()> {
        ctx.accounts.initialize(fee, seeds, ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        ctx.accounts.deposit(amount, max_x, max_y)
    }

    pub fn swap(ctx: Context<Swap>, is_x: bool, amount_in: u64, min_out: u64) -> Result<()> {
        ctx.accounts.swap(is_x, amount_in, min_out)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        ctx.accounts.withdraw(amount, max_x, max_y)
    }
}
