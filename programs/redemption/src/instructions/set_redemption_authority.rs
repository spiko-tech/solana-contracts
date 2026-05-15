use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::RedemptionError;
use crate::state::RedemptionConfig;

#[derive(Accounts)]
pub struct SetRedemptionAuthority<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [REDEMPTION_CONFIG_SEED],
        bump = redemption_config.bump,
        constraint = redemption_config.admin == admin.key() @ RedemptionError::Unauthorized,
    )]
    pub redemption_config: Account<'info, RedemptionConfig>,
}

pub(crate) fn handler(ctx: Context<SetRedemptionAuthority>, new_authority: Pubkey) -> Result<()> {
    ctx.accounts.redemption_config.redemption_authority = new_authority;
    Ok(())
}
