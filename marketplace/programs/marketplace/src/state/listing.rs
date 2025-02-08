use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Listing {
    pub maker: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
    pub price: u64,
}
