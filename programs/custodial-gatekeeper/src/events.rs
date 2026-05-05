use anchor_lang::prelude::*;

#[event]
pub struct GatekeeperInitialized {
    pub admin: Pubkey,
}

#[event]
pub struct WithdrawalInitiated {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
    pub deadline: i64,
}

#[event]
pub struct WithdrawalApproved {
    pub caller: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
}

#[event]
pub struct WithdrawalCanceled {
    pub caller: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
}

#[event]
pub struct WithdrawalBlocked {
    pub sender: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
    pub deadline: i64,
}

#[event]
pub struct DailyLimitUpdated {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub limit: u64,
}
