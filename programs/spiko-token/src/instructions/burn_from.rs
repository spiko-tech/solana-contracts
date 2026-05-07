use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::SpTokenError;
use crate::events::Burned;
use crate::state::*;

#[derive(Accounts)]
#[event_cpi]
pub struct BurnFrom<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump = token_config.bump,
    )]
    pub token_config: Account<'info, TokenConfig>,

    #[account(
        mut,
        address = token_config.mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program,
    )]
    pub source: InterfaceAccount<'info, TokenAccount>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        bump = permission_manager_config.bump,
        seeds::program = permission_manager_program_id(),
        constraint = is_admin(&permission_manager_config, &admin.key()) @ SpTokenError::Unauthorized,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    #[account(
        seeds = [MINT_AUTHORITY_SEED, mint.key().as_ref()],
        bump = mint_authority.bump,
    )]
    pub mint_authority: Account<'info, MintAuthority>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler(ctx: Context<BurnFrom>, amount: u64) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();
    let seeds = &[
        MINT_AUTHORITY_SEED,
        mint_key.as_ref(),
        &[ctx.accounts.mint_authority.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = Burn {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.source.to_account_info(),
        authority: ctx.accounts.mint_authority.to_account_info(),
    };
    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, signer_seeds);
    token_interface::burn(cpi_ctx, amount)?;

    emit_cpi!(Burned {
        caller: ctx.accounts.admin.key(),
        mint: ctx.accounts.mint.key(),
        source: ctx.accounts.source.key(),
        amount,
    });

    Ok(())
}
