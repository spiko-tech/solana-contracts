use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("13jYMgAoRQHSKVT6LakgRKFiyygFTN7LYsKym9Lv84MQ");

#[program]
pub mod minter {
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

    pub fn initiate_mint(
        ctx: Context<InitiateMint>,
        operation_id: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::initiate_mint::handler(ctx, operation_id, recipient, amount, salt)
    }

    pub fn approve_mint(
        ctx: Context<ApproveMint>,
        operation_id: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::approve_mint::handler(ctx, operation_id, recipient, amount, salt)
    }

    pub fn cancel_mint(
        ctx: Context<CancelMint>,
        operation_id: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        salt: u64,
    ) -> Result<()> {
        instructions::cancel_mint::handler(ctx, operation_id, recipient, amount, salt)
    }
}
