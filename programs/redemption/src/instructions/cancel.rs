use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::RedemptionError;
use crate::state::{RedemptionConfig, RedemptionRecord, VaultAuthority};
use crate::utils::invoke_transfer_checked_with_hook;

#[derive(Accounts)]
#[instruction(salt: u64, amount: u64, user: Pubkey)]
pub struct Cancel<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [REDEMPTION_CONFIG_SEED],
        bump = redemption_config.bump,
        constraint = redemption_config.admin == admin.key() @ RedemptionError::Unauthorized,
    )]
    pub redemption_config: Account<'info, RedemptionConfig>,

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
        token::authority = user,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        space = 8 + RedemptionRecord::INIT_SPACE,
        seeds = [REDEMPTION_RECORD_SEED, salt.to_le_bytes().as_ref()],
        bump,
    )]
    pub redemption_record: Account<'info, RedemptionRecord>,

    pub token_program: Interface<'info, TokenInterface>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler<'info>(
    ctx: Context<'info, Cancel<'info>>,
    _salt: u64,
    amount: u64,
    _user: Pubkey,
) -> Result<()> {
    require!(amount > 0, RedemptionError::Unauthorized);

    let seeds: &[&[u8]] = &[VAULT_AUTHORITY_SEED, &[ctx.accounts.vault_authority.bump]];
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

    ctx.accounts.redemption_record.bump = ctx.bumps.redemption_record;

    Ok(())
}
