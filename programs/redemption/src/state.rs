use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct RedemptionConfig {
    pub admin: Pubkey,
    pub redemption_authority: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct VaultAuthority {
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct RedemptionRecord {
    pub bump: u8,
}
