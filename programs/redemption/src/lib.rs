use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("F6P3cmm4xDxxZCF6vj3K9pbY2LFjVrYpEft6x6CXJxmu");

#[program]
pub mod redemption {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, deadline_delay: i64) -> Result<()> {
        instructions::initialize::handler(ctx, deadline_delay)
    }

    pub fn create_vault(ctx: Context<CreateVault>) -> Result<()> {
        instructions::create_vault::handler(ctx)
    }

    pub fn on_redeem(
        ctx: Context<OnRedeem>,
        operation_id: [u8; 32],
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::on_redeem::handler(ctx, operation_id, amount, salt)
    }

    pub fn execute(
        ctx: Context<Execute>,
        operation_id: [u8; 32],
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::execute::handler(ctx, operation_id, amount, salt)
    }

    pub fn cancel<'info>(
        ctx: Context<'_, '_, 'info, 'info, Cancel<'info>>,
        operation_id: [u8; 32],
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::cancel::handler(ctx, operation_id, amount, salt)
    }
}
