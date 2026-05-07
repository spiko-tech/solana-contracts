use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, SetAuthority, TokenInterface};
use spl_token_2022_interface::instruction::AuthorityType;

use crate::constants::*;
use crate::errors::SpTokenError;
use crate::events::TokenInitialized;
use crate::state::*;

#[derive(Accounts)]
#[event_cpi]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        mint::token_program = token_program,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = 8 + TokenConfig::INIT_SPACE,
        seeds = [TOKEN_CONFIG_SEED, mint.key().as_ref()],
        bump,
    )]
    pub token_config: Account<'info, TokenConfig>,

    #[account(
        init,
        payer = admin,
        space = 8 + MintAuthority::INIT_SPACE,
        seeds = [MINT_AUTHORITY_SEED, mint.key().as_ref()],
        bump,
    )]
    pub mint_authority: Account<'info, MintAuthority>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        bump = permission_manager_config.bump,
        seeds::program = permission_manager_program_id(),
        constraint = permission_manager_config.admin == admin.key() @ SpTokenError::Unauthorized,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler(ctx: Context<Initialize>) -> Result<()> {
    ctx.accounts.token_config.set_inner(TokenConfig {
        paused: false,
        permission_manager: ctx.accounts.permission_manager_config.key(),
        mint: ctx.accounts.mint.key(),
        bump: ctx.bumps.token_config,
    });

    let mint_authority = &mut ctx.accounts.mint_authority;
    mint_authority.bump = ctx.bumps.mint_authority;

    // Transfer mint authority from admin to PDA
    let cpi_accounts = SetAuthority {
        current_authority: ctx.accounts.admin.to_account_info(),
        account_or_mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);
    token_interface::set_authority(
        cpi_ctx,
        AuthorityType::MintTokens,
        Some(ctx.accounts.mint_authority.key()),
    )?;

    emit_cpi!(TokenInitialized {
        admin: ctx.accounts.admin.key(),
        mint: ctx.accounts.mint.key(),
        permission_manager: ctx.accounts.permission_manager_config.key(),
    });

    Ok(())
}
