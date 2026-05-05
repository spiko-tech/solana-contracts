use anchor_lang::prelude::*;

#[event]
pub struct MinterInitialized {
    pub admin: Pubkey,
    pub permission_manager: Pubkey,
    pub max_delay: i64,
}

#[event]
pub struct DailyLimitUpdated {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub limit: u64,
}

#[event]
pub struct MintInitiated {
    pub caller: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
}

#[event]
pub struct MintBlocked {
    pub caller: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
    pub deadline: i64,
}

#[event]
pub struct MintApproved {
    pub caller: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
}

#[event]
pub struct MintCanceled {
    pub caller: Pubkey,
    pub recipient: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
}
