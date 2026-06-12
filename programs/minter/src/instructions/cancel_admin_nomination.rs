use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::MinterConfig;

#[derive(Accounts)]
pub struct CancelAdminNomination<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.admin == admin.key() @ MinterError::Unauthorized,
        constraint = minter_config.pending_admin.is_some() @ MinterError::Unauthorized,
    )]
    pub minter_config: Account<'info, MinterConfig>,
}

pub(crate) fn handler(ctx: Context<CancelAdminNomination>) -> Result<()> {
    ctx.accounts.minter_config.pending_admin = None;
    Ok(())
}
