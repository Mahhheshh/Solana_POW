use anchor_lang::{ prelude::*, system_program::Transfer, system_program::transfer };
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        Mint,
        TokenAccount,
        TokenInterface,
        TransferChecked,
        transfer_checked,
        CloseAccount,
        close_account,
    },
};

use crate::state::{ Listing, MarketPlace };

#[derive(Accounts)]
pub struct Purchase<'info> {
    // taker account (who is buying the nft)
    #[account(mut)]
    pub taker: Signer<'info>,
    // maker account (who listed the nft)
    #[account(mut)]
    pub maker: SystemAccount<'info>,
    // marketplace account
    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes()],
        bump = marketplace.bump
    )]
    pub marketplace: Account<'info, MarketPlace>,
    // nft mint account
    pub maker_mint: InterfaceAccount<'info, Mint>,

    // taker ata for recieving nft
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = maker_mint,
        associated_token::authority = taker
    )]
    pub taker_ata: InterfaceAccount<'info, TokenAccount>,
    // vault for storing the nft
    #[account(
        mut,
        associated_token::mint = maker_mint,
        associated_token::authority = listing
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    // listing account strores metadata for the listing
    #[account(
        mut,
        close = maker,
        seeds = [marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump = listing.bump
    )]
    pub listing: Account<'info, Listing>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl<'info> Purchase<'info> {
    pub fn transfer_sol(&mut self) -> Result<()> {
        let lamports = self.listing.price;
        let cpi_program = self.system_program.to_account_info();

        let cpi_accounts = Transfer {
            from: self.taker.to_account_info(),
            to: self.maker.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, lamports)
    }
    pub fn transfer_nft(&mut self) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();
        let cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            to: self.taker_ata.to_account_info(),
            authority: self.listing.to_account_info(),
            mint: self.maker_mint.to_account_info(),
        };
        let signer_seeds: &[&[&[u8]]] = &[
            &[
                &self.marketplace.key().to_bytes()[..],
                &self.maker_mint.key().to_bytes()[..],
                &[self.listing.bump],
            ],
        ];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer_checked(cpi_ctx, 1, self.maker_mint.decimals)
    }
    pub fn close_listing(&mut self) -> Result<()> {
        let signer_seeds: &[&[&[u8]]] = &[
            &[
                &self.marketplace.key().to_bytes()[..],
                &self.maker_mint.key().to_bytes()[..],
                &[self.listing.bump],
            ],
        ];
        let accounts = CloseAccount {
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.listing.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            accounts,
            signer_seeds
        );
        close_account(cpi_ctx)
    }
}
