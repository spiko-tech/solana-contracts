use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::HookConfig;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + HookConfig::INIT_SPACE,
        seeds = [HOOK_CONFIG_SEED],
        bump,
    )]
    pub hook_config: Account<'info, HookConfig>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Initialize>, whitelist_authority: Pubkey) -> Result<()> {
    ctx.accounts.hook_config.set_inner(HookConfig {
        whitelist_authority,
        admin: ctx.accounts.admin.key(),
        bump: ctx.bumps.hook_config,
    });
    Ok(())
}
