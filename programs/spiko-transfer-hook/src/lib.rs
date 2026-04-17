use pinocchio::address::declare_id;

pub mod discriminators;
pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("CKV53PkgjvoTmfpzdkbuQc9fMukqu7Qey7kLoSiTwYmY");

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;
