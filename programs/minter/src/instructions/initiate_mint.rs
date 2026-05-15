use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, MintTo, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::{MintDailyLimit, MintOperation, MinterConfig};

#[derive(Accounts)]
#[instruction(salt: u64, recipient: Pubkey)]
pub struct InitiateMint<'info> {
    pub initiator: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.mint_initiator == initiator.key() @ MinterError::Unauthorized,
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
        payer = payer,
        space = 8 + MintOperation::INIT_SPACE,
        seeds = [MINT_OPERATION_SEED, salt.to_le_bytes().as_ref()],
        bump,
    )]
    pub mint_operation: Account<'info, MintOperation>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = recipient,
    )]
    pub destination: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(
    ctx: Context<InitiateMint>,
    _salt: u64,
    recipient: Pubkey,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, MinterError::Unauthorized);

    let clock = Clock::get()?;
    let current_day = clock
        .unix_timestamp
        .checked_div(SECONDS_PER_DAY)
        .ok_or(MinterError::ArithmeticOverflow)?;

    let daily_limit = &mut ctx.accounts.mint_daily_limit;
    daily_limit.reset_if_new_day(current_day);

    if daily_limit.is_within_limit(amount)? {
        let seeds = &[MINTER_CONFIG_SEED, &[ctx.accounts.minter_config.bump]];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = MintTo {
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.destination.to_account_info(),
            authority: ctx.accounts.minter_config.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.key(),
            cpi_accounts,
            signer_seeds,
        );
        token_interface::mint_to(cpi_ctx, amount)?;

        daily_limit.record_usage(amount)?;

        ctx.accounts.mint_operation.set_inner(MintOperation {
            status: STATUS_DONE,
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            bump: ctx.bumps.mint_operation,
        });
    } else {
        ctx.accounts.mint_operation.set_inner(MintOperation {
            status: STATUS_PENDING,
            recipient,
            mint: ctx.accounts.mint.key(),
            amount,
            bump: ctx.bumps.mint_operation,
        });
    }

    Ok(())
}
