use anchor_lang::prelude::*;

use crate::constants::*;
use crate::state::MinterConfig;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + MinterConfig::INIT_SPACE,
        seeds = [MINTER_CONFIG_SEED],
        bump,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<Initialize>, mint_initiator: Pubkey) -> Result<()> {
    ctx.accounts.minter_config.set_inner(MinterConfig {
        admin: ctx.accounts.admin.key(),
        pending_admin: None,
        mint_initiator,
        bump: ctx.bumps.minter_config,
    });
    Ok(())
}
