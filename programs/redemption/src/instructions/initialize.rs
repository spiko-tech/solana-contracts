use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::RedemptionError;
use crate::events::RedemptionInitialized;
use crate::state::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + RedemptionConfig::INIT_SPACE,
        seeds = [REDEMPTION_CONFIG_SEED],
        bump,
    )]
    pub redemption_config: Account<'info, RedemptionConfig>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        bump,
        seeds::program = permission_manager_program_id(),
        constraint = permission_manager_config.admin == admin.key() @ RedemptionError::Unauthorized,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Initialize>, deadline_delay: i64) -> Result<()> {
    ctx.accounts.redemption_config.set_inner(RedemptionConfig {
        deadline_delay,
        permission_manager: ctx.accounts.permission_manager_config.key(),
    });

    emit!(RedemptionInitialized {
        admin: ctx.accounts.admin.key(),
        deadline_delay,
    });

    Ok(())
}
