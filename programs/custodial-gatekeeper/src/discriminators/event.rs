/// Event discriminators for the CustodialGatekeeper program.
#[repr(u8)]
pub enum GatekeeperEventDiscriminators {
    GatekeeperInitialized = 0,
    WithdrawalInitiated = 1,
    WithdrawalApproved = 2,
    WithdrawalCanceled = 3,
    WithdrawalBlocked = 4,
    DailyLimitUpdated = 5,
}
