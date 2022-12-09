#![forbid(unsafe_code, bad_style, nonstandard_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2021_compatibility)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, deny(warnings))]

//! Libdata re-exports public interface from [datacore],
//! defines async [CoreIterator],
//! defines interface for managing collection of [Cores],
//! and specifies [replication] over [protocol].

mod cores;
mod iter;

pub mod key;
pub mod keypair;
pub mod replication;

pub use datacore::ed25519_dalek;
pub use datacore::{Core, IndexAccess, Signature, MAX_CORE_LENGTH};

pub use cores::Cores;
pub use iter::CoreIterator;
pub use key::{DiscoveryKey, PublicKey, SecretKey};
pub use keypair::Keypair;

