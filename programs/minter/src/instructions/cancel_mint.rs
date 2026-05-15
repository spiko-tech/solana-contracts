use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::{MintOperation, MinterConfig};

#[derive(Accounts)]
#[instruction(salt: u64)]
pub struct CancelMint<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.admin == admin.key() @ MinterError::Unauthorized,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    #[account(
        mut,
        seeds = [MINT_OPERATION_SEED, salt.to_le_bytes().as_ref()],
        bump = mint_operation.bump,
        constraint = mint_operation.status == STATUS_PENDING @ MinterError::Unauthorized,
    )]
    pub mint_operation: Account<'info, MintOperation>,
}

pub(crate) fn handler(ctx: Context<CancelMint>, _salt: u64) -> Result<()> {
    ctx.accounts.mint_operation.status = STATUS_CANCELED;
    Ok(())
}
