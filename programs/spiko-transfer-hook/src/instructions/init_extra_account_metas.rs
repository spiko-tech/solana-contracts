use pinocchio::{
    account::AccountView,
    address::Address,
    cpi::{Seed, Signer},
    error::ProgramError,
    ProgramResult,
};

use spiko_token::state::TokenConfig;

use crate::{
    error::TransferHookError,
    helpers::{create_pda_account, verify_pda},
    state::EXTRA_ACCOUNT_METAS_SEED,
};

/// Number of extra accounts declared in the ExtraAccountMetaList.
///
/// 5 extra accounts:
///   [0] PermissionManager program (literal)
///   [1] SpikoToken program (literal)
///   [2] TokenConfig PDA (external PDA from SpikoToken)
///   [3] Sender UserPermissions PDA (external PDA from PermissionManager)
///   [4] Recipient UserPermissions PDA (external PDA from PermissionManager)
const EXTRA_ACCOUNT_COUNT: usize = 5;

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
/// ExtraAccountMetaList PDA and writes the 5 extra account meta entries
/// that Token-2022 will resolve when calling the Transfer Hook Execute handler.
///
/// The 5 extra accounts are:
///   0. PermissionManager program (literal pubkey)
///   1. SpikoToken program (literal pubkey)
///   2. TokenConfig PDA (seeds: ["token_config", mint] → SpikoToken program)
///   3. Sender UserPermissions PDA (seeds: ["user_perm", source_owner] → PermissionManager)
///   4. Recipient UserPermissions PDA (seeds: ["user_perm", dest_owner] → PermissionManager)
///
/// Combined account indices in the Execute call:
///   0 = Source token account
///   1 = Mint
///   2 = Destination token account
///   3 = Owner/delegate
///   4 = ExtraAccountMetaList PDA
///   5 = Extra[0] = PermissionManager program
///   6 = Extra[1] = SpikoToken program
///   7 = Extra[2] = TokenConfig PDA
///   8 = Extra[3] = Sender UserPerms
///   9 = Extra[4] = Recipient UserPerms
///
/// Accounts:
///   0. `[signer, writable]` Admin / deployer (payer)
///   1. `[writable]`         ExtraAccountMetaList PDA (to be created)
///   2. `[]`                 Token-2022 Mint
///   3. `[]`                 TokenConfig PDA (must be initialized, owned by spiko-token)
///   4. `[]`                 SpikoToken program
///   5. `[]`                 System program
///
/// Data:
///   (none beyond the discriminator byte)
pub struct InitExtraAccountMetas<'a> {
    pub admin: &'a AccountView,
    pub extra_account_meta_list: &'a AccountView,
    pub mint: &'a AccountView,
    pub token_config: &'a AccountView,
    pub spiko_token_program: &'a AccountView,
    pub system_program: &'a AccountView,
}

impl<'a> TryFrom<(&'a [u8], &'a [AccountView])> for InitExtraAccountMetas<'a> {
    type Error = ProgramError;

    fn try_from((_data, accounts): (&'a [u8], &'a [AccountView])) -> Result<Self, Self::Error> {
        let [admin, extra_account_meta_list, mint, token_config, spiko_token_program, system_program, ..] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !admin.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(Self {
            admin,
            extra_account_meta_list,
            mint,
            token_config,
            spiko_token_program,
            system_program,
        })
    }
}

