use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::TokenEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct TokenPausedEvent {
    pub caller: Address,
    pub config: Address,
}

impl EventDiscriminator for TokenPausedEvent {
    const DISCRIMINATOR: u8 = TokenEventDiscriminators::TokenPaused as u8;
}

impl EventSerialize for TokenPausedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.caller.as_ref());
        data.extend_from_slice(self.config.as_ref());
        data
    }
}

impl TokenPausedEvent {
    pub const DATA_LEN: usize = 32 + 32;

    #[inline(always)]
    pub fn new(caller: Address, config: Address) -> Self {
        Self { caller, config }
    }
}
