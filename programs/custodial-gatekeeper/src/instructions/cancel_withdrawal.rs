use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::GatekeeperError;
use crate::events::WithdrawalCanceled;
use crate::state::*;
use crate::utils::invoke_transfer_checked_with_hook;

#[derive(Accounts)]
#[instruction(operation_id: [u8; 32], recipient: Pubkey, amount: u64, salt: u64)]
pub struct CancelWithdrawal<'info> {
    pub caller: Signer<'info>,

    #[account(
        seeds = [GATEKEEPER_CONFIG_SEED],
        bump,
    )]
    pub gatekeeper_config: Account<'info, GatekeeperConfig>,

    #[account(
        mut,
        seeds = [WITHDRAWAL_OPERATION_SEED, operation_id.as_ref()],
        bump,
        constraint = withdrawal_operation.status == STATUS_PENDING @ GatekeeperError::NotPending,
    )]
    pub withdrawal_operation: Account<'info, WithdrawalOperation>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = vault_authority,
    )]
    pub vault_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [VAULT_SEED],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    // --- Permission accounts ---
    #[account(
        owner = permission_manager_program_id(),
    )]
    pub vault_authority_permissions: Account<'info, UserPermissions>,

    #[account(
        owner = permission_manager_program_id(),
    )]
    pub sender_permissions: Account<'info, UserPermissions>,

    /// CHECK: Permission manager config PDA.
    pub permission_manager_config: UncheckedAccount<'info>,

    /// CHECK: Permission manager program (needed by transfer hook resolution).
    #[account(address = permission_manager_program_id())]
    pub permission_manager_program: UncheckedAccount<'info>,

    // --- Transfer hook accounts ---
    /// CHECK: ExtraAccountMetaList PDA: seeds = [b"extra-account-metas", mint] on hook program.
    pub extra_account_metas_list: UncheckedAccount<'info>,

    /// CHECK: Hook config PDA: seeds = [b"hook_config", mint] on hook program.
    pub hook_config: UncheckedAccount<'info>,

    /// CHECK: The spiko-transfer-hook program.
    #[account(address = transfer_hook_program_id())]
    pub transfer_hook_program: UncheckedAccount<'info>,

    // --- Standard programs ---
    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler(
    ctx: Context<CancelWithdrawal>,
    operation_id: [u8; 32],
    recipient: Pubkey,
    amount: u64,
    salt: u64,
) -> Result<()> {
    ctx.accounts
        .withdrawal_operation
        .validate_for_cancellation(&operation_id, &ctx.accounts.mint.key(), amount, salt)?;

    let decimals = ctx.accounts.mint.decimals;
    let seeds = &[VAULT_SEED, &[ctx.accounts.vault_authority.bump]];
    let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

    let hook_accounts = &[
        ctx.accounts.extra_account_metas_list.to_account_info(),
        ctx.accounts.hook_config.to_account_info(),
        ctx.accounts.permission_manager_program.to_account_info(),
        ctx.accounts.permission_manager_config.to_account_info(),
        ctx.accounts.vault_authority_permissions.to_account_info(),
        ctx.accounts.sender_permissions.to_account_info(),
        ctx.accounts.transfer_hook_program.to_account_info(),
    ];

    invoke_transfer_checked_with_hook(
        ctx.accounts.token_program.key,
        &ctx.accounts.vault_token_account.to_account_info(),
        &ctx.accounts.mint.to_account_info(),
        &ctx.accounts.sender_token_account.to_account_info(),
        &ctx.accounts.vault_authority.to_account_info(),
        amount,
        decimals,
        signer_seeds,
        hook_accounts,
    )?;

    ctx.accounts.withdrawal_operation.status = STATUS_CANCELED;

    emit!(WithdrawalCanceled {
        caller: ctx.accounts.caller.key(),
        recipient,
        mint: ctx.accounts.mint.key(),
        amount,
        salt,
    });

    Ok(())
}
