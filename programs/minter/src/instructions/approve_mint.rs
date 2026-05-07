use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;
use crate::events::MintApproved;
use crate::state::*;

#[derive(Accounts)]
#[instruction(operation_id: [u8; 32])]
#[event_cpi]
pub struct ApproveMint<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    #[account(
        mut,
        seeds = [MINT_OPERATION_SEED, operation_id.as_ref()],
        bump = mint_operation.bump,
        constraint = mint_operation.status == STATUS_PENDING @ MinterError::NotPending,
    )]
    pub mint_operation: Account<'info, MintOperation>,

    /// CHECK: Token-2022 mint account, validated by spiko-token program
    #[account(mut)]
    pub mint: UncheckedAccount<'info>,

    /// CHECK: Destination token account, validated by spiko-token program
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [USER_PERMISSION_SEED, admin.key().as_ref(), permission_manager_config.key().as_ref()],
        seeds::program = permission_manager_program_id(),
        bump = admin_permissions.bump,
    )]
    pub admin_permissions: Account<'info, UserPermissions>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        bump = permission_manager_config.bump,
        seeds::program = permission_manager_program_id(),
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    pub spiko_token_program: Program<'info, spiko_token::program::SpikoToken>,

    /// CHECK: Event authority PDA for the spiko-token program (seeds = [b"__event_authority"]).
    pub spiko_token_event_authority: UncheckedAccount<'info>,

    /// CHECK: Token-2022 program, validated by spiko-token program
    pub token_program: UncheckedAccount<'info>,

    /// CHECK: spiko-token TokenConfig PDA, validated by spiko-token program
    pub token_config: UncheckedAccount<'info>,

    /// CHECK: spiko-token MintAuthority PDA, validated by spiko-token program
    pub mint_authority: UncheckedAccount<'info>,

    /// CHECK: The MinterConfig PDA's permissions for the spiko-token CPI (needs MINTER role)
    pub minter_config_permissions: UncheckedAccount<'info>,
}

pub(crate) fn handler(
    ctx: Context<ApproveMint>,
    operation_id: [u8; 32],
    recipient: Pubkey,
    amount: u64,
    salt: u64,
) -> Result<()> {
    require!(
        has_role(&ctx.accounts.admin_permissions, ROLE_MINT_APPROVER)
            || ctx.accounts.permission_manager_config.admin == ctx.accounts.admin.key(),
        MinterError::Unauthorized
    );

    ctx.accounts.mint_operation.validate_for_approval(
        &operation_id,
        &ctx.accounts.mint.key(),
        &recipient,
        amount,
        salt,
    )?;

    // Execute the mint via CPI
    let cpi_accounts = spiko_token::cpi::accounts::MintTokens {
        minter: ctx.accounts.minter_config.to_account_info(),
        token_config: ctx.accounts.token_config.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        destination: ctx.accounts.destination.to_account_info(),
        mint_authority: ctx.accounts.mint_authority.to_account_info(),
        minter_permissions: ctx.accounts.minter_config_permissions.to_account_info(),
        permission_manager_config: ctx.accounts.permission_manager_config.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
        event_authority: ctx.accounts.spiko_token_event_authority.to_account_info(),
        program: ctx.accounts.spiko_token_program.to_account_info(),
    };
    let seeds = &[MINTER_CONFIG_SEED, &[ctx.accounts.minter_config.bump]];
    let signer_seeds = &[&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.spiko_token_program.key(),
        cpi_accounts,
        signer_seeds,
    );
    spiko_token::cpi::mint(cpi_ctx, amount)?;

    ctx.accounts.mint_operation.status = STATUS_DONE;

    emit_cpi!(MintApproved {
        caller: ctx.accounts.admin.key(),
        recipient,
        mint: ctx.accounts.mint.key(),
        amount,
        salt,
    });

    Ok(())
}
