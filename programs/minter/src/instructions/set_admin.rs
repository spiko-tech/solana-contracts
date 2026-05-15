use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::MinterConfig;

#[derive(Accounts)]
pub struct SetAdmin<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.admin == admin.key() @ MinterError::Unauthorized,
    )]
    pub minter_config: Account<'info, MinterConfig>,
}

pub(crate) fn handler(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
    ctx.accounts.minter_config.admin = new_admin;
    Ok(())
}
