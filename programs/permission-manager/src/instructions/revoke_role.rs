use anchor_lang::prelude::*;

use crate::constants::{CONFIG_SEED, USER_PERMISSION_SEED};
use crate::errors::PermissionError;
use crate::events::RoleRevoked;
use crate::state::{PermissionConfig, UserPermissions};

#[derive(Accounts)]
#[event_cpi]
pub struct RevokeRole<'info> {
    pub admin: Signer<'info>,
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = config.admin == admin.key() @ PermissionError::Unauthorized,
    )]
    pub config: Account<'info, PermissionConfig>,
    #[account(
        mut,
        seeds = [USER_PERMISSION_SEED, user.key().as_ref(), config.key().as_ref()],
        bump = user_permissions.bump,
    )]
    pub user_permissions: Account<'info, UserPermissions>,
    /// CHECK: Target user whose permissions are being modified
    pub user: UncheckedAccount<'info>,
}

pub(crate) fn handler(ctx: Context<RevokeRole>, role: u16) -> Result<()> {
    ctx.accounts.user_permissions.roles &= !role;

    emit_cpi!(RoleRevoked {
        admin: ctx.accounts.admin.key(),
        user: ctx.accounts.user.key(),
        role,
    });

    Ok(())
}
