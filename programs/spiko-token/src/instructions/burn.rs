use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::SpTokenError;
use crate::events::Burned;
use crate::state::*;

#[derive(Accounts)]
pub struct BurnTokens<'info> {
    pub burner: Signer<'info>,

    #[account(
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump,
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
        constraint = has_role(&burner_permissions, ROLE_BURNER) @ SpTokenError::Unauthorized,
    )]
    pub burner_permissions: Account<'info, UserPermissions>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler(ctx: Context<BurnTokens>, amount: u64) -> Result<()> {
    let cpi_accounts = Burn {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.source.to_account_info(),
        authority: ctx.accounts.burner.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_interface::burn(cpi_ctx, amount)?;

    emit!(Burned {
        caller: ctx.accounts.burner.key(),
        mint: ctx.accounts.mint.key(),
        source: ctx.accounts.source.key(),
        amount,
    });

    Ok(())
}
