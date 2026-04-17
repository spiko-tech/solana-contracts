/// Event discriminators for the Redemption program.
#[repr(u8)]
pub enum RedemptionEventDiscriminators {
    RedemptionInitialized = 0,
    RedemptionInitiated = 1,
    RedemptionExecuted = 2,
    RedemptionCanceled = 3,
    TokenMinimumUpdated = 4,
}
