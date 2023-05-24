//! Generate a `Keypair`, sign and verify messages with `Keypair`.
//! Uses `Ed25519` cryptography.

use anyhow::{ensure, Result};

pub use ed25519_compact::{KeyPair, Seed, PublicKey, SecretKey, Signature};

/// Sign a byte slice.
#[must_use]
pub fn sign(secret: &SecretKey, msg: &[u8]) -> Signature {
    secret.sign(msg, None)
}

/// Verify a signature of a byte slice.
pub fn verify(public: &PublicKey, msg: &[u8], signature: &Signature) -> Result<()> {
    ensure!(public.verify(msg, signature).is_ok(), "Signature invalid.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_verify() {
        let keypair = KeyPair::generate();
        let msg = b"hello";
        let signature = sign(&keypair.sk, msg);
        assert!(verify(&keypair.pk, msg, &signature).is_ok());
        assert!(verify(&keypair.pk, b"oops", &signature).is_err());
    }
}
