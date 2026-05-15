use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, MintTo, TokenAccount, TokenInterface};

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::{MintOperation, MinterConfig};

#[derive(Accounts)]
#[instruction(salt: u64)]
pub struct ApproveMint<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.admin == admin.key() @ MinterError::Unauthorized,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    #[account(
        mut,
        seeds = [MINT_OPERATION_SEED, salt.to_le_bytes().as_ref()],
        bump = mint_operation.bump,
        constraint = mint_operation.status == STATUS_PENDING @ MinterError::Unauthorized,
    )]
    pub mint_operation: Account<'info, MintOperation>,

    #[account(
        mut,
        address = mint_operation.mint,
    )]
    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        token::mint = mint,
        token::authority = mint_operation.recipient,
    )]
    pub destination: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler(ctx: Context<ApproveMint>, _salt: u64) -> Result<()> {
    let amount = ctx.accounts.mint_operation.amount;

    let seeds = &[MINTER_CONFIG_SEED, &[ctx.accounts.minter_config.bump]];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.destination.to_account_info(),
        authority: ctx.accounts.minter_config.to_account_info(),
    };
    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, signer_seeds);
    token_interface::mint_to(cpi_ctx, amount)?;

    ctx.accounts.mint_operation.status = STATUS_DONE;

    Ok(())
}
