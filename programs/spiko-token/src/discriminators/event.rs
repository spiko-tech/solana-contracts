/// Event discriminators for the Spiko Token program.
#[repr(u8)]
pub enum TokenEventDiscriminators {
    TokenInitialized = 0,
    Mint = 1,
    Burn = 2,
    RedeemInitiated = 3,
    TokenPaused = 4,
    TokenUnpaused = 5,
    RedemptionContractSet = 6,
}
