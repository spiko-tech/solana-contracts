use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::TransferHookError;
use crate::state::{HookConfig, WhitelistState};

#[derive(Accounts)]
pub struct AddGate<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [HOOK_CONFIG_SEED],
        bump = hook_config.bump,
        constraint = hook_config.admin == admin.key() @ TransferHookError::Unauthorized,
    )]
    pub hook_config: Account<'info, HookConfig>,

    /// CHECK: The wallet to add gate role.
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

pub(crate) fn handler(ctx: Context<AddGate>) -> Result<()> {
    let current_role = ctx.accounts.whitelist_state.role;
    require!(
        current_role == 0 || current_role == ROLE_WHITELISTED_GATE,
        TransferHookError::Unauthorized
    );

    ctx.accounts.whitelist_state.role = ROLE_WHITELISTED_GATE;
    ctx.accounts.whitelist_state.bump = ctx.bumps.whitelist_state;
    Ok(())
}
