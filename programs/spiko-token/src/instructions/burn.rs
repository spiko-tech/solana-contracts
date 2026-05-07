use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::SpTokenError;
use crate::events::Burned;
use crate::state::*;

#[derive(Accounts)]
#[event_cpi]
pub struct BurnTokens<'info> {
    pub burner: Signer<'info>,

    #[account(
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump = token_config.bump,
        constraint = !token_config.paused @ SpTokenError::TokenPaused,
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
        token::authority = burner,
        token::token_program = token_program,
    )]
    pub source: InterfaceAccount<'info, TokenAccount>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [USER_PERMISSION_SEED, burner.key().as_ref(), permission_manager_config.key().as_ref()],
        seeds::program = permission_manager_program_id(),
        bump = burner_permissions.bump,
        constraint = has_role(&burner_permissions, ROLE_BURNER) @ SpTokenError::Unauthorized,
    )]
    pub burner_permissions: Account<'info, UserPermissions>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        seeds::program = permission_manager_program_id(),
        bump = permission_manager_config.bump,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
    let cpi_accounts = Burn {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.source.to_account_info(),
        authority: ctx.accounts.burner.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.key(), cpi_accounts);
    token_interface::burn(cpi_ctx, amount)?;

    emit_cpi!(Burned {
        caller: ctx.accounts.burner.key(),
        mint: ctx.accounts.mint.key(),
        source: ctx.accounts.source.key(),
        amount,
    });

    Ok(())
}
