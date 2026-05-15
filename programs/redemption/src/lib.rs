use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("B3ustaVazAzqwbgkxARcsL9KKKaNKT6o6FFQyo4b4EBr");

#[program]
pub mod redemption {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, redemption_authority: Pubkey) -> Result<()> {
        instructions::initialize::handler(ctx, redemption_authority)
    }

    pub fn redeem(ctx: Context<Redeem>, salt: u64, amount: u64) -> Result<()> {
        instructions::redeem::handler(ctx, salt, amount)
    }

    pub fn cancel<'info>(
        ctx: Context<'info, Cancel<'info>>,
        salt: u64,
        amount: u64,
        user: Pubkey,
    ) -> Result<()> {
        instructions::cancel::handler(ctx, salt, amount, user)
    }

    pub fn set_admin(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::set_admin::handler(ctx, new_admin)
    }

    pub fn set_redemption_authority(
        ctx: Context<SetRedemptionAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        instructions::set_redemption_authority::handler(ctx, new_authority)
    }
}
