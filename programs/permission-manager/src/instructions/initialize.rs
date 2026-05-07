use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::events::ConfigInitialized;
use crate::state::PermissionConfig;

#[derive(Accounts)]
#[event_cpi]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + PermissionConfig::INIT_SPACE,
        seeds = [CONFIG_SEED],
        bump,
    )]
    pub config: Account<'info, PermissionConfig>,
    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Initialize>) -> Result<()> {
    ctx.accounts.config.set_inner(PermissionConfig {
        admin: ctx.accounts.admin.key(),
        pending_admin: Pubkey::default(),
        bump: ctx.bumps.config,
    });

    emit_cpi!(ConfigInitialized {
        admin: ctx.accounts.admin.key(),
    });

    Ok(())
}
