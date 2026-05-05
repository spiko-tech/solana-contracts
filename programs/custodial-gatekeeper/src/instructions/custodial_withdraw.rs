use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::GatekeeperError;
use crate::events::{WithdrawalBlocked, WithdrawalInitiated};
use crate::state::*;
use crate::utils::{invoke_transfer_checked_with_hook, verify_operation_id};

#[derive(Accounts)]
#[instruction(operation_id: [u8; 32], recipient: Pubkey, amount: u64, salt: u64)]
pub struct CustodialWithdraw<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(
        seeds = [GATEKEEPER_CONFIG_SEED],
        bump,
    )]
    pub gatekeeper_config: Account<'info, GatekeeperConfig>,

    #[account(
        mut,
        seeds = [WITHDRAWAL_DAILY_LIMIT_SEED, mint.key().as_ref()],
        bump,
    )]
    pub withdrawal_daily_limit: Account<'info, WithdrawalDailyLimit>,

    #[account(
        init,
        payer = sender,
        space = 8 + WithdrawalOperation::INIT_SPACE,
        seeds = [WITHDRAWAL_OPERATION_SEED, operation_id.as_ref()],
        bump,
    )]
    pub withdrawal_operation: Account<'info, WithdrawalOperation>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = sender,
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,

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
    pub recipient_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [VAULT_SEED],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    // --- Permission accounts ---
    #[account(
        owner = permission_manager_program_id(),
        constraint = has_role(&sender_permissions, ROLE_WHITELISTED) @ GatekeeperError::UnauthorizedSender,
    )]
    pub sender_permissions: Account<'info, UserPermissions>,

    #[account(
        owner = permission_manager_program_id(),
    )]
    pub vault_authority_permissions: Account<'info, UserPermissions>,

    #[account(
        owner = permission_manager_program_id(),
        constraint = has_role(&recipient_permissions, ROLE_WHITELISTED_EXT) @ GatekeeperError::UnauthorizedRecipient,
    )]
    pub recipient_permissions: Account<'info, UserPermissions>,

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
    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(
    ctx: Context<CustodialWithdraw>,
    operation_id: [u8; 32],
    recipient: Pubkey,
    amount: u64,
    salt: u64,
) -> Result<()> {
    verify_operation_id(
        &ctx.accounts.sender.key(),
        &ctx.accounts.mint.key(),
        amount,
        salt,
        &operation_id,
    )?;

    let clock = Clock::get()?;
    let decimals = ctx.accounts.mint.decimals;

    // Transfer 1: sender -> vault (with hook accounts)
    let hook_accounts_sender_to_vault = &[
        ctx.accounts.extra_account_metas_list.to_account_info(),
        ctx.accounts.hook_config.to_account_info(),
        ctx.accounts.permission_manager_program.to_account_info(),
        ctx.accounts.permission_manager_config.to_account_info(),
        ctx.accounts.sender_permissions.to_account_info(),
        ctx.accounts.vault_authority_permissions.to_account_info(),
        ctx.accounts.transfer_hook_program.to_account_info(),
    ];

    invoke_transfer_checked_with_hook(
        ctx.accounts.token_program.key,
        &ctx.accounts.sender_token_account.to_account_info(),
        &ctx.accounts.mint.to_account_info(),
        &ctx.accounts.vault_token_account.to_account_info(),
        &ctx.accounts.sender.to_account_info(),
        amount,
        decimals,
        &[],
        hook_accounts_sender_to_vault,
    )?;

    let current_day = clock
        .unix_timestamp
        .checked_div(SECONDS_PER_DAY)
        .ok_or(GatekeeperError::ArithmeticOverflow)?;

    let daily_limit = &mut ctx.accounts.withdrawal_daily_limit;
    daily_limit.reset_if_new_day(current_day);

    let op = &mut ctx.accounts.withdrawal_operation;

    if daily_limit.is_within_limit(amount)? {
        let seeds = &[VAULT_SEED, &[ctx.accounts.vault_authority.bump]];
        let signer_seeds: &[&[&[u8]]] = &[&seeds[..]];

        let hook_accounts_vault_to_recipient = &[
            ctx.accounts.extra_account_metas_list.to_account_info(),
            ctx.accounts.hook_config.to_account_info(),
            ctx.accounts.permission_manager_program.to_account_info(),
            ctx.accounts.permission_manager_config.to_account_info(),
            ctx.accounts.vault_authority_permissions.to_account_info(),
            ctx.accounts.recipient_permissions.to_account_info(),
            ctx.accounts.transfer_hook_program.to_account_info(),
        ];

        invoke_transfer_checked_with_hook(
            ctx.accounts.token_program.key,
            &ctx.accounts.vault_token_account.to_account_info(),
            &ctx.accounts.mint.to_account_info(),
            &ctx.accounts.recipient_token_account.to_account_info(),
            &ctx.accounts.vault_authority.to_account_info(),
            amount,
            decimals,
            signer_seeds,
            hook_accounts_vault_to_recipient,
        )?;

        daily_limit.record_usage(amount)?;

        op.set_inner(WithdrawalOperation {
            sender: ctx.accounts.sender.key(),
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            status: STATUS_DONE,
            deadline: 0,
        });

        emit!(WithdrawalInitiated {
            sender: ctx.accounts.sender.key(),
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            salt,
            deadline: 0,
        });
    } else {
        let deadline = clock
            .unix_timestamp
            .checked_add(ctx.accounts.gatekeeper_config.max_delay)
            .ok_or(GatekeeperError::ArithmeticOverflow)?;

        op.set_inner(WithdrawalOperation {
            sender: ctx.accounts.sender.key(),
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            status: STATUS_PENDING,
            deadline,
        });

        emit!(WithdrawalInitiated {
            sender: ctx.accounts.sender.key(),
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            salt,
            deadline,
        });

        emit!(WithdrawalBlocked {
            sender: ctx.accounts.sender.key(),
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            salt,
            deadline,
        });
    }

    Ok(())
}
