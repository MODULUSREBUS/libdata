use anyhow::{anyhow, bail, ensure, Result};
use blake2_rfc::blake2b::Blake2b;
use getrandom::getrandom;
use prost::Message;
use snow::{Builder, HandshakeState};

use super::CAP_NS_BUF;
use crate::schema::NoisePayload;

pub use snow::Keypair;

const CIPHER_KEY_LENGTH: usize = 32;
const HANDSHAKE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2b";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub is_initiator: bool,
    pub local_pubkey: Vec<u8>,
    pub local_seckey: Vec<u8>,
    pub remote_pubkey: Vec<u8>,
    pub local_nonce: Vec<u8>,
    pub remote_nonce: Vec<u8>,
    pub split_tx: [u8; CIPHER_KEY_LENGTH],
    pub split_rx: [u8; CIPHER_KEY_LENGTH],
}
impl Outcome {
    pub fn capability(&self, key: &[u8]) -> Vec<u8> {
        let mut context = Blake2b::with_key(32, &self.split_rx[..32]);
        context.update(CAP_NS_BUF);
        context.update(&self.split_tx[..32]);
        context.update(key);
        let hash = context.finalize();
        hash.as_bytes().to_vec()
    }

    pub fn remote_capability(&self, key: &[u8]) -> Vec<u8> {
        let mut context = Blake2b::with_key(32, &self.split_tx[..32]);
        context.update(CAP_NS_BUF);
        context.update(&self.split_rx[..32]);
        context.update(key);
        let hash = context.finalize();
        hash.as_bytes().to_vec()
    }

    pub fn verify_remote_capability(&self, capability: Option<Vec<u8>>, key: &[u8]) -> Result<()> {
        let expected_capability = self.remote_capability(key);
        if let Some(capability) = capability {
            if capability == expected_capability {
                Ok(())
            } else {
                bail!("Invalid remote channel capability");
            }
        } else {
            bail!("Missing capability for verification");
        }
    }
}

pub fn build_handshake_state(is_initiator: bool) -> Result<(HandshakeState, Keypair)> {
    let builder: Builder<'_> = Builder::new(HANDSHAKE_PATTERN.parse()?);
    let key_pair = builder.generate_keypair()?;
    let builder = builder.local_private_key(&key_pair.private);
    let handshake_state = if is_initiator {
        builder.build_initiator()?
    } else {
        builder.build_responder()?
    };
    Ok((handshake_state, key_pair))
}

#[derive(Debug)]
pub struct Handshake {
    outcome: Outcome,
    state: HandshakeState,
    payload: Vec<u8>,
    tx_buf: Vec<u8>,
    rx_buf: Vec<u8>,
    complete: bool,
    did_receive: bool,
}
impl Handshake {
    pub fn new(is_initiator: bool) -> Result<Self> {
        let (state, local_keypair) = build_handshake_state(is_initiator)?;

        let local_nonce = generate_nonce()?;
        let payload = encode_nonce(local_nonce.clone());

        let outcome = Outcome {
            is_initiator,
            local_pubkey: local_keypair.public,
            local_seckey: local_keypair.private,
            local_nonce,
            ..Default::default()
        };
        Ok(Self {
            state,
            outcome,
            payload,
            tx_buf: vec![0u8; 512],
            rx_buf: vec![0u8; 512],
            complete: false,
            did_receive: false,
        })
    }

    #[inline]
    pub fn start(&mut self) -> Result<Option<&'_ [u8]>> {
        if self.is_initiator() {
            let tx_len = self.send()?;
            Ok(Some(&self.tx_buf[..tx_len]))
        } else {
            Ok(None)
        }
    }

    #[inline]
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    #[inline]
    pub fn is_initiator(&self) -> bool {
        self.outcome.is_initiator
    }

    #[inline]
    fn recv(&mut self, msg: &[u8]) -> Result<usize> {
        self.state
            .read_message(msg, &mut self.rx_buf)
            .map_err(|e| anyhow!(e))
    }
    #[inline]
    fn send(&mut self) -> Result<usize> {
        self.state
            .write_message(&self.payload, &mut self.tx_buf)
            .map_err(|e| anyhow!(e))
    }

    pub fn read(&mut self, msg: &[u8]) -> Result<Option<&'_ [u8]>> {
        ensure!(!self.is_complete(), "handshake read after complete");

        let rx_len = self.recv(msg)?;

        if !self.is_initiator() && !self.did_receive {
            self.did_receive = true;
            let tx_len = self.send()?;
            return Ok(Some(&self.tx_buf[..tx_len]));
        }

        let tx_buf = if self.is_initiator() {
            let tx_len = self.send()?;
            Some(&self.tx_buf[..tx_len])
        } else {
            None
        };

        let split = self.state.dangerously_get_raw_split();
        if self.is_initiator() {
            self.outcome.split_tx = split.0;
            self.outcome.split_rx = split.1;
        } else {
            self.outcome.split_tx = split.1;
            self.outcome.split_rx = split.0;
        }
        self.outcome.remote_nonce = decode_nonce(&self.rx_buf[..rx_len])?;
        self.outcome.remote_pubkey = self.state.get_remote_static().unwrap().to_vec();
        self.complete = true;

        Ok(tx_buf)
    }

    #[inline]
    pub fn into_result(self) -> Result<Outcome> {
        ensure!(self.is_complete(), "hanshake is not complete");
        Ok(self.outcome)
    }
}

#[inline]
fn generate_nonce() -> Result<Vec<u8>> {
    let mut bytes: [u8; 24] = Default::default();
    getrandom(&mut bytes)?;
    Ok(bytes.to_vec())
}
#[inline]
fn encode_nonce(nonce: Vec<u8>) -> Vec<u8> {
    let nonce_msg = NoisePayload { nonce };
    let mut buf = Vec::with_capacity(CIPHER_KEY_LENGTH);
    nonce_msg.encode(&mut buf).unwrap();
    buf
}
#[inline]
fn decode_nonce(msg: &[u8]) -> Result<Vec<u8>> {
    let decoded = NoisePayload::decode(msg)?;
    Ok(decoded.nonce)
}
