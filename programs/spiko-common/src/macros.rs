/// Validate that instruction data is at least `$len` bytes.
/// Returns `ProgramError::InvalidInstructionData` on failure.
#[macro_export]
macro_rules! require_len {
    ($data:expr, $len:expr) => {
        if $data.len() < $len {
            return Err(pinocchio::error::ProgramError::InvalidInstructionData);
        }
    };
}

/// Validate that account data is at least `$len` bytes.
/// Returns `ProgramError::InvalidAccountData` on failure.
#[macro_export]
macro_rules! require_account_len {
    ($data:expr, $len:expr) => {
        if $data.len() < $len {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
    };
}

/// Validate that byte 0 of data matches the expected discriminator.
/// Returns `ProgramError::InvalidAccountData` on failure.
#[macro_export]
macro_rules! validate_discriminator {
    ($data:expr, $disc:expr) => {
        if $data[0] != $disc {
            return Err(pinocchio::error::ProgramError::InvalidAccountData);
        }
    };
}

/// Compile-time assertion that `size_of::<$type>()` equals `$expected`.
/// Catches silent padding in `#[repr(C)]` structs.
#[macro_export]
macro_rules! assert_no_padding {
    ($type:ty, $expected:expr) => {
        const _: () = assert!(
            core::mem::size_of::<$type>() == $expected,
            concat!(
                "unexpected padding in ",
                stringify!($type),
                ": size_of != expected"
            )
        );
    };
}
