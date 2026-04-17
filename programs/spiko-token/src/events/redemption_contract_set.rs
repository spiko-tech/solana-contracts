use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::TokenEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct RedemptionContractSetEvent {
    pub caller: Address,
    pub config: Address,
    pub contract: Address,
}

impl EventDiscriminator for RedemptionContractSetEvent {
    const DISCRIMINATOR: u8 = TokenEventDiscriminators::RedemptionContractSet as u8;
}

impl EventSerialize for RedemptionContractSetEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.caller.as_ref());
        data.extend_from_slice(self.config.as_ref());
        data.extend_from_slice(self.contract.as_ref());
        data
    }
}

impl RedemptionContractSetEvent {
    pub const DATA_LEN: usize = 32 + 32 + 32;

    #[inline(always)]
    pub fn new(caller: Address, config: Address, contract: Address) -> Self {
        Self {
            caller,
            config,
            contract,
        }
    }
}
