use codama::CodamaAccount;
use pinocchio::{account::AccountView, address::Address, error::ProgramError};

use spiko_common::{
    assert_no_padding, AccountDeserialize, AccountSize, Discriminator, PdaAccount, PdaSeeds,
    Versioned,
};

pub const TOKEN_CONFIG_SEED: &[u8] = b"token_config";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";

/// Backward-compatible discriminator constant for cross-program imports.
pub const DISCRIMINATOR_TOKEN_CONFIG: u8 = 1;

/// Configuration for a single Spiko fund token.
///
/// Each fund (e.g., EUTBL, USTBL) has its own TokenConfig PDA and its
/// own Token-2022 mint. The TokenConfig ties the mint to the
/// PermissionManager and stores the paused state.
///
/// Seeds: ["token_config", spl_mint_pubkey]
///
/// On-chain layout (total: 101 bytes):
///   [0]       discriminator (u8) -- external, trait-provided
///   [1]       version (u8) -- external, trait-provided
///   [2]       bump (u8)
///   [3]       paused (u8, 0 = unpaused, 1 = paused)
///   [4]       mint_authority_bump (u8) -- bump for the mint authority PDA
///   [5..37]   permission_manager program ID (Address / 32 bytes)
///   [37..69]  spl_mint address (Address / 32 bytes)
///   [69..101] redemption_contract program ID (Address / 32 bytes, all zeros = not set)
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(field("discriminator", number(u8), default_value = 1))]
#[codama(discriminator(field = "discriminator"))]
#[codama(seed(type = string(utf8), value = "token_config"))]
#[codama(seed(name = "splMint", type = public_key))]
#[repr(C)]
pub struct TokenConfig {
    pub bump: u8,
    pub paused: u8,
    pub mint_authority_bump: u8,
    pub permission_manager: Address,
    pub spl_mint: Address,
    pub redemption_contract: Address,
}

assert_no_padding!(TokenConfig, 1 + 1 + 1 + 32 + 32 + 32);

impl Discriminator for TokenConfig {
    const DISCRIMINATOR: u8 = DISCRIMINATOR_TOKEN_CONFIG;
}

impl Versioned for TokenConfig {
    const VERSION: u8 = 1;
}

impl AccountSize for TokenConfig {
    const DATA_LEN: usize = 1 + 1 + 1 + 32 + 32 + 32; // bump + paused + mint_authority_bump + permission_manager + spl_mint + redemption_contract
}

impl AccountDeserialize for TokenConfig {}

impl PdaSeeds for TokenConfig {
    const PREFIX: &'static [u8] = TOKEN_CONFIG_SEED;

    fn validate_pda_address(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<u8, ProgramError> {
        let (derived, bump) =
            Address::find_program_address(&[Self::PREFIX, self.spl_mint.as_ref()], program_id);
        if account.address() != &derived {
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(bump)
    }
}

impl PdaAccount for TokenConfig {
    fn bump(&self) -> u8 {
        self.bump
    }

    fn validate_self(
        &self,
        account: &AccountView,
        program_id: &Address,
    ) -> Result<(), ProgramError> {
        let (derived, _) =
            Address::find_program_address(&[Self::PREFIX, self.spl_mint.as_ref()], program_id);
        if account.address() != &derived {
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(())
    }
}

/// Mint authority PDA — no on-chain data, used only as a CPI signer.
///
/// This struct exists solely so Codama auto-generates `findMintAuthorityPda`.
/// Seeds: ["mint_authority", mint_pubkey]
#[derive(Clone, Debug, PartialEq, CodamaAccount)]
#[codama(pda = "mintAuthority")]
#[codama(seed(type = string(utf8), value = "mint_authority"))]
#[codama(seed(name = "mint", type = public_key))]
pub struct MintAuthority {
    pub bump: u8,
}

impl TokenConfig {
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused != 0
    }

    /// Returns true if a redemption contract has been set (non-zero address).
    #[inline]
    pub fn has_redemption_contract(&self) -> bool {
        self.redemption_contract.to_bytes() != [0u8; 32]
    }
}
