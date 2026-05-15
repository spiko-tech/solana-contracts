use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::TransferHookError;
use crate::state::HookConfig;

#[derive(Accounts)]
pub struct SetWhitelistAuthority<'info> {
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [HOOK_CONFIG_SEED],
        bump = hook_config.bump,
        constraint = hook_config.whitelist_authority == authority.key() @ TransferHookError::Unauthorized,
    )]
    pub hook_config: Account<'info, HookConfig>,
}

pub(crate) fn handler(ctx: Context<SetWhitelistAuthority>, new_authority: Pubkey) -> Result<()> {
    ctx.accounts.hook_config.whitelist_authority = new_authority;
    Ok(())
}
