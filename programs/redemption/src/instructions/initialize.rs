use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::{RedemptionConfig, VaultAuthority};

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
        init,
        payer = admin,
        space = 8 + VaultAuthority::INIT_SPACE,
        seeds = [VAULT_AUTHORITY_SEED],
        bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Initialize>, redemption_authority: Pubkey) -> Result<()> {
    ctx.accounts.redemption_config.set_inner(RedemptionConfig {
        admin: ctx.accounts.admin.key(),
        redemption_authority,
        bump: ctx.bumps.redemption_config,
    });
    ctx.accounts.vault_authority.bump = ctx.bumps.vault_authority;
    Ok(())
}
