use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::events::RedemptionCanceled;
use crate::state::*;
use crate::utils::invoke_transfer_checked_with_hook;

#[derive(Accounts)]
#[instruction(operation_id: [u8; 32])]
#[event_cpi]
pub struct Cancel<'info> {
    pub caller: Signer<'info>,

    #[account(address = redemption_operation.mint)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [REDEMPTION_CONFIG_SEED],
        bump = redemption_config.bump,
    )]
    pub redemption_config: Account<'info, RedemptionConfig>,

    #[account(
        mut,
        seeds = [REDEMPTION_OPERATION_SEED, operation_id.as_ref()],
        bump = redemption_operation.bump,
    )]
    pub redemption_operation: Account<'info, RedemptionOperation>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = vault_authority,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = redemption_operation.user,
        token::token_program = token_program,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [VAULT_AUTHORITY_SEED, mint.key().as_ref()],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler<'info>(
    ctx: Context<'info, Cancel<'info>>,
    operation_id: [u8; 32],
    amount: u64,
    salt: u64,
) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();

    ctx.accounts
        .redemption_operation
        .validate_for_cancellation(&operation_id, &mint_key, amount, salt)?;

    let seeds: &[&[u8]] = &[
        VAULT_AUTHORITY_SEED,
        mint_key.as_ref(),
        &[ctx.accounts.vault_authority.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    invoke_transfer_checked_with_hook(
        ctx.accounts.token_program.key,
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.mint.to_account_info(),
        &ctx.accounts.user_token_account.to_account_info(),
        &ctx.accounts.vault_authority.to_account_info(),
        ctx.remaining_accounts,
        amount,
        ctx.accounts.mint.decimals,
        signer_seeds,
    )?;

    let op = &mut ctx.accounts.redemption_operation;
    op.status = STATUS_CANCELED;

    emit_cpi!(RedemptionCanceled {
        caller: ctx.accounts.caller.key(),
        user: op.user,
        mint: mint_key,
        amount,
        salt,
    });

    Ok(())
}
