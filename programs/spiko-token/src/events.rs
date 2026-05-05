use anchor_lang::prelude::*;

#[event]
pub struct TokenInitialized {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub permission_manager: Pubkey,
}

#[event]
pub struct Paused {
    pub caller: Pubkey,
}

#[event]
pub struct Unpaused {
    pub caller: Pubkey,
}

#[event]
pub struct Minted {
    pub caller: Pubkey,
    pub mint: Pubkey,
    pub destination: Pubkey,
    pub amount: u64,
}

#[event]
pub struct Burned {
    pub caller: Pubkey,
    pub mint: Pubkey,
    pub source: Pubkey,
    pub amount: u64,
}
