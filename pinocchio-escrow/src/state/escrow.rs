use bytemuck::{Pod, Zeroable};
use pinocchio::pubkey::Pubkey;

#[repr(C, packed)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Escrow {
    pub maker: Pubkey,  // maker's pubkey
    pub mint_x: Pubkey, // offering token's mint address
    pub mint_y: Pubkey, // receiving token's mint address
    pub receive: u64, // amount to receive in exchange of token x
    pub bump: u8 // store the bump of the Account
}

impl Escrow {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 1;
}