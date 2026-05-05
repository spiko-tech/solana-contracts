use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::constants::*;
use crate::errors::RedemptionError;
use crate::events::VaultCreated;
use crate::state::*;

#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [REDEMPTION_CONFIG_SEED],
        bump,
    )]
    pub redemption_config: Account<'info, RedemptionConfig>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        bump,
        seeds::program = permission_manager_program_id(),
        constraint = permission_manager_config.admin == admin.key() @ RedemptionError::Unauthorized,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    #[account(mint::token_program = token_program)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = admin,
        space = 8 + VaultAuthority::INIT_SPACE,
        seeds = [VAULT_AUTHORITY_SEED, mint.key().as_ref()],
        bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    #[account(
        init,
        payer = admin,
        associated_token::mint = mint,
        associated_token::authority = vault_authority,
        associated_token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<CreateVault>) -> Result<()> {
    ctx.accounts.vault_authority.bump = ctx.bumps.vault_authority;

    emit!(VaultCreated {
        admin: ctx.accounts.admin.key(),
        mint: ctx.accounts.mint.key(),
    });

    Ok(())
}
