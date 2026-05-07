use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::GatekeeperError;
use crate::events::GatekeeperInitialized;
use crate::state::*;

#[derive(Accounts)]
#[event_cpi]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + GatekeeperConfig::INIT_SPACE,
        seeds = [GATEKEEPER_CONFIG_SEED],
        bump,
    )]
    pub gatekeeper_config: Account<'info, GatekeeperConfig>,

    #[account(
        init,
        payer = admin,
        space = 8 + VaultAuthority::INIT_SPACE,
        seeds = [VAULT_SEED],
        bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        bump = permission_manager_config.bump,
        seeds::program = permission_manager_program_id(),
        constraint = permission_manager_config.admin == admin.key() @ GatekeeperError::Unauthorized,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(
    ctx: Context<Initialize>,
    permission_manager: Pubkey,
    max_delay: i64,
) -> Result<()> {
    require!(max_delay > 0, GatekeeperError::InvalidMaxDelay);

    ctx.accounts.gatekeeper_config.set_inner(GatekeeperConfig {
        max_delay,
        permission_manager,
        bump: ctx.bumps.gatekeeper_config,
    });

    let vault_authority = &mut ctx.accounts.vault_authority;
    vault_authority.bump = ctx.bumps.vault_authority;

    emit_cpi!(GatekeeperInitialized {
        admin: ctx.accounts.admin.key(),
    });

    Ok(())
}
