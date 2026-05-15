use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::GatekeeperError;
use crate::state::GatekeeperConfig;

#[derive(Accounts)]
pub struct SetGatekeeperInitiator<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [GATEKEEPER_CONFIG_SEED],
        bump = gatekeeper_config.bump,
        constraint = gatekeeper_config.admin == admin.key() @ GatekeeperError::Unauthorized,
    )]
    pub gatekeeper_config: Account<'info, GatekeeperConfig>,
}

pub(crate) fn handler(ctx: Context<SetGatekeeperInitiator>, new_initiator: Pubkey) -> Result<()> {
    ctx.accounts.gatekeeper_config.gatekeeper_initiator = new_initiator;
    Ok(())
}
