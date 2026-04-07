use pinocchio::error::ProgramError;

#[derive(Clone, Debug, PartialEq)]
pub enum RedemptionError {
    AlreadyInitialized = 0,
    NotInitialized,
    Unauthorized,
    NotPending,
    DeadlineNotPassed,
    DeadlinePassed,
    OperationExists,
    BelowMinimum,
    InvalidPda,
    InvalidVault,
}

impl From<RedemptionError> for ProgramError {
    fn from(e: RedemptionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
