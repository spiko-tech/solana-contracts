use pinocchio::address::Address;

// -----------------------------------------------------------------
// PDA Seeds
// -----------------------------------------------------------------

pub const TOKEN_CONFIG_SEED: &[u8] = b"token_config";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";

// -----------------------------------------------------------------
// Account discriminator
// -----------------------------------------------------------------

pub const DISCRIMINATOR_TOKEN_CONFIG: u8 = 1;

// -----------------------------------------------------------------
// Token decimals (as specified in Architecture: 5 decimal places)
// -----------------------------------------------------------------

pub const TOKEN_DECIMALS: u8 = 5;

// -----------------------------------------------------------------
// TokenConfig -- Per-fund configuration account
// -----------------------------------------------------------------

/// Configuration for a single Spiko fund token.
///
/// Each fund (e.g., EUTBL, USTBL) has its own TokenConfig PDA and its
/// own Token-2022 mint. The TokenConfig ties the mint to the
/// PermissionManager and stores the paused state.
///
/// Seeds: ["token_config", spl_mint_pubkey]
///
/// Layout (total: 100 bytes):
///   [0]       discriminator (u8)
///   [1]       bump (u8)
///   [2]       paused (u8, 0 = unpaused, 1 = paused)
///   [3]       mint_authority_bump (u8) -- bump for the mint authority PDA
///   [4..36]   permission_manager program ID (Address / 32 bytes)
///   [36..68]  spl_mint address (Address / 32 bytes)
///   [68..100] redemption_contract program ID (Address / 32 bytes, all zeros = not set)
#[repr(C)]
pub struct TokenConfig {
    pub discriminator: u8,
    pub bump: u8,
    pub paused: u8,
    pub mint_authority_bump: u8,
    pub permission_manager: Address,
    pub spl_mint: Address,
    pub redemption_contract: Address,
}

impl TokenConfig {
    pub const LEN: usize = core::mem::size_of::<Self>();

    pub fn from_bytes(data: &[u8]) -> Result<&Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        if data[0] != DISCRIMINATOR_TOKEN_CONFIG {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr() as *const Self) })
    }

    pub fn from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, pinocchio::error::ProgramError> {
        if data.len() < Self::LEN {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

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
