use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::MinterError;
use crate::events::{MintBlocked, MintInitiated};
use crate::state::*;
use crate::utils::verify_operation_id;

#[derive(Accounts)]
#[instruction(operation_id: [u8; 32])]
#[event_cpi]
pub struct InitiateMint<'info> {
    #[account(mut)]
    pub minter: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    #[account(
        mut,
        seeds = [MINT_DAILY_LIMIT_SEED, mint.key().as_ref()],
        bump = mint_daily_limit.bump,
    )]
    pub mint_daily_limit: Account<'info, MintDailyLimit>,

    #[account(
        init,
        payer = minter,
        space = 8 + MintOperation::INIT_SPACE,
        seeds = [MINT_OPERATION_SEED, operation_id.as_ref()],
        bump,
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
        seeds = [USER_PERMISSION_SEED, minter.key().as_ref(), permission_manager_config.key().as_ref()],
        seeds::program = permission_manager_program_id(),
        bump = minter_permissions.bump,
        constraint = has_role(&minter_permissions, ROLE_MINT_INITIATOR) @ MinterError::Unauthorized,
    )]
    pub minter_permissions: Account<'info, UserPermissions>,

    /// UserPermissions for MinterConfig PDA (must have ROLE_MINTER for spiko-token CPI)
    /// CHECK: Validated by spiko-token program during CPI
    pub minter_config_permissions: UncheckedAccount<'info>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        seeds::program = permission_manager_program_id(),
        bump = permission_manager_config.bump,
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

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(
    ctx: Context<InitiateMint>,
    operation_id: [u8; 32],
    recipient: Pubkey,
    amount: u64,
    salt: u64,
) -> Result<()> {
    require!(amount > 0, MinterError::InvalidAmount);
    verify_operation_id(
        &recipient,
        &ctx.accounts.mint.key(),
        amount,
        salt,
        &operation_id,
    )?;

    let clock = Clock::get()?;
    let current_day = clock
        .unix_timestamp
        .checked_div(SECONDS_PER_DAY)
        .ok_or(MinterError::ArithmeticOverflow)?;

    let daily_limit = &mut ctx.accounts.mint_daily_limit;
    daily_limit.reset_if_new_day(current_day);

    let operation = &mut ctx.accounts.mint_operation;

    if daily_limit.is_within_limit(amount)? {
        // Within daily limit — mint immediately
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

        daily_limit.record_usage(amount)?;

        operation.set_inner(MintOperation {
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            status: STATUS_DONE,
            deadline: 0,
            bump: ctx.bumps.mint_operation,
        });
    } else {
        let deadline = clock
            .unix_timestamp
            .checked_add(ctx.accounts.minter_config.max_delay)
            .ok_or(MinterError::ArithmeticOverflow)?;

        operation.set_inner(MintOperation {
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            status: STATUS_PENDING,
            deadline,
            bump: ctx.bumps.mint_operation,
        });

        emit_cpi!(MintBlocked {
            caller: ctx.accounts.minter.key(),
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            salt,
            deadline,
        });
    }

    emit_cpi!(MintInitiated {
        caller: ctx.accounts.minter.key(),
        recipient,
        mint: ctx.accounts.mint.key(),
        amount,
        salt,
    });

    Ok(())
}
