use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;

use crate::constants::*;
use crate::errors::RedemptionError;
use crate::events::RedemptionInitiated;
use crate::state::*;
use crate::utils::verify_operation_id;

#[derive(Accounts)]
#[instruction(operation_id: [u8; 32])]
pub struct OnRedeem<'info> {
    pub user: Signer<'info>,

    /// spiko-token MintAuthority PDA — must be a signer, enforcing CPI-only access.
    pub mint_authority: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [VAULT_AUTHORITY_SEED, mint.key().as_ref()],
        bump = vault_authority.bump,
    )]
    pub vault_authority: Account<'info, VaultAuthority>,

    #[account(
        seeds = [REDEMPTION_CONFIG_SEED],
        bump,
    )]
    pub redemption_config: Account<'info, RedemptionConfig>,

    #[account(
        init,
        payer = payer,
        space = 8 + RedemptionOperation::INIT_SPACE,
        seeds = [REDEMPTION_OPERATION_SEED, operation_id.as_ref()],
        bump,
    )]
    pub redemption_operation: Account<'info, RedemptionOperation>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(
    ctx: Context<OnRedeem>,
    operation_id: [u8; 32],
    amount: u64,
    salt: u64,
) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();
    verify_operation_id(
        &ctx.accounts.user.key(),
        &mint_key,
        amount,
        salt,
        &operation_id,
    )?;

    let clock = Clock::get()?;
    let config = &ctx.accounts.redemption_config;
    let deadline = clock
        .unix_timestamp
        .checked_add(config.deadline_delay)
        .ok_or(RedemptionError::ArithmeticOverflow)?;

    ctx.accounts
        .redemption_operation
        .set_inner(RedemptionOperation {
            status: STATUS_PENDING,
            deadline,
            user: ctx.accounts.user.key(),
            mint: mint_key,
            amount,
        });

    emit!(RedemptionInitiated {
        user: ctx.accounts.user.key(),
        mint: mint_key,
        amount,
        salt,
        deadline,
        operation_id,
    });

    Ok(())
}
