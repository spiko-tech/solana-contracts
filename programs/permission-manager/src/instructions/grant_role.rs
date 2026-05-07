use anchor_lang::prelude::*;

use crate::constants::{CONFIG_SEED, USER_PERMISSION_SEED};
use crate::errors::PermissionError;
use crate::events::RoleGranted;
use crate::state::{PermissionConfig, UserPermissions};

#[derive(Accounts)]
#[event_cpi]
pub struct GrantRole<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
        constraint = config.admin == admin.key() @ PermissionError::Unauthorized,
    )]
    pub config: Account<'info, PermissionConfig>,
    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + UserPermissions::INIT_SPACE,
        seeds = [USER_PERMISSION_SEED, user.key().as_ref(), config.key().as_ref()],
        bump,
    )]
    pub user_permissions: Account<'info, UserPermissions>,
    /// CHECK: Target user whose permissions are being modified
    pub user: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<GrantRole>, role: u16) -> Result<()> {
    require!(
        UserPermissions::is_valid_role_mask(role),
        PermissionError::InvalidRole
    );

    ctx.accounts.user_permissions.roles |= role;
    ctx.accounts.user_permissions.bump = ctx.bumps.user_permissions;

    emit_cpi!(RoleGranted {
        caller: ctx.accounts.admin.key(),
        user: ctx.accounts.user.key(),
        role,
    });

    Ok(())
}
