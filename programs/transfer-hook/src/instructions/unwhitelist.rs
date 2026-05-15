use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::TransferHookError;
use crate::state::{HookConfig, WhitelistState};

#[derive(Accounts)]
pub struct Unwhitelist<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [HOOK_CONFIG_SEED],
        bump = hook_config.bump,
        constraint = hook_config.whitelist_authority == authority.key() @ TransferHookError::Unauthorized,
    )]
    pub hook_config: Account<'info, HookConfig>,

    /// CHECK: The wallet to unwhitelist.
    pub wallet: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [WHITELIST_SEED, wallet.key().as_ref()],
        bump = whitelist_state.bump,
    )]
    pub whitelist_state: Account<'info, WhitelistState>,
}

pub(crate) fn handler(ctx: Context<Unwhitelist>) -> Result<()> {
    let current_role = ctx.accounts.whitelist_state.role;

    ctx.accounts.whitelist_state.role = match current_role {
        ROLE_WHITELISTED => ROLE_UNWHITELISTED,
        ROLE_WHITELISTED_EXT => ROLE_UNWHITELISTED_EXT,
        _ => return Err(TransferHookError::Unauthorized.into()),
    };

    Ok(())
}
