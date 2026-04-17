/// Event discriminators for the Minter program.
#[repr(u8)]
pub enum MinterEventDiscriminators {
    MinterInitialized = 0,
    MintExecuted = 1,
    MintBlocked = 2,
    MintApproved = 3,
    MintCanceled = 4,
    DailyLimitUpdated = 5,
    MaxDelayUpdated = 6,
}
