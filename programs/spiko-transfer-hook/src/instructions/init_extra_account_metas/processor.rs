use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::{Seed, Signer},
    error::ProgramError,
    ProgramResult,
};

use spiko_common::AccountDeserialize;
use spiko_token::state::TokenConfig;

use crate::{
    error::TransferHookError,
    helpers::{create_pda_account, verify_pda},
    state::EXTRA_ACCOUNT_METAS_SEED,
};

use super::accounts::InitExtraAccountMetasAccounts;

/// Number of extra accounts declared in the ExtraAccountMetaList.
///
/// 7 extra accounts:
///   [0] PermissionManager program (literal)
///   [1] SpikoToken program (literal)
///   [2] TokenConfig PDA (external PDA from SpikoToken)
///   [3] Sender UserPermissions PDA (external PDA from PermissionManager)
///   [4] Recipient UserPermissions PDA (external PDA from PermissionManager)
///   [5] Event authority PDA (literal)
///   [6] Transfer hook program itself (literal)
const EXTRA_ACCOUNT_COUNT: usize = 7;

/// Size of each ExtraAccountMeta entry.
const EXTRA_ACCOUNT_META_SIZE: usize = 35;

/// Total size of the ExtraAccountMetaList account data.
/// Layout: TLV discriminator (8) + TLV length (4) + PodSlice count (4) + N * 35
const EXTRA_ACCOUNT_META_LIST_SIZE: usize =
    8 + 4 + 4 + (EXTRA_ACCOUNT_COUNT * EXTRA_ACCOUNT_META_SIZE);

/// TLV type discriminator = SHA256("spl-transfer-hook-interface:execute")[0..8]
/// Same as the Execute sighash.
const TLV_DISCRIMINATOR: [u8; 8] = [105, 37, 101, 197, 75, 251, 102, 26];

/// Initialize the ExtraAccountMetaList PDA for a given mint.
///
/// This must be called once per mint after InitializeToken. It creates the
/// ExtraAccountMetaList PDA and writes the 7 extra account meta entries
/// that Token-2022 will resolve when calling the Transfer Hook Execute handler.
pub struct InitExtraAccountMetas<'a> {
    pub accounts: InitExtraAccountMetasAccounts<'a>,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitExtraAccountMetas<'a> {
    type Error = ProgramError;

    fn try_from((_data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: InitExtraAccountMetasAccounts::try_from(accounts)?,
        })
    }
}

