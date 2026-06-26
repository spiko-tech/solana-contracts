use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("8CVKFptWa13Z43e82tYufueoWH7tqJfsNQXB33g1WeVw");

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

    pub fn nominate_admin(ctx: Context<NominateAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::nominate_admin::handler(ctx, new_admin)
    }

    pub fn accept_admin(ctx: Context<AcceptAdmin>) -> Result<()> {
        instructions::accept_admin::handler(ctx)
    }

    pub fn cancel_admin_nomination(ctx: Context<CancelAdminNomination>) -> Result<()> {
        instructions::cancel_admin_nomination::handler(ctx)
    }

    pub fn set_mint_initiator(ctx: Context<SetMintInitiator>, new_initiator: Pubkey) -> Result<()> {
        instructions::set_mint_initiator::handler(ctx, new_initiator)
    }

    pub fn nominate_mint_authority(
        ctx: Context<NominateMintAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        instructions::nominate_mint_authority::handler(ctx, new_authority)
    }

    pub fn accept_mint_authority(ctx: Context<AcceptMintAuthority>) -> Result<()> {
        instructions::accept_mint_authority::handler(ctx)
    }

    pub fn cancel_mint_authority_nomination(
        ctx: Context<CancelMintAuthorityNomination>,
    ) -> Result<()> {
        instructions::cancel_mint_authority_nomination::handler(ctx)
    }
}
