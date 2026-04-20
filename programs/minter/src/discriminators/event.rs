/// Event discriminators for the Minter program.
#[repr(u8)]
pub enum MinterEventDiscriminators {
    MinterInitialized = 0,
    MintInitiated = 1,
    MintApproved = 2,
    MintCanceled = 3,
    MintBlocked = 4,
    DailyLimitUpdated = 5,
    MaxDelayUpdated = 6,
}
