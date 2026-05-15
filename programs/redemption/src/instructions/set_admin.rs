use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::RedemptionError;
use crate::state::RedemptionConfig;

#[derive(Accounts)]
pub struct SetAdmin<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [REDEMPTION_CONFIG_SEED],
        bump = redemption_config.bump,
        constraint = redemption_config.admin == admin.key() @ RedemptionError::Unauthorized,
    )]
    pub redemption_config: Account<'info, RedemptionConfig>,
}

pub(crate) fn handler(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
    ctx.accounts.redemption_config.admin = new_admin;
    Ok(())
}
