use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::constants::*;
use crate::errors::MinterError;
use crate::events::DailyLimitUpdated;
use crate::state::*;

#[derive(Accounts)]
#[event_cpi]
pub struct SetDailyLimit<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    #[account(
        init_if_needed,
        payer = admin,
        space = 8 + MintDailyLimit::INIT_SPACE,
        seeds = [MINT_DAILY_LIMIT_SEED, mint.key().as_ref()],
        bump,
    )]
    pub mint_daily_limit: Account<'info, MintDailyLimit>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        owner = permission_manager_program_id(),
        seeds = [PERMISSION_MANAGER_CONFIG_SEED],
        bump = permission_manager_config.bump,
        seeds::program = permission_manager_program_id(),
        constraint = permission_manager_config.admin == admin.key() @ MinterError::Unauthorized,
    )]
    pub permission_manager_config: Account<'info, PermissionConfig>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<SetDailyLimit>, limit: u64) -> Result<()> {
    let clock = Clock::get()?;
    let current_day = clock
        .unix_timestamp
        .checked_div(SECONDS_PER_DAY)
        .ok_or(MinterError::ArithmeticOverflow)?;

    let daily_limit = &mut ctx.accounts.mint_daily_limit;
    daily_limit.limit = limit;
    daily_limit.used_amount = 0;
    daily_limit.last_day = current_day;
    daily_limit.bump = ctx.bumps.mint_daily_limit;

    emit_cpi!(DailyLimitUpdated {
        admin: ctx.accounts.admin.key(),
        mint: ctx.accounts.mint.key(),
        limit,
    });

    Ok(())
}
