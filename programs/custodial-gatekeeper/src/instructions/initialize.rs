use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::{GatekeeperConfig, VaultAuthority};

#[derive(Accounts)]
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
        seeds = [VAULT_AUTHORITY_SEED],
        bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Initialize>, gatekeeper_initiator: Pubkey) -> Result<()> {
    ctx.accounts.gatekeeper_config.set_inner(GatekeeperConfig {
        admin: ctx.accounts.admin.key(),
        gatekeeper_initiator,
        bump: ctx.bumps.gatekeeper_config,
    });
    ctx.accounts.vault_authority.bump = ctx.bumps.vault_authority;
    Ok(())
}
