use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::TransferHookError;
use crate::state::{HookConfig, WhitelistState};

#[derive(Accounts)]
pub struct RemoveGate<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [HOOK_CONFIG_SEED],
        bump = hook_config.bump,
        constraint = hook_config.admin == admin.key() @ TransferHookError::Unauthorized,
    )]
    pub hook_config: Account<'info, HookConfig>,

    /// CHECK: The wallet to remove gate role.
    pub wallet: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [WHITELIST_SEED, wallet.key().as_ref()],
        bump = whitelist_state.bump,
        close = payer,
    )]
    pub whitelist_state: Account<'info, WhitelistState>,

    #[account(mut)]
    pub payer: Signer<'info>,
}

pub(crate) fn handler(_ctx: Context<RemoveGate>) -> Result<()> {
    Ok(())
}
