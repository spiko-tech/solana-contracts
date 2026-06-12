use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::{MinterConfig, PendingMintAuthorityTransfer};

#[derive(Accounts)]
pub struct CancelMintAuthorityNomination<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.admin == admin.key() @ MinterError::Unauthorized,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        close = admin,
        seeds = [PENDING_MINT_AUTHORITY_SEED, mint.key().as_ref()],
        bump = pending_transfer.bump,
        constraint = pending_transfer.mint == mint.key() @ MinterError::Unauthorized,
    )]
    pub pending_transfer: Account<'info, PendingMintAuthorityTransfer>,
}

pub(crate) fn handler(_ctx: Context<CancelMintAuthorityNomination>) -> Result<()> {
    Ok(())
}
