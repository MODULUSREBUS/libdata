//! Export [PublicKey] and [SecretKey].
//! Export [DiscoveryKey] and a function to create it from a [PublicKey].

pub use datacore::{PublicKey, SecretKey};
pub use protocol::{discovery_key as discovery, DiscoveryKey};
