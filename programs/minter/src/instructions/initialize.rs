use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;
use crate::events::MinterInitialized;
use crate::state::*;

#[derive(Accounts)]
#[event_cpi]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + MinterConfig::INIT_SPACE,
        seeds = [MINTER_CONFIG_SEED],
        bump,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        bump = permission_manager_config.bump,
        seeds::program = permission_manager_program_id(),
        constraint = permission_manager_config.admin == admin.key() @ MinterError::Unauthorized,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(
    ctx: Context<Initialize>,
    permission_manager: Pubkey,
    max_delay: i64,
) -> Result<()> {
    require!(max_delay > 0, MinterError::InvalidMaxDelay);

    ctx.accounts.minter_config.set_inner(MinterConfig {
        max_delay,
        permission_manager,
        bump: ctx.bumps.minter_config,
    });

    emit_cpi!(MinterInitialized {
        admin: ctx.accounts.admin.key(),
        permission_manager,
        max_delay,
    });

    Ok(())
}
