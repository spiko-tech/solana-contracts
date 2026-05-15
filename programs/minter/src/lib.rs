use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("Hygpx48FpJyDjW1uW8fykwb94Jmak4CaWvihRREsJyFX");

#[program]
pub mod minter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, mint_initiator: Pubkey) -> Result<()> {
        instructions::initialize::handler(ctx, mint_initiator)
    }

    pub fn set_daily_limit(ctx: Context<SetDailyLimit>, limit: u64) -> Result<()> {
        instructions::set_daily_limit::handler(ctx, limit)
    }

    pub fn initiate_mint(
        ctx: Context<InitiateMint>,
        salt: u64,
        recipient: Pubkey,
        amount: u64,
    ) -> Result<()> {
        instructions::initiate_mint::handler(ctx, salt, recipient, amount)
    }

    pub fn approve_mint(ctx: Context<ApproveMint>, salt: u64) -> Result<()> {
        instructions::approve_mint::handler(ctx, salt)
    }

    pub fn cancel_mint(ctx: Context<CancelMint>, salt: u64) -> Result<()> {
        instructions::cancel_mint::handler(ctx, salt)
    }

    pub fn set_admin(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::set_admin::handler(ctx, new_admin)
    }

    pub fn set_mint_initiator(ctx: Context<SetMintInitiator>, new_initiator: Pubkey) -> Result<()> {
        instructions::set_mint_initiator::handler(ctx, new_initiator)
    }
}
