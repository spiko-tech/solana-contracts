use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::SpTokenError;
use crate::events::Unpaused;
use crate::state::*;

#[derive(Accounts)]
pub struct Unpause<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [TOKEN_CONFIG_SEED, token_config.mint.as_ref()],
        bump,
        constraint = token_config.paused @ SpTokenError::NotPaused,
    )]
    pub token_config: Account<'info, TokenConfig>,

    /// CHECK: Mint address, validated by token_config constraint.
    #[account(
        address = token_config.mint,
    )]
    pub mint: AccountInfo<'info>,

    #[account(
        seeds = [MINT_AUTHORITY_SEED, mint.key().as_ref()],
        bump = mint_authority.bump,
    )]
    pub mint_authority: Account<'info, MintAuthority>,

    /// CHECK: HookConfig PDA on transfer hook program, validated by CPI.
    #[account(mut)]
    pub hook_config: UncheckedAccount<'info>,

    pub spiko_transfer_hook_program:
        Program<'info, spiko_transfer_hook::program::SpikoTransferHook>,

    #[account(
        owner = permission_manager_program_id(),
        constraint = has_role(&admin_permissions, ROLE_PAUSER) @ SpTokenError::Unauthorized,
    )]
    pub admin_permissions: Account<'info, UserPermissions>,
}

pub(crate) fn handler(ctx: Context<Unpause>) -> Result<()> {
    ctx.accounts.token_config.paused = false;

    let mint_key = ctx.accounts.mint.key();
    let seeds = &[
        MINT_AUTHORITY_SEED,
        mint_key.as_ref(),
        &[ctx.accounts.mint_authority.bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = spiko_transfer_hook::cpi::accounts::UnpauseHook {
        caller: ctx.accounts.admin.to_account_info(),
        mint_authority: ctx.accounts.mint_authority.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        hook_config: ctx.accounts.hook_config.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.spiko_transfer_hook_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    spiko_transfer_hook::cpi::unpause_hook(cpi_ctx)?;

    emit!(Unpaused {
        caller: ctx.accounts.admin.key(),
    });

    Ok(())
}
