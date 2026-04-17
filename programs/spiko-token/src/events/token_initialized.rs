use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::TokenEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct TokenInitializedEvent {
    pub admin: Address,
    pub mint: Address,
}

impl EventDiscriminator for TokenInitializedEvent {
    const DISCRIMINATOR: u8 = TokenEventDiscriminators::TokenInitialized as u8;
}

impl EventSerialize for TokenInitializedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.admin.as_ref());
        data.extend_from_slice(self.mint.as_ref());
        data
    }
}

impl TokenInitializedEvent {
    pub const DATA_LEN: usize = 32 + 32;

    #[inline(always)]
    pub fn new(admin: Address, mint: Address) -> Self {
        Self { admin, mint }
    }
}
