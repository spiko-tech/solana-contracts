use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

declare_id!("7DXckwPHM1ktduwLXWxsn87hWrmyUVKDNNst5ycAj8VU");

#[program]
pub mod spiko_transfer_hook {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    #[instruction(discriminator = [105, 37, 101, 197, 75, 251, 102, 26])]
    pub fn execute(ctx: Context<Execute>, amount: u64) -> Result<()> {
        instructions::transfer_hook::handler(ctx, amount)
    }

    pub fn pause_hook(ctx: Context<PauseHook>) -> Result<()> {
        instructions::pause_hook::handler(ctx)
    }

    pub fn unpause_hook(ctx: Context<UnpauseHook>) -> Result<()> {
        instructions::unpause_hook::handler(ctx)
    }
}
