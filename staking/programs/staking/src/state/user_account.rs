use anchor_lang::prelude::*;

#[account()]
#[derive(InitSpace)]
pub struct UserAccount {
    // number of reward token
    pub points: u32,
    // number of nft staked
    pub amount_staked: u8,
    pub bump: u8,
}