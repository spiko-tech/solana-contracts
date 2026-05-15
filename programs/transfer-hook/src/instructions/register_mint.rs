use anchor_lang::prelude::*;
use anchor_lang::system_program;
use anchor_spl::token_interface::Mint;
use spl_tlv_account_resolution::{
    account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
};

use crate::constants::*;
use crate::errors::TransferHookError;
use crate::state::HookConfig;

#[derive(Accounts)]
pub struct RegisterMint<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(
        seeds = [HOOK_CONFIG_SEED],
        bump = hook_config.bump,
        constraint = hook_config.admin == admin.key() @ TransferHookError::Unauthorized,
    )]
    pub hook_config: Account<'info, HookConfig>,

    /// CHECK: ExtraAccountMetaList PDA, initialized manually.
    #[account(
        mut,
        seeds = [EXTRA_ACCOUNT_METAS_SEED, mint.key().as_ref()],
        bump,
    )]
    pub extra_account_metas: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub(crate) fn handler(ctx: Context<RegisterMint>) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();

    // ExtraAccountMetas: HookConfig (global), source WhitelistState, dest WhitelistState
    let hook_config_meta = ExtraAccountMeta::new_with_seeds(
        &[Seed::Literal {
            bytes: HOOK_CONFIG_SEED.to_vec(),
        }],
        false,
        false,
    )
    .unwrap();

    // Source WhitelistState PDA: seeds = [WHITELIST_SEED, source_owner]
    // source_owner is at account_data[0] offset 32 (owner field in TokenAccount)
    let source_whitelist_meta = ExtraAccountMeta::new_with_seeds(
        &[
            Seed::Literal {
                bytes: WHITELIST_SEED.to_vec(),
            },
            Seed::AccountData {
                account_index: 0, // source token account
                data_index: 32,   // owner field offset
                length: 32,
            },
        ],
        false,
        false,
    )
    .unwrap();

    // Destination WhitelistState PDA: seeds = [WHITELIST_SEED, dest_owner]
    let dest_whitelist_meta = ExtraAccountMeta::new_with_seeds(
        &[
            Seed::Literal {
                bytes: WHITELIST_SEED.to_vec(),
            },
            Seed::AccountData {
                account_index: 2, // destination token account
                data_index: 32,   // owner field offset
                length: 32,
            },
        ],
        false,
        false,
    )
    .unwrap();

    let extra_metas = vec![hook_config_meta, source_whitelist_meta, dest_whitelist_meta];

    let account_size = ExtraAccountMetaList::size_of(extra_metas.len()).unwrap();
    let lamports = Rent::get()?.minimum_balance(account_size);

    let extra_metas_bump = ctx.bumps.extra_account_metas;
    let signer_seeds: &[&[u8]] = &[
        EXTRA_ACCOUNT_METAS_SEED,
        mint_key.as_ref(),
        &[extra_metas_bump],
    ];

    system_program::create_account(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.key(),
            system_program::CreateAccount {
                from: ctx.accounts.admin.to_account_info(),
                to: ctx.accounts.extra_account_metas.to_account_info(),
            },
            &[signer_seeds],
        ),
        lamports,
        account_size as u64,
        ctx.program_id,
    )?;

    let mut data = ctx.accounts.extra_account_metas.try_borrow_mut_data()?;
    ExtraAccountMetaList::init::<spl_transfer_hook_interface::instruction::ExecuteInstruction>(
        &mut data,
        &extra_metas,
    )?;

    Ok(())
}
