use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::MinterConfig;

#[derive(Accounts)]
pub struct SetMintInitiator<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.admin == admin.key() @ MinterError::Unauthorized,
    )]
    pub minter_config: Account<'info, MinterConfig>,
}

pub(crate) fn handler(ctx: Context<SetMintInitiator>, new_initiator: Pubkey) -> Result<()> {
    ctx.accounts.minter_config.mint_initiator = new_initiator;
    Ok(())
}
