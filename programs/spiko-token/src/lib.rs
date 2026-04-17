use pinocchio::address::declare_id;

pub mod discriminators;
pub mod error;
pub mod events;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use instructions::*;

declare_id!("3V5sE4AFgkS8T8Jrt41wK8t2rJXo9VhURt6AGfqar9Zd");

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;
