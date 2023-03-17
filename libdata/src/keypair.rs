//! Export [Keypair].
//! Define utility functions to [generate] and [derive] [Keypair]s.

use anyhow::Result;
use blake3::derive_key;
use datacore::SecretKey;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use bip39_dict::{Entropy, Mnemonics, ENGLISH, seed_from_mnemonics};
use getrandom::getrandom;

pub use datacore::{generate_keypair as generate, Keypair};

/// Derive a named [Keypair] from a base [SecretKey].
#[must_use]
pub fn derive(key: &SecretKey, name: &str) -> Keypair {
    let seed: <ChaCha20Rng as SeedableRng>::Seed = derive_key(name, &key.to_bytes());
    let mut rng = ChaCha20Rng::from_seed(seed);
    Keypair::generate(&mut rng)
}

/// Generate a new [Keypair] with a BIP39 mnemonic.
#[must_use]
pub fn generate_bip39() -> (Keypair, String) {
    let mut seed: [u8; 32] = Default::default();
    getrandom(&mut seed).expect("Could not get RNG");

    let entropy = Entropy::<32>::from_slice(&seed).expect("Could not seed entropy");

    let mnemonics = entropy.to_mnemonics::<24, 8>().expect("Could not get mnemonics");
    let seed: [u8; 32] = seed_from_mnemonics(
        &ENGLISH, &mnemonics, b"libdata_keypair_generate_bip39", 2048);

    let mut rng = ChaCha20Rng::from_seed(seed);
    let keypair = Keypair::generate(&mut rng);

    (keypair, mnemonics.to_string(&ENGLISH))
}

/// Recover a [Keypair] from a BIP39 mnemonic.
#[must_use]
pub fn recover_bip39(phrase: &str) -> Result<Keypair> {
    let mnemonics = Mnemonics::<24>::from_string(&ENGLISH, phrase)?;

    let seed: [u8; 32] = seed_from_mnemonics(
        &ENGLISH, &mnemonics, b"libdata_keypair_generate_bip39", 2048);

    let mut rng = ChaCha20Rng::from_seed(seed);
    let keypair = Keypair::generate(&mut rng);

    Ok(keypair)
}
