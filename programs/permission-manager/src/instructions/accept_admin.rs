use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::errors::PermissionError;
use crate::events::AdminTransferAccepted;
use crate::state::PermissionConfig;

#[derive(Accounts)]
#[event_cpi]
pub struct AcceptAdmin<'info> {
    pub new_admin: Signer<'info>,
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, PermissionConfig>,
}

pub(crate) fn handler(ctx: Context<AcceptAdmin>) -> Result<()> {
    require!(
        ctx.accounts.new_admin.key() == ctx.accounts.config.pending_admin,
        PermissionError::Unauthorized
    );

    let old_admin = ctx.accounts.config.admin;
    ctx.accounts.config.admin = ctx.accounts.new_admin.key();
    ctx.accounts.config.pending_admin = Pubkey::default();

    emit_cpi!(AdminTransferAccepted {
        old_admin,
        new_admin: ctx.accounts.new_admin.key(),
    });

    Ok(())
}
