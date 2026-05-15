use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::GatekeeperError;
use crate::state::{GatekeepOperation, GatekeeperConfig, VaultAuthority, WithdrawalDailyLimit};
use crate::utils::invoke_transfer_checked_with_hook;

#[derive(Accounts)]
#[instruction(salt: u64, sender: Pubkey, destination: Pubkey, amount: u64)]
pub struct InitiateGatekeep<'info> {
    pub gatekeeper_initiator: Signer<'info>,

    #[account(
        seeds = [GATEKEEPER_CONFIG_SEED],
        bump = gatekeeper_config.bump,
        constraint = gatekeeper_config.gatekeeper_initiator == gatekeeper_initiator.key() @ GatekeeperError::Unauthorized,
    )]
    pub gatekeeper_config: Account<'info, GatekeeperConfig>,

    #[account(
        init,
        payer = payer,
        space = 8 + GatekeepOperation::INIT_SPACE,
        seeds = [GATEKEEP_OPERATION_SEED, salt.to_le_bytes().as_ref()],
        bump,
    )]
    pub gatekeep_operation: Account<'info, GatekeepOperation>,

    #[account(
        mut,
        seeds = [WITHDRAWAL_DAILY_LIMIT_SEED, mint.key().as_ref()],
        bump = withdrawal_daily_limit.bump,
    )]
    pub withdrawal_daily_limit: Account<'info, WithdrawalDailyLimit>,

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
        token::authority = destination,
    )]
    pub destination_token_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler<'info>(
    ctx: Context<'info, InitiateGatekeep<'info>>,
    _salt: u64,
    sender: Pubkey,
    destination: Pubkey,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, GatekeeperError::InvalidAmount);

    let clock = Clock::get()?;
    let current_day = clock
        .unix_timestamp
        .checked_div(SECONDS_PER_DAY)
        .ok_or(GatekeeperError::ArithmeticOverflow)?;

    let daily_limit = &mut ctx.accounts.withdrawal_daily_limit;
    daily_limit.reset_if_new_day(current_day);

    if daily_limit.is_within_limit(amount)? {
        let seeds: &[&[u8]] = &[VAULT_AUTHORITY_SEED, &[ctx.accounts.vault_authority.bump]];
        let signer_seeds = &[&seeds[..]];

        invoke_transfer_checked_with_hook(
            ctx.accounts.token_program.key,
            &ctx.accounts.vault.to_account_info(),
            &ctx.accounts.mint.to_account_info(),
            &ctx.accounts.destination_token_account.to_account_info(),
            &ctx.accounts.vault_authority.to_account_info(),
            ctx.remaining_accounts,
            amount,
            ctx.accounts.mint.decimals,
            signer_seeds,
        )?;

        daily_limit.record_usage(amount)?;

        ctx.accounts
            .gatekeep_operation
            .set_inner(GatekeepOperation {
                status: STATUS_DONE,
                sender,
                destination,
                mint: ctx.accounts.mint.key(),
                amount,
                bump: ctx.bumps.gatekeep_operation,
            });
    } else {
        ctx.accounts
            .gatekeep_operation
            .set_inner(GatekeepOperation {
                status: STATUS_PENDING,
                sender,
                destination,
                mint: ctx.accounts.mint.key(),
                amount,
                bump: ctx.bumps.gatekeep_operation,
            });
    }

    Ok(())
}
