use generic_array::GenericArray;
use salsa20::cipher::{KeyIvInit, StreamCipher};
use salsa20::XSalsa20;

use super::Outcome;

// TODO: Don't define here but use the values from the XSalsa20 impl.
const KEY_SIZE: usize = 32;
const NONCE_SIZE: usize = 24;

pub struct Cipher(XSalsa20);
impl std::fmt::Debug for Cipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Cipher(XSalsa20)")
    }
}
impl Cipher {
    pub fn from_handshake_rx(handshake: &Outcome) -> Self {
        let cipher = XSalsa20::new(
            GenericArray::from_slice(&handshake.split_rx[..KEY_SIZE]),
            GenericArray::from_slice(&handshake.remote_nonce[..NONCE_SIZE]),
        );
        Self(cipher)
    }

    pub fn from_handshake_tx(handshake: &Outcome) -> Self {
        let cipher = XSalsa20::new(
            GenericArray::from_slice(&handshake.split_tx[..KEY_SIZE]),
            GenericArray::from_slice(&handshake.local_nonce[..NONCE_SIZE]),
        );
        Self(cipher)
    }

    pub fn apply(&mut self, buffer: &mut [u8]) {
        self.0.apply_keystream(buffer);
    }
}
