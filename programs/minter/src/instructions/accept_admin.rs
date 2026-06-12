use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::MinterConfig;

#[derive(Accounts)]
pub struct AcceptAdmin<'info> {
    pub new_admin: Signer<'info>,

    #[account(
        mut,
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.pending_admin == Some(new_admin.key()) @ MinterError::Unauthorized,
    )]
    pub minter_config: Account<'info, MinterConfig>,
}

pub(crate) fn handler(ctx: Context<AcceptAdmin>) -> Result<()> {
    let config = &mut ctx.accounts.minter_config;
    config.admin = ctx.accounts.new_admin.key();
    config.pending_admin = None;
    Ok(())
}
