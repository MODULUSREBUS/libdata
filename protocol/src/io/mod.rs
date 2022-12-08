mod reader;
mod writer;

use anyhow::{anyhow, Result};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};

use self::reader::ReadState;
use self::writer::WriteState;
use crate::message::{Frame, FrameType};
use crate::Options;
use crate::noise::HandshakeResult;

#[derive(Debug)]
pub struct IO<T> {
    io: T,
    options: Options,
    reader: ReadState,
    writer: WriteState,
}

impl<T> IO<T>
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    /// Create [IO].
    #[must_use]
    #[inline]
    pub fn new(io: T, options: Options) -> Self {
        let keepalive_ms = options.keepalive_ms;
        Self {
            io,
            options,
            reader: ReadState::new(keepalive_ms),
            writer: WriteState::default(),
        }
    }

    /// Check if `noise` is enabled.
    #[must_use]
    #[inline]
    pub fn noise_enabled(&self) -> bool {
        self.options.noise
    }
    /// Check if this is set as the initiator.
    #[must_use]
    #[inline]
    pub fn is_initiator(&self) -> bool {
        self.options.is_initiator
    }

    /// Upgrade with [HandshakeResult].
    #[inline]
    pub fn upgrade_with_handshake(&mut self, result: &Option<HandshakeResult>) {
        if self.options.encrypted {
            if let Some(handshake) = &result {
                self.reader.upgrade_with_handshake(handshake);
                self.writer.upgrade_with_handshake(handshake);
            }
        }
        self.reader.set_frame_type(FrameType::Message);
    }

    /// Poll for inbound messages and process them.
    #[inline]
    pub fn poll_inbound_read(&mut self, cx: &mut Context<'_>) -> Result<Option<Frame>> {
        let msg = self.reader.poll_reader(cx, &mut self.io);
        match msg {
            Poll::Ready(Ok(message)) => Ok(Some(message)),
            Poll::Ready(Err(e)) => Err(anyhow!(e)),
            Poll::Pending => Ok(None),
        }
    }

    /// Poll for outbound messages and write them.
    #[inline]
    pub fn poll_outbound_write(&mut self, cx: &mut Context<'_>) -> Result<()> {
        if let Poll::Ready(Err(e)) = self.writer.poll_send(cx, &mut self.io) {
            Err(anyhow!(e))
        } else {
            Ok(())
        }
    }

    /// Queue a frame to be sent.
    #[inline]
    pub fn queue_frame<F: Into<Frame>>(&mut self, frame: F) {
        self.writer.queue_frame(frame);
    }
}
