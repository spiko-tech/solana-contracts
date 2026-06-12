use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::{MinterConfig, PendingMintAuthorityTransfer};

#[derive(Accounts)]
pub struct NominateMintAuthority<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.admin == admin.key() @ MinterError::Unauthorized,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        init,
        payer = payer,
        space = 8 + PendingMintAuthorityTransfer::INIT_SPACE,
        seeds = [PENDING_MINT_AUTHORITY_SEED, mint.key().as_ref()],
        bump,
    )]
    pub pending_transfer: Account<'info, PendingMintAuthorityTransfer>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<NominateMintAuthority>, new_authority: Pubkey) -> Result<()> {
    ctx.accounts
        .pending_transfer
        .set_inner(PendingMintAuthorityTransfer {
            new_authority,
            mint: ctx.accounts.mint.key(),
            bump: ctx.bumps.pending_transfer,
        });
    Ok(())
}
