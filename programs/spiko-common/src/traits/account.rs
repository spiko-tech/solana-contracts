use pinocchio::error::ProgramError;

/// Discriminator for account types.
/// Byte 0 of the on-chain account data.
pub trait Discriminator {
    const DISCRIMINATOR: u8;
}

/// Version marker for account types.
/// Byte 1 of the on-chain account data.
pub trait Versioned {
    const VERSION: u8;
}

/// Account size constants.
/// `DATA_LEN` is the size of the struct data (excluding discriminator and version).
/// `LEN` is the total on-chain size (discriminator + version + data).
pub trait AccountSize: Discriminator + Versioned + Sized {
    /// Size of the account data (excluding discriminator and version).
    const DATA_LEN: usize;

    /// Total on-chain size including discriminator and version.
    const LEN: usize = 1 + 1 + Self::DATA_LEN;
}

/// Zero-copy account deserialization.
///
/// On-chain layout: `[discriminator(1)] [version(1)] [struct data...]`
///
/// `from_bytes()` validates discriminator + version, then pointer-casts at byte 2.
/// `from_bytes_mut()` validates discriminator + version, then pointer-casts at byte 2.
pub trait AccountDeserialize: AccountSize {
    /// Zero-copy read from byte slice (validates discriminator and version).
    #[inline(always)]
    fn from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        require_account_len!(data, Self::LEN);
        validate_discriminator!(data, Self::DISCRIMINATOR);
        if data[1] != Self::VERSION {
            return Err(ProgramError::InvalidAccountData);
        }
        // Skip discriminator (byte 0) and version (byte 1)
        unsafe { Self::from_bytes_unchecked(&data[2..]) }
    }

    /// Zero-copy read without discriminator/version validation.
    ///
    /// # Safety
    /// Caller must ensure data is valid, properly sized, and aligned.
    /// Struct must be `#[repr(C)]` with no padding.
    #[inline(always)]
    unsafe fn from_bytes_unchecked(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() < Self::DATA_LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(&*(data.as_ptr() as *const Self))
    }

    /// Mutable zero-copy access (validates discriminator and version).
    #[inline(always)]
    fn from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        require_account_len!(data, Self::LEN);
        validate_discriminator!(data, Self::DISCRIMINATOR);
        if data[1] != Self::VERSION {
            return Err(ProgramError::InvalidAccountData);
        }
        // Skip discriminator (byte 0) and version (byte 1)
        unsafe { Self::from_bytes_mut_unchecked(&mut data[2..]) }
    }

    /// Mutable zero-copy access without validation.
    ///
    /// # Safety
    /// Caller must ensure data is valid, properly sized, and aligned.
    /// Struct must be `#[repr(C)]` with no padding.
    #[inline(always)]
    unsafe fn from_bytes_mut_unchecked(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::DATA_LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(&mut *(data.as_mut_ptr() as *mut Self))
    }

    /// Mutable zero-copy access for initialization (skips discriminator/version check).
    /// Used when creating new accounts where discriminator hasn't been written yet.
    #[inline(always)]
    fn from_bytes_mut_init(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        // Write discriminator and version
        data[0] = Self::DISCRIMINATOR;
        data[1] = Self::VERSION;
        // Skip discriminator (byte 0) and version (byte 1)
        unsafe { Self::from_bytes_mut_unchecked(&mut data[2..]) }
    }
}
