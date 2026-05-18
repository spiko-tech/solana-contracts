use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::GatekeeperError;
use crate::state::{GatekeepOperation, GatekeeperConfig, VaultAuthority};
use crate::utils::invoke_transfer_checked_with_hook;

#[derive(Accounts)]
#[instruction(salt: u64)]
pub struct CancelGatekeep<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [GATEKEEPER_CONFIG_SEED],
        bump = gatekeeper_config.bump,
        constraint = gatekeeper_config.admin == admin.key() @ GatekeeperError::Unauthorized,
    )]
    pub gatekeeper_config: Account<'info, GatekeeperConfig>,

    #[account(
        mut,
        seeds = [GATEKEEP_OPERATION_SEED, salt.to_le_bytes().as_ref()],
        bump = gatekeep_operation.bump,
        constraint = gatekeep_operation.status == STATUS_PENDING @ GatekeeperError::Unauthorized,
    )]
    pub gatekeep_operation: Account<'info, GatekeepOperation>,

    #[account(
        address = gatekeep_operation.mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [VAULT_AUTHORITY_SEED],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = vault_authority,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = gatekeep_operation.sender,
    )]
    pub sender_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler<'info>(ctx: Context<'info, CancelGatekeep<'info>>, _salt: u64) -> Result<()> {
    let amount = ctx.accounts.gatekeep_operation.amount;

    let seeds: &[&[u8]] = &[VAULT_AUTHORITY_SEED, &[ctx.accounts.vault_authority.bump]];
    let signer_seeds = &[&seeds[..]];

    invoke_transfer_checked_with_hook(
        ctx.accounts.token_program.key,
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.mint.to_account_info(),
        &ctx.accounts.sender_token_account.to_account_info(),
        &ctx.accounts.vault_authority.to_account_info(),
        ctx.remaining_accounts,
        amount,
        ctx.accounts.mint.decimals,
        signer_seeds,
    )?;

    ctx.accounts.gatekeep_operation.status = STATUS_CANCELED;

    Ok(())
}
