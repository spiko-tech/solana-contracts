use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::TransferHookError;
use crate::state::{HookConfig, WhitelistState};

#[derive(Accounts)]
pub struct Whitelist<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [HOOK_CONFIG_SEED],
        bump = hook_config.bump,
        constraint = hook_config.whitelist_authority == authority.key() @ TransferHookError::Unauthorized,
    )]
    pub hook_config: Account<'info, HookConfig>,

    /// CHECK: The wallet to whitelist.
    pub wallet: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + WhitelistState::INIT_SPACE,
        seeds = [WHITELIST_SEED, wallet.key().as_ref()],
        bump,
    )]
    pub whitelist_state: Account<'info, WhitelistState>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Whitelist>, ext: bool) -> Result<()> {
    let current_role = ctx.accounts.whitelist_state.role;

    if ext {
        require!(
            current_role == 0
                || current_role == ROLE_WHITELISTED_EXT
                || current_role == ROLE_UNWHITELISTED_EXT,
            TransferHookError::Unauthorized
        );
        ctx.accounts.whitelist_state.role = ROLE_WHITELISTED_EXT;
    } else {
        require!(
            current_role == 0
                || current_role == ROLE_WHITELISTED
                || current_role == ROLE_UNWHITELISTED,
            TransferHookError::Unauthorized
        );
        ctx.accounts.whitelist_state.role = ROLE_WHITELISTED;
    }

    ctx.accounts.whitelist_state.bump = ctx.bumps.whitelist_state;
    Ok(())
}
