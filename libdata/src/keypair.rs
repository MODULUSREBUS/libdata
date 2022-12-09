//! Export [Keypair].
//! Define utility functions to [generate] and [derive] [Keypair]s.

use blake3::derive_key;
use datacore::SecretKey;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub use datacore::{generate_keypair as generate, Keypair};

/// Derive a named [Keypair] from a base [SecretKey].
#[must_use]
pub fn derive(key: &SecretKey, name: &str) -> Keypair {
    let seed: <ChaCha20Rng as SeedableRng>::Seed = derive_key(name, &key.to_bytes());
    let mut rng = ChaCha20Rng::from_seed(seed);
    Keypair::generate(&mut rng)
}
