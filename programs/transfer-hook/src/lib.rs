use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("Fzyd28cVXwzaqoU9bqU8hLpcYqhQQtSyJVaGCgcGFEjq");

#[program]
pub mod transfer_hook {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, whitelist_authority: Pubkey) -> Result<()> {
        instructions::initialize::handler(ctx, whitelist_authority)
    }

    pub fn register_mint(ctx: Context<RegisterMint>) -> Result<()> {
        instructions::register_mint::handler(ctx)
    }

    #[instruction(discriminator = [105, 37, 101, 197, 75, 251, 102, 26])]
    pub fn execute(ctx: Context<Execute>, amount: u64) -> Result<()> {
        instructions::transfer_hook::handler(ctx, amount)
    }

    pub fn whitelist(ctx: Context<Whitelist>, ext: bool) -> Result<()> {
        instructions::whitelist::handler(ctx, ext)
    }

    pub fn unwhitelist(ctx: Context<Unwhitelist>) -> Result<()> {
        instructions::unwhitelist::handler(ctx)
    }

    pub fn add_gate(ctx: Context<AddGate>) -> Result<()> {
        instructions::add_gate::handler(ctx)
    }

    pub fn remove_gate(ctx: Context<RemoveGate>) -> Result<()> {
        instructions::remove_gate::handler(ctx)
    }

    pub fn set_whitelist_authority(
        ctx: Context<SetWhitelistAuthority>,
        new_authority: Pubkey,
    ) -> Result<()> {
        instructions::set_whitelist_authority::handler(ctx, new_authority)
    }

    pub fn set_admin(ctx: Context<SetAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::set_admin::handler(ctx, new_admin)
    }
}
