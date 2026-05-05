use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("7raQ9TfCJkFWFDg2X2GsuPh3rso5n6jRS2WGa7enhtfg");

#[program]
pub mod custodial_gatekeeper {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        permission_manager: Pubkey,
        max_delay: i64,
    ) -> Result<()> {
        instructions::initialize::handler(ctx, permission_manager, max_delay)
    }

    pub fn set_daily_limit(ctx: Context<SetDailyLimit>, limit: u64) -> Result<()> {
        instructions::set_daily_limit::handler(ctx, limit)
    }

    pub fn custodial_withdraw<'info>(
        ctx: Context<'_, '_, 'info, 'info, CustodialWithdraw<'info>>,
        operation_id: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::custodial_withdraw::handler(ctx, operation_id, recipient, amount, salt)
    }

    pub fn approve_withdrawal<'info>(
        ctx: Context<'_, '_, 'info, 'info, ApproveWithdrawal<'info>>,
        operation_id: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::approve_withdrawal::handler(ctx, operation_id, recipient, amount, salt)
    }

    pub fn cancel_withdrawal(
        ctx: Context<CancelWithdrawal>,
        operation_id: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::cancel_withdrawal::handler(ctx, operation_id, recipient, amount, salt)
    }
}
