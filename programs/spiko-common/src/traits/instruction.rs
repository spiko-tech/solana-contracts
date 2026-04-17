use pinocchio::{account::AccountView, error::ProgramError};

/// Marker trait for instruction account structs.
///
/// Implementors should use `TryFrom<&'a [AccountView]>` for parsing.
pub trait InstructionAccounts<'a>:
    Sized + TryFrom<&'a [AccountView], Error = ProgramError>
{
}

/// Marker trait for instruction data structs.
///
/// Implementors should use `TryFrom<&'a [u8]>` for parsing.
pub trait InstructionData<'a>: Sized + TryFrom<&'a [u8], Error = ProgramError> {
    /// Expected length of instruction data.
    const LEN: usize;
}
