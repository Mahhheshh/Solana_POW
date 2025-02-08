use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{ MasterEditionAccount, Metadata, MetadataAccount },
    token_interface::{ transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked },
};

use crate::state::{ Listing, MarketPlace };

#[derive(Accounts)]
pub struct List<'info> {
    //makes the listing for the NFT
    #[account(mut)]
    pub maker: Signer<'info>,

    //marketplace metadata verification
    #[account(
        seeds = [b"marketplace", marketplace.name.as_str().as_bytes()],
        bump = marketplace.bump
    )]
    pub marketplace: Account<'info, MarketPlace>,

    // nft mint account
    pub maker_mint: InterfaceAccount<'info, Mint>,

    // maker ata for transferring nft
    #[account(
        mut,
        associated_token::mint= maker_mint,
        associated_token::authority= maker,
    )]
    pub maker_ata: InterfaceAccount<'info, TokenAccount>,

    // vault for storing the nft
    #[account(
        init,
        payer = maker,
        associated_token::mint = maker_mint,
        associated_token::authority = listing
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    // listing account strores metadata for the listing
    #[account(
        init,
        payer = maker,
        seeds = [marketplace.key().as_ref(), maker_mint.key().as_ref()],
        bump,
        space = 8 + Listing::INIT_SPACE
    )]
    pub listing: Account<'info, Listing>,

    // Collection mint account nft is part of
    pub collection_mint: InterfaceAccount<'info, Mint>,

    // metadata account for the nft
    #[account(
        seeds = [b"metadata", metadata_program.key().as_ref(), maker_mint.key().as_ref()],
        seeds::program = metadata_program.key(),
        bump,
        constraint = metadata.collection.as_ref().unwrap().key.as_ref() ==
        collection_mint.key().as_ref(),
        constraint = metadata.collection.as_ref().unwrap().verified == true
    )]
    pub metadata: Account<'info, MetadataAccount>,

    // master edition account for the nft
    #[account(
        seeds = [
            b"metadata",
            metadata_program.key().as_ref(),
            maker_mint.key().as_ref(),
            b"edition",
        ],
        seeds::program = metadata_program.key(),
        bump
    )]
    pub master_edition_account: Account<'info, MasterEditionAccount>,

    // programs
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub metadata_program: Program<'info, Metadata>,
}

impl<'info> List<'info> {
    pub fn create_listing(&mut self, price: u64, bumps: ListBumps) -> Result<()> {
        self.listing.set_inner(Listing {
            maker: self.maker.key(),
            mint: self.maker_mint.key(),
            price,
            bump: bumps.listing,
        });
        Ok(())
    }

    pub fn deposit_nft(&mut self) -> Result<()> {
        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = TransferChecked {
            from: self.maker_ata.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.maker.to_account_info(),
            mint: self.maker_mint.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer_checked(cpi_ctx, 1, self.maker_mint.decimals)
    }
}
