use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, SetAuthority, TokenInterface};
use spl_token_2022_interface::instruction::AuthorityType;

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::{MinterConfig, PendingMintAuthorityTransfer};

#[derive(Accounts)]
pub struct AcceptMintAuthority<'info> {
    pub new_authority: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        close = new_authority,
        seeds = [PENDING_MINT_AUTHORITY_SEED, mint.key().as_ref()],
        bump = pending_transfer.bump,
        constraint = pending_transfer.new_authority == new_authority.key() @ MinterError::Unauthorized,
        constraint = pending_transfer.mint == mint.key() @ MinterError::Unauthorized,
    )]
    pub pending_transfer: Account<'info, PendingMintAuthorityTransfer>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler(ctx: Context<AcceptMintAuthority>) -> Result<()> {
    let seeds = &[MINTER_CONFIG_SEED, &[ctx.accounts.minter_config.bump]];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = SetAuthority {
        current_authority: ctx.accounts.minter_config.to_account_info(),
        account_or_mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, signer_seeds);
    token_interface::set_authority(
        cpi_ctx,
        AuthorityType::MintTokens,
        Some(ctx.accounts.new_authority.key()),
    )?;

    Ok(())
}
