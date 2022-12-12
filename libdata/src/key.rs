//! Export [Public] key and [Secret] key.
//! Export [Discovery] key and a [discover] to derive it from a [Public] key.

pub use datacore::{PublicKey as Public, SecretKey as Secret};
pub use protocol::{discovery_key as discovery, DiscoveryKey as Discovery};
