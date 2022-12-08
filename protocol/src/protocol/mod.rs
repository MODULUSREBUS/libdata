use tokio::io::{AsyncRead, AsyncWrite};

use crate::io::IO;
use crate::Options;

/// Handshake stage of the [Protocol].
pub mod handshake;
/// Main stage of the [Protocol].
pub mod main;

/// Init a new [Protocol] with [Options].
#[inline]
pub fn new<T>(io: T, options: Options) -> Protocol<T, handshake::Stage>
where
    T: AsyncWrite + AsyncRead + Send + Unpin + 'static,
{
    Protocol::<T, handshake::Stage>::new(io, options)
}

/// Init a new [Protocol] with default [Options].
#[inline]
pub fn default<T>(io: T, is_initiator: bool) -> Protocol<T, handshake::Stage>
where
    T: AsyncWrite + AsyncRead + Send + Unpin + 'static,
{
    let options = Options::new(is_initiator);
    new(io, options)
}

/// [Stage] of the [Protocol].
pub trait Stage {}

/// Replication [Protocol].
#[derive(Debug)]
pub struct Protocol<T, S: Stage> {
    io: IO<T>,
    state: S,
}
