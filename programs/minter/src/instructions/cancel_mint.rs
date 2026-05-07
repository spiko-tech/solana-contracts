use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;
use crate::events::MintCanceled;
use crate::state::*;

#[derive(Accounts)]
#[instruction(operation_id: [u8; 32])]
#[event_cpi]
pub struct CancelMint<'info> {
    pub caller: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    #[account(
        mut,
        seeds = [MINT_OPERATION_SEED, operation_id.as_ref()],
        bump = mint_operation.bump,
        constraint = mint_operation.status == STATUS_PENDING @ MinterError::NotPending,
    )]
    pub mint_operation: Account<'info, MintOperation>,

    /// CHECK: The mint referenced in the operation
    pub mint: UncheckedAccount<'info>,
}

pub(crate) fn handler(
    ctx: Context<CancelMint>,
    operation_id: [u8; 32],
    recipient: Pubkey,
    amount: u64,
    salt: u64,
) -> Result<()> {
    ctx.accounts.mint_operation.validate_for_cancellation(
        &operation_id,
        &ctx.accounts.mint.key(),
        &recipient,
        amount,
        salt,
    )?;

    ctx.accounts.mint_operation.status = STATUS_CANCELED;

    emit_cpi!(MintCanceled {
        caller: ctx.accounts.caller.key(),
        recipient,
        mint: ctx.accounts.mint.key(),
        amount,
        salt,
    });

    Ok(())
}
