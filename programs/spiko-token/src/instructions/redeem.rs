use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::SpTokenError;
use crate::state::*;
use crate::utils::{compute_operation_id, invoke_transfer_checked_with_remaining};

#[derive(Accounts)]
pub struct Redeem<'info> {
    #[account(mut)]
    pub redeemer: Signer<'info>,

    #[account(
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump,
        constraint = !token_config.paused @ SpTokenError::TokenPaused,
    )]
    pub token_config: Account<'info, TokenConfig>,

    #[account(
        mut,
        address = token_config.mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program,
    )]
    pub source: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mint,
        token::token_program = token_program,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        owner = permission_manager_program_id(),
        constraint = has_role(&redeemer_permissions, ROLE_WHITELISTED) @ SpTokenError::Unauthorized,
    )]
    pub redeemer_permissions: Account<'info, UserPermissions>,

    /// CHECK: Permission manager config
    pub permission_manager_config: UncheckedAccount<'info>,

    #[account(
        seeds = [MINT_AUTHORITY_SEED, mint.key().as_ref()],
        bump = mint_authority.bump,
    )]
    pub mint_authority: Account<'info, MintAuthority>,

    pub token_program: Interface<'info, TokenInterface>,

    // --- Redemption CPI accounts ---
    pub redemption_program: Program<'info, redemption::program::Redemption>,

    /// CHECK: Vault authority PDA from redemption program, validated by CPI.
    pub redemption_vault_authority: UncheckedAccount<'info>,

    /// CHECK: Redemption config PDA from redemption program, validated by CPI.
    pub redemption_config: UncheckedAccount<'info>,

    /// CHECK: Redemption operation PDA, initialized by CPI.
    #[account(mut)]
    pub redemption_operation: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler<'info>(
    ctx: Context<'_, '_, 'info, 'info, Redeem<'info>>,
    amount: u64,
    salt: u64,
) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();
    let decimals = ctx.accounts.mint.decimals;
    let seeds = &[
        MINT_AUTHORITY_SEED,
        mint_key.as_ref(),
        &[ctx.accounts.mint_authority.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    invoke_transfer_checked_with_remaining(
        ctx.accounts.token_program.key,
        &ctx.accounts.source.to_account_info(),
        &ctx.accounts.mint.to_account_info(),
        &ctx.accounts.vault.to_account_info(),
        &ctx.accounts.mint_authority.to_account_info(),
        amount,
        decimals,
        signer_seeds,
        ctx.remaining_accounts,
    )?;

    let operation_id = compute_operation_id(&ctx.accounts.redeemer.key(), &mint_key, amount, salt);

    let cpi_accounts = redemption::cpi::accounts::OnRedeem {
        user: ctx.accounts.redeemer.to_account_info(),
        mint_authority: ctx.accounts.mint_authority.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        vault_authority: ctx.accounts.redemption_vault_authority.to_account_info(),
        redemption_config: ctx.accounts.redemption_config.to_account_info(),
        redemption_operation: ctx.accounts.redemption_operation.to_account_info(),
        payer: ctx.accounts.redeemer.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.redemption_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    redemption::cpi::on_redeem(cpi_ctx, operation_id, amount, salt)?;

    Ok(())
}
