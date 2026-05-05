use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("G3KXsXdrTz85MjA7avs89fTHmQa4SkybRdRRNBYq5XZE");

#[program]
pub mod permission_manager {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    pub fn grant_role(ctx: Context<GrantRole>, role: u16) -> Result<()> {
        instructions::grant_role::handler(ctx, role)
    }

    pub fn grant_role_whitelister(ctx: Context<GrantRoleWhitelister>, role: u16) -> Result<()> {
        instructions::grant_role_whitelister::handler(ctx, role)
    }

    pub fn revoke_role(ctx: Context<RevokeRole>, role: u16) -> Result<()> {
        instructions::revoke_role::handler(ctx, role)
    }

    pub fn transfer_admin(ctx: Context<TransferAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::transfer_admin::handler(ctx, new_admin)
    }

    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        instructions::accept_admin::handler(ctx)
    }
}
