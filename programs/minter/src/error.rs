use pinocchio::error::ProgramError;

#[derive(Clone, Debug, PartialEq)]
pub enum MinterError {
    AlreadyInitialized = 0,
    NotInitialized,
    Unauthorized,
    NotPending,
    DeadlinePassed,
    DeadlineNotPassed,
    OperationExists,
    OperationFinalized,
    InvalidPda,
    InvalidLimit,
    ArithmeticOverflow,
}

impl From<MinterError> for ProgramError {
    fn from(e: MinterError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
