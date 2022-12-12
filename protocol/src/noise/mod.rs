mod cipher;
mod handshake;

pub use cipher::Cipher;
pub use handshake::{Handshake, Outcome};

/// Seed for the capability hash
pub const CAP_NS_BUF: &[u8] = b"hypercore capability";