impl<'a> InitExtraAccountMetas<'a> {
    pub fn process(&self, program_id: &Address) -> ProgramResult {
        let mint_key = self.mint.address();

        // 1. Verify TokenConfig is initialized (owned by spiko-token program)
        if !self
            .token_config
            .owned_by(self.spiko_token_program.address())
        {
            return Err(TransferHookError::NotInitialized.into());
        }

        // 2. Read permission_manager and spiko_token program from config
        let (permission_manager_bytes, spiko_token_bytes) = {
            let config_data = self.token_config.try_borrow()?;
            let config = TokenConfig::from_bytes(&config_data)?;

            // Verify the mint matches
            if &config.spl_mint != mint_key {
                return Err(TransferHookError::InvalidMint.into());
            }

            // spiko_token_program matches owner of token_config — already
            // verified by the `owned_by` check above.

            (
                config.permission_manager.to_bytes(),
                self.spiko_token_program.address().to_bytes(),
            )
        };

        // 3. Verify ExtraAccountMetaList PDA is not already initialized
        if self.extra_account_meta_list.owned_by(program_id) {
            return Err(TransferHookError::AlreadyInitialized.into());
        }

        // 4. Derive ExtraAccountMetaList PDA: ["extra-account-metas", mint_pubkey]
        let meta_list_bump = verify_pda(
            self.extra_account_meta_list,
            &[EXTRA_ACCOUNT_METAS_SEED, mint_key.as_ref()],
            program_id,
        )?;

        // 5. Create the PDA account
        let bump_bytes = [meta_list_bump];
        let seeds = crate::helpers::extra_account_metas_seeds(mint_key.as_ref(), &bump_bytes);
        let signer = Signer::from(&seeds);

        create_pda_account(
            self.admin,
            self.extra_account_meta_list,
            EXTRA_ACCOUNT_META_LIST_SIZE,
            program_id,
            &[signer],
        )?;

        // 6. Write the ExtraAccountMetaList data
        {
            let mut data = self.extra_account_meta_list.try_borrow_mut()?;
            let buf = &mut data[..EXTRA_ACCOUNT_META_LIST_SIZE];

            // TLV discriminator (8 bytes)
            buf[0..8].copy_from_slice(&TLV_DISCRIMINATOR);

            // TLV length (4 bytes LE) = 4 + (5 * 35) = 179
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
            // Seeds: [Literal("token_config"), AccountKey(1=mint)]
            // Derived from SpikoToken program at combined index 6
            // Discriminator = 128 + 6 = 134
            let meta2 = build_token_config_pda_meta();
            buf[86..121].copy_from_slice(&meta2);

            // Extra[3]: Sender UserPermissions PDA
            // Seeds: [Literal("user_perm"), AccountData(account=0, offset=32, length=32)]
            // Derived from PermissionManager at combined index 5
            // Discriminator = 128 + 5 = 133
            let meta3 = build_user_perms_pda_meta(0); // source token account = combined index 0
            buf[121..156].copy_from_slice(&meta3);

            // Extra[4]: Recipient UserPermissions PDA
            // Seeds: [Literal("user_perm"), AccountData(account=2, offset=32, length=32)]
            // Derived from PermissionManager at combined index 5
            // Discriminator = 128 + 5 = 133
            let meta4 = build_user_perms_pda_meta(2); // dest token account = combined index 2
            buf[156..191].copy_from_slice(&meta4);
        }

        pinocchio_log::log!("ExtraAccountMetaList initialized");

        Ok(())
    }
}

// -----------------------------------------------------------------
// ExtraAccountMeta builders (35 bytes each)
// -----------------------------------------------------------------

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
    meta[0] = 128 + 6; // discriminator: external PDA, program at combined index 6 (SpikoToken)

    // address_config (32 bytes):
    // Seed 1: Literal("token_config") = [0x01, 0x0C, b"token_config"] = 14 bytes
    meta[1] = 1; // Seed::Literal
    meta[2] = 12; // length
    meta[3..15].copy_from_slice(b"token_config");

    // Seed 2: AccountKey(1) = [0x03, 0x01] = 2 bytes
    meta[15] = 3; // Seed::AccountKey
    meta[16] = 1; // account index 1 = mint

    // bytes 17..33 = zero padding (already zeroed)

    meta[33] = 0; // is_signer = false
    meta[34] = 0; // is_writable = false
    meta
}

/// Build UserPermissions PDA ExtraAccountMeta.
/// Seeds: [Literal("user_perm"), AccountData(token_account_index, 32, 32)]
/// Discriminator = 128 + 5 = 133 (PDA from PermissionManager at combined index 5).
///
/// `token_account_index`: 0 for source (sender), 2 for destination (recipient).
/// The owner pubkey is at offset 32 in Token-2022 account data layout.
#[inline]
fn build_user_perms_pda_meta(token_account_index: u8) -> [u8; 35] {
    let mut meta = [0u8; EXTRA_ACCOUNT_META_SIZE];
    meta[0] = 128 + 5; // discriminator: external PDA, program at combined index 5 (PermissionManager)

    // address_config (32 bytes):
    // Seed 1: Literal("user_perm") = [0x01, 0x09, b"user_perm"] = 11 bytes
    meta[1] = 1; // Seed::Literal
    meta[2] = 9; // length
    meta[3..12].copy_from_slice(b"user_perm");

    // Seed 2: AccountData(token_account_index, 32, 32) = [0x04, index, 0x20, 0x20] = 4 bytes
    meta[12] = 4; // Seed::AccountData
    meta[13] = token_account_index; // account index in combined list
    meta[14] = 32; // data_index: offset to owner field in Token-2022 account
    meta[15] = 32; // length: 32 bytes (pubkey size)

    // bytes 16..33 = zero padding (already zeroed)

    meta[33] = 0; // is_signer = false
    meta[34] = 0; // is_writable = false
    meta
}

// -----------------------------------------------------------------
// Signer seeds builder
// -----------------------------------------------------------------

/// Build Seed array for ExtraAccountMetaList PDA.
/// Seeds: ["extra-account-metas", mint_pubkey, bump]
#[inline]
pub fn extra_account_metas_seeds<'a>(mint_key: &'a [u8], bump: &'a [u8; 1]) -> [Seed<'a>; 3] {
    [
        Seed::from(EXTRA_ACCOUNT_METAS_SEED),
        Seed::from(mint_key),
        Seed::from(bump.as_ref()),
    ]
}
