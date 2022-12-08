use blake3::derive_key;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;

pub use datacore::{generate_keypair, Keypair, PublicKey, SecretKey};
pub use protocol::{discovery_key, DiscoveryKey};

/// Derive a named [Keypair] from a base [SecretKey].
pub fn derive_keypair(key: &SecretKey, name: &str) -> Keypair {
    let seed: <ChaCha20Rng as SeedableRng>::Seed = derive_key(name, &key.to_bytes());

    let mut rng = ChaCha20Rng::from_seed(seed);
    Keypair::generate(&mut rng)
}
