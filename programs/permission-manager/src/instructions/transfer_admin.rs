use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::errors::PermissionError;
use crate::events::AdminTransferRequested;
use crate::state::PermissionConfig;

#[derive(Accounts)]
pub struct TransferAdmin<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump,
        constraint = config.admin == admin.key() @ PermissionError::Unauthorized,
    )]
    pub config: Account<'info, PermissionConfig>,
}

pub(crate) fn handler(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
    ctx.accounts.config.pending_admin = new_admin;

    emit!(AdminTransferRequested {
        admin: ctx.accounts.admin.key(),
        new_admin,
    });

    Ok(())
}
