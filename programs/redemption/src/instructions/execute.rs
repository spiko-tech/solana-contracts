use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Burn, Mint, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::RedemptionError;
use crate::events::RedemptionExecuted;
use crate::state::*;

#[derive(Accounts)]
#[instruction(operation_id: [u8; 32])]
#[event_cpi]
pub struct Execute<'info> {
    pub burner: Signer<'info>,

    #[account(
        mut,
        address = redemption_operation.mint,
    )]
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
        seeds = [VAULT_AUTHORITY_SEED, mint.key().as_ref()],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [USER_PERMISSION_SEED, burner.key().as_ref(), permission_manager_config.key().as_ref()],
        seeds::program = permission_manager_program_id(),
        bump = burner_permissions.bump,
        constraint = has_role(&burner_permissions, ROLE_REDEMPTION_EXECUTOR) @ RedemptionError::Unauthorized,
    )]
    pub burner_permissions: Account<'info, UserPermissions>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        seeds::program = permission_manager_program_id(),
        bump = permission_manager_config.bump,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler(
    ctx: Context<Execute>,
    operation_id: [u8; 32],
    amount: u64,
    salt: u64,
) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();

    ctx.accounts.redemption_operation.validate_for_execution(
        &operation_id,
        &mint_key,
        amount,
        salt,
    )?;

    let seeds: &[&[u8]] = &[
        VAULT_AUTHORITY_SEED,
        mint_key.as_ref(),
        &[ctx.accounts.vault_authority.bump],
    ];
    let signer_seeds = &[seeds];

    let cpi_accounts = Burn {
        mint: ctx.accounts.mint.to_account_info(),
        from: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.vault_authority.to_account_info(),
    };
    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, signer_seeds);
    token_interface::burn(cpi_ctx, amount)?;

    let op = &mut ctx.accounts.redemption_operation;
    op.status = STATUS_DONE;

    emit_cpi!(RedemptionExecuted {
        burner: ctx.accounts.burner.key(),
        user: op.user,
        mint: mint_key,
        amount,
        salt,
    });

    Ok(())
}
