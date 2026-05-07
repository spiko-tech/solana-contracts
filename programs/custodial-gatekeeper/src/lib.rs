use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("9z86yHHZEojd2HoGBviCKf7kWbbZJqWzRgQQm3bKCBh5");

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

    pub fn custodial_withdraw(
        ctx: Context<CustodialWithdraw>,
        operation_id: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::custodial_withdraw::handler(ctx, operation_id, recipient, amount, salt)
    }

    pub fn approve_withdrawal(
        ctx: Context<ApproveWithdrawal>,
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
