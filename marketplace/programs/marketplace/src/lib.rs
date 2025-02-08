use anchor_lang::prelude::*;

declare_id!("993e6yijhKRWBA5b1DJgagrJD1BpXaiv4fyDBKwS8wZT");
mod state;
mod contexts;
mod error;

use crate::contexts::*;

#[program]
pub mod marketplace {
    use super::*;

    pub fn initialize_market(ctx: Context<Initialize>, fee: u16, name: String) -> Result<()> {
        ctx.accounts.init(fee, name, ctx.bumps)
    }

    pub fn list(ctx: Context<List>, price: u64) -> Result<()> {
        ctx.accounts.create_listing(price, ctx.bumps)?;
        ctx.accounts.deposit_nft()
    }

    pub fn delist(ctx: Context<Delist>) -> Result<()> {
        ctx.accounts.refund_nft()?;
        ctx.accounts.close_vault()
    }

    pub fn purchase(ctx: Context<Purchase>) -> Result<()> {
        ctx.accounts.transfer_sol()?;
        ctx.accounts.transfer_nft()?;
        ctx.accounts.close_listing()
    }
}
