use codama::CodamaType;
use pinocchio::Address;
use std::vec::Vec;

use crate::discriminators::PermissionManagerEventDiscriminators;
use spiko_events::{EventDiscriminator, EventSerialize};

#[derive(CodamaType)]
pub struct RoleGrantedEvent {
    pub caller: Address,
    pub target: Address,
    pub role_id: u8,
}

impl EventDiscriminator for RoleGrantedEvent {
    const DISCRIMINATOR: u8 = PermissionManagerEventDiscriminators::RoleGranted as u8;
}

impl EventSerialize for RoleGrantedEvent {
    #[inline(always)]
    fn to_bytes_inner(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(Self::DATA_LEN);
        data.extend_from_slice(self.caller.as_ref());
        data.extend_from_slice(self.target.as_ref());
        data.push(self.role_id);
        data
    }
}

impl RoleGrantedEvent {
    pub const DATA_LEN: usize = 32 + 32 + 1;

    #[inline(always)]
    pub fn new(caller: Address, target: Address, role_id: u8) -> Self {
        Self {
            caller,
            target,
            role_id,
        }
    }
}
