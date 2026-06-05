use anchor_lang::prelude::*;
use anchor_spl::token_interface::{self, Mint, SetAuthority, TokenInterface};
use spl_token_2022_interface::instruction::AuthorityType;

use crate::constants::*;
use crate::errors::MinterError;
use crate::state::MinterConfig;

#[derive(Accounts)]
pub struct SetMintAuthority<'info> {
    pub admin: Signer<'info>,

    #[account(
        seeds = [MINTER_CONFIG_SEED],
        bump = minter_config.bump,
        constraint = minter_config.admin == admin.key() @ MinterError::Unauthorized,
    )]
    pub minter_config: Account<'info, MinterConfig>,

    #[account(mut)]
    pub mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
}

pub(crate) fn handler(ctx: Context<SetMintAuthority>, new_authority: Pubkey) -> Result<()> {
    let seeds = &[MINTER_CONFIG_SEED, &[ctx.accounts.minter_config.bump]];
    let signer_seeds = &[&seeds[..]];

    let cpi_accounts = SetAuthority {
        current_authority: ctx.accounts.minter_config.to_account_info(),
        account_or_mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx =
        CpiContext::new_with_signer(ctx.accounts.token_program.key(), cpi_accounts, signer_seeds);
    token_interface::set_authority(cpi_ctx, AuthorityType::MintTokens, Some(new_authority))?;

    Ok(())
}
