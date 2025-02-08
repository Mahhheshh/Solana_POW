use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        transfer_checked,
        Mint,
        TokenAccount,
        TokenInterface,
        TransferChecked,
        CloseAccount,
        close_account,
    },
};

use crate::state::{ Listing, MarketPlace };

#[derive(Accounts)]
pub struct Delist<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes()],
        bump = marketplace.bump
    )]
    pub marketplace: Account<'info, MarketPlace>,

    pub maker_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint=maker_mint,
        associated_token::authority=maker
    )]
    pub maker_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint=maker_mint,
        associated_token::authority = listing,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        close=maker,
        seeds=[marketplace.key().as_ref(),maker_mint.key().as_ref()],
        bump=listing.bump,
    )]
    pub listing: Account<'info, Listing>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Delist<'info> {
    pub fn refund_nft(&mut self) -> Result<()> {
        let program = self.token_program.to_account_info();
        let marketplace_key = self.marketplace.key();
        let mint_key = self.maker_mint.key();
        let accounts = TransferChecked {
            authority: self.listing.to_account_info(),
            from: self.vault.to_account_info(),
            mint: self.maker_mint.to_account_info(),
            to: self.maker_ata.to_account_info(),
        };
        let seeds: &[&[&[u8]]] = &[
            &[marketplace_key.as_ref(), mint_key.as_ref(), &[self.marketplace.bump]],
        ];
        let ctx = CpiContext::new_with_signer(program, accounts, seeds);
        transfer_checked(ctx, 1, self.maker_mint.decimals)
    }

    pub fn close_vault(&mut self) -> Result<()> {
        let program = self.associated_token_program.to_account_info();
        let marketplace_key = self.marketplace.key();
        let mint_key = self.maker_mint.key();
        let accounts = CloseAccount {
            account: self.vault.to_account_info(),
            authority: self.listing.to_account_info(),
            destination: self.maker.to_account_info(),
        };
        let signer_seeds: &[&[&[u8]]] = &[
            &[marketplace_key.as_ref(), mint_key.as_ref(), &[self.marketplace.bump]],
        ];
        let ctx = CpiContext::new_with_signer(program, accounts, signer_seeds);
        close_account(ctx)
    }
}