impl<'a> InitExtraAccountMetas<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let mint_key = self.accounts.mint.address();

        if !self
            .accounts
            .token_config
            .owned_by(self.accounts.spiko_token_program.address())
        {
            return Err(TransferHookError::NotInitialized.into());
        }

        let (permission_manager_bytes, spiko_token_bytes) = {
            let config_data = self.accounts.token_config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;

            // Verify the mint matches
            if &config.spl_mint != mint_key {
                return Err(TransferHookError::InvalidMint.into());
            }

            (
                config.permission_manager.to_bytes(),
                self.accounts.spiko_token_program.address().to_bytes(),
            )
        };

        if self.accounts.extra_account_meta_list.owned_by(program_id) {
            return Err(TransferHookError::AlreadyInitialized.into());
        }

        let meta_list_bump = verify_pda(
            self.accounts.extra_account_meta_list,
            &[EXTRA_ACCOUNT_METAS_SEED, mint_key.as_ref()],
            program_id,
        )?;

        let bump_bytes = [meta_list_bump];
        let seeds = crate::helpers::extra_account_metas_seeds(mint_key.as_ref(), &bump_bytes);
        let signer = Signer::from(&seeds);

        create_pda_account(
            self.accounts.admin,
            self.accounts.extra_account_meta_list,
            EXTRA_ACCOUNT_META_LIST_SIZE,
            program_id,
            &[signer],
        )?;

        {
            let mut data = self.accounts.extra_account_meta_list.try_borrow_mut()?;
            let buf = &mut data[..EXTRA_ACCOUNT_META_LIST_SIZE];

            // TLV discriminator (8 bytes)
            buf[0..8].copy_from_slice(&TLV_DISCRIMINATOR);

            // TLV length (4 bytes LE) = 4 + (7 * 35) = 249
            let tlv_length: u32 = 4 + (EXTRA_ACCOUNT_COUNT as u32 * EXTRA_ACCOUNT_META_SIZE as u32);
            buf[8..12].copy_from_slice(&tlv_length.to_le_bytes());

            // PodSlice count (4 bytes LE)
            buf[12..16].copy_from_slice(&(EXTRA_ACCOUNT_COUNT as u32).to_le_bytes());

            // Extra[0]: PermissionManager program (literal pubkey)
            let meta0 = build_literal_meta(&permission_manager_bytes);
            buf[16..51].copy_from_slice(&meta0);

            // Extra[1]: SpikoToken program (literal pubkey)
            let meta1 = build_literal_meta(&spiko_token_bytes);
            buf[51..86].copy_from_slice(&meta1);

            // Extra[2]: TokenConfig PDA
            let meta2 = build_token_config_pda_meta();
            buf[86..121].copy_from_slice(&meta2);

            // Extra[3]: Sender UserPermissions PDA
            let meta3 = build_user_perms_pda_meta(0);
            buf[121..156].copy_from_slice(&meta3);

            // Extra[4]: Recipient UserPermissions PDA
            let meta4 = build_user_perms_pda_meta(2);
            buf[156..191].copy_from_slice(&meta4);

            // Extra[5]: Event authority PDA (literal pubkey)
            let meta5 = build_literal_meta(&crate::events::event_authority_pda::ID.to_bytes());
            buf[191..226].copy_from_slice(&meta5);

            // Extra[6]: Transfer hook program itself (literal pubkey)
            let meta6 = build_literal_meta(&crate::ID.to_bytes());
            buf[226..261].copy_from_slice(&meta6);
        }

        Ok(())
    }
}

/// Build a literal pubkey ExtraAccountMeta (discriminator = 0).
#[inline]
fn build_literal_meta(pubkey: &[u8; 32]) -> [u8; 35] {
    let mut meta = [0u8; EXTRA_ACCOUNT_META_SIZE];
    meta[0] = 0; // discriminator: literal pubkey
    meta[1..33].copy_from_slice(pubkey);
    meta[33] = 0; // is_signer = false
    meta[34] = 0; // is_writable = false
    meta
}

/// Build TokenConfig PDA ExtraAccountMeta.
/// Seeds: [Literal("token_config"), AccountKey(1)]
/// Discriminator = 128 + 6 = 134 (PDA from SpikoToken program at combined index 6).
#[inline]
fn build_token_config_pda_meta() -> [u8; 35] {
    let mut meta = [0u8; EXTRA_ACCOUNT_META_SIZE];
    meta[0] = 128 + 6;
    meta[1] = 1; // Seed::Literal
    meta[2] = 12; // length
    meta[3..15].copy_from_slice(b"token_config");
    meta[15] = 3; // Seed::AccountKey
    meta[16] = 1; // account index 1 = mint
    meta[33] = 0; // is_signer = false
    meta[34] = 0; // is_writable = false
    meta
}

/// Build UserPermissions PDA ExtraAccountMeta.
/// Seeds: [Literal("user_perm"), AccountData(token_account_index, 32, 32)]
/// Discriminator = 128 + 5 = 133 (PDA from PermissionManager at combined index 5).
#[inline]
fn build_user_perms_pda_meta(token_account_index: u8) -> [u8; 35] {
    let mut meta = [0u8; EXTRA_ACCOUNT_META_SIZE];
    meta[0] = 128 + 5;
    meta[1] = 1; // Seed::Literal
    meta[2] = 9; // length
    meta[3..12].copy_from_slice(b"user_perm");
    meta[12] = 4; // Seed::AccountData
    meta[13] = token_account_index;
    meta[14] = 32; // data_index: offset to owner field
    meta[15] = 32; // length: pubkey size
    meta[33] = 0; // is_signer = false
    meta[34] = 0; // is_writable = false
    meta
}
