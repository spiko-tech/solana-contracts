use anchor_lang::prelude::*;

#[event]
pub struct HookInitialized {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub permission_manager: Pubkey,
}

#[event]
pub struct TransferExecuted {
    pub source: Pubkey,
    pub destination: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}
