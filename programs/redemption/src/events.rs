use anchor_lang::prelude::*;

#[event]
pub struct RedemptionInitialized {
    pub admin: Pubkey,
    pub deadline_delay: i64,
}

#[event]
pub struct VaultCreated {
    pub admin: Pubkey,
    pub mint: Pubkey,
}

#[event]
pub struct RedemptionInitiated {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
    pub deadline: i64,
    pub operation_id: [u8; 32],
}

#[event]
pub struct RedemptionExecuted {
    pub burner: Pubkey,
    pub user: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
}

#[event]
pub struct RedemptionCanceled {
    pub caller: Pubkey,
    pub user: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub salt: u64,
}
