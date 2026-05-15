use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::RedemptionError;
use crate::state::{RedemptionConfig, RedemptionRecord, VaultAuthority};

#[derive(Accounts)]
#[instruction(salt: u64)]
pub struct Redeem<'info> {
    pub authority: Signer<'info>,

    #[account(
        seeds = [REDEMPTION_CONFIG_SEED],
        bump = redemption_config.bump,
        constraint = redemption_config.redemption_authority == authority.key() @ RedemptionError::Unauthorized,
    )]
    pub redemption_config: Account<'info, RedemptionConfig>,

    #[account(mut)]
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

pub(crate) fn handler(ctx: Context<Redeem>, _salt: u64, amount: u64) -> Result<()> {
    require!(amount > 0, RedemptionError::Unauthorized);

    let seeds: &[&[u8]] = &[VAULT_AUTHORITY_SEED, &[ctx.accounts.vault_authority.bump]];
    let signer_seeds = &[seeds];

    let cpi_accounts = Burn {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };
    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, signer_seeds);
    token_interface::burn(cpi_ctx, amount)?;

    ctx.accounts.redemption_record.bump = ctx.bumps.redemption_record;

    Ok(())
}
