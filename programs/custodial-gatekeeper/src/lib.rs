use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("5Y7mJuJRdBFTXBrXG3rCUZTjRtNKhrRjCA3vKnVX2Zb6");

#[program]
pub mod custodial_gatekeeper {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, gatekeeper_initiator: Pubkey) -> Result<()> {
        instructions::initialize::handler(ctx, gatekeeper_initiator)
    }

    pub fn set_daily_limit(ctx: Context<SetDailyLimit>, limit: u64) -> Result<()> {
        instructions::set_daily_limit::handler(ctx, limit)
    }

    pub fn initiate_gatekeep<'info>(
        ctx: Context<'info, InitiateGatekeep<'info>>,
        salt: u64,
        sender: Pubkey,
        destination: Pubkey,
        amount: u64,
    ) -> Result<()> {
        instructions::initiate_gatekeep::handler(ctx, salt, sender, destination, amount)
    }

    pub fn approve_gatekeep<'info>(
        ctx: Context<'info, ApproveGatekeep<'info>>,
        salt: u64,
    ) -> Result<()> {
        instructions::approve_gatekeep::handler(ctx, salt)
    }

    pub fn cancel_gatekeep<'info>(
        ctx: Context<'info, CancelGatekeep<'info>>,
        salt: u64,
    ) -> Result<()> {
        instructions::cancel_gatekeep::handler(ctx, salt)
    }

    pub fn set_admin(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::set_admin::handler(ctx, new_admin)
    }

    pub fn set_gatekeeper_initiator(
        ctx: Context<SetGatekeeperInitiator>,
        new_initiator: Pubkey,
    ) -> Result<()> {
        instructions::set_gatekeeper_initiator::handler(ctx, new_initiator)
    }
}
