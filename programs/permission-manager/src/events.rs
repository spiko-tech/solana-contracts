use anchor_lang::prelude::*;

#[event]
pub struct ConfigInitialized {
    pub admin: Pubkey,
}

#[event]
pub struct RoleGranted {
    pub caller: Pubkey,
    pub user: Pubkey,
    pub role: u16,
}

#[event]
pub struct RoleRevoked {
    pub admin: Pubkey,
    pub user: Pubkey,
    pub role: u16,
}

#[event]
pub struct AdminTransferRequested {
    pub admin: Pubkey,
    pub new_admin: Pubkey,
}

#[event]
pub struct AdminTransferAccepted {
    pub old_admin: Pubkey,
    pub new_admin: Pubkey,
}
