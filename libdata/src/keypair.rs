//! Export [Keypair].
//! Define utility functions to [generate] and [derive] [Keypair]s.

use anyhow::Result;
use blake3::derive_key;
use datacore::SecretKey;
use bip39_dict::{Entropy, Mnemonics, ENGLISH, seed_from_mnemonics};
use getrandom::getrandom;

pub use datacore::{KeyPair, Seed};

/// Derive a named [KeyPair] from a base [SecretKey].
#[must_use]
pub fn derive(key: &SecretKey, name: &str) -> KeyPair {
    let seed = Seed::new(derive_key(name, key.as_slice()));
    KeyPair::from_seed(seed)
}

/// Generate a new [Keypair] with a BIP39 mnemonic.
#[must_use]
pub fn generate_bip39() -> (KeyPair, String) {
    let mut seed: [u8; 32] = Default::default();
    getrandom(&mut seed).expect("Could not get RNG");

    let entropy = Entropy::<32>::from_slice(&seed).expect("Could not seed entropy");

    let mnemonics = entropy.to_mnemonics::<24, 8>().expect("Could not get mnemonics");
    let seed = Seed::new(seed_from_mnemonics(
        &ENGLISH, &mnemonics, b"libdata_keypair_generate_bip39", 2048));

    let keypair = KeyPair::from_seed(seed);

    (keypair, mnemonics.to_string(&ENGLISH))
}

/// Recover a [Keypair] from a BIP39 mnemonic.
#[must_use]
pub fn recover_bip39(phrase: &str) -> Result<KeyPair> {
    let mnemonics = Mnemonics::<24>::from_string(&ENGLISH, phrase)?;

    let seed = Seed::new(seed_from_mnemonics(
        &ENGLISH, &mnemonics, b"libdata_keypair_generate_bip39", 2048));

    let keypair = KeyPair::from_seed(seed);

    Ok(keypair)
}
