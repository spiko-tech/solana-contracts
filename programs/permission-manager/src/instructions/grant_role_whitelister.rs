use anchor_lang::prelude::*;

use crate::constants::{
    CONFIG_SEED, ROLE_WHITELISTED, ROLE_WHITELISTED_EXT, ROLE_WHITELISTER, USER_PERMISSION_SEED,
};
use crate::errors::PermissionError;
use crate::events::RoleGranted;
use crate::state::{PermissionConfig, UserPermissions};

#[derive(Accounts)]
#[event_cpi]
pub struct GrantRoleWhitelister<'info> {
    pub caller: Signer<'info>,

    #[account(
        seeds = [CONFIG_SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, PermissionConfig>,

    #[account(
        seeds = [USER_PERMISSION_SEED, caller.key().as_ref(), config.key().as_ref()],
        bump = caller_permissions.bump,
        constraint = caller_permissions.roles & ROLE_WHITELISTER != 0 @ PermissionError::Unauthorized,
    )]
    pub caller_permissions: Account<'info, UserPermissions>,

    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + UserPermissions::INIT_SPACE,
        seeds = [USER_PERMISSION_SEED, user.key().as_ref(), config.key().as_ref()],
        bump,
    )]
    pub user_permissions: Account<'info, UserPermissions>,

    /// CHECK: Target user whose permissions are being modified
    pub user: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<GrantRoleWhitelister>, role: u16) -> Result<()> {
    require!(
        role == ROLE_WHITELISTED || role == ROLE_WHITELISTED_EXT,
        PermissionError::RoleNotWhitelistManageable
    );

    require!(
        ctx.accounts.user_permissions.can_add_whitelist_role(role),
        PermissionError::MutualExclusionViolation
    );

    ctx.accounts.user_permissions.roles |= role;
    ctx.accounts.user_permissions.bump = ctx.bumps.user_permissions;

    emit_cpi!(RoleGranted {
        caller: ctx.accounts.caller.key(),
        user: ctx.accounts.user.key(),
        role,
    });

    Ok(())
}
