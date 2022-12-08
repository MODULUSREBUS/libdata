use anyhow::{anyhow, Result};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::{Stream, StreamExt};

use crate::io::IO;
use crate::message::{Frame, FrameType};
use crate::noise;
use crate::Options;

use super::{main, Protocol, ProtocolStage};

macro_rules! return_error {
    ($msg:expr) => {
        if let Err(e) = $msg {
            return Poll::Ready(Some(Err(e)));
        }
    };
}

/// Handshake events.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// Emitted after the handshake with the remote peer is complete.
    /// This is the first event (if the handshake is not disabled).
    Handshake(noise::HandshakeResult),
}

/// Handshake stage of [Protocol], contains stage-specific fields.
#[derive(Debug)]
pub struct Stage {
    handshake: Option<noise::Handshake>,
}
impl ProtocolStage for Stage {}

impl<T> Protocol<T, Stage>
where
    T: AsyncWrite + AsyncRead + Send + Unpin + 'static,
{
    /// Create a new replication protocol in handshake stage.
    pub fn new(io: T, options: Options) -> Self {
        let mut io = IO::new(io, options);
        io.read_state.set_frame_type(FrameType::Raw);

        Self {
            io,
            state: Stage { handshake: None },
        }
    }

    /// Wait for handshake and upgrade to [Protocol<IO>].
    pub async fn handshake(mut self) -> Result<Protocol<T, main::Stage>> {
        if !self.io.options.noise {
            return Ok(Protocol::<T, main::Stage>::new(self.io, None));
        }

        let Event::Handshake(handshake) = self.next().await.unwrap()?;

        Ok(Protocol::<T, main::Stage>::new(self.io, Some(handshake)))
    }

    fn init(&mut self) -> Result<()> {
        if self.io.options.noise {
            let mut handshake = noise::Handshake::new(self.io.options.is_initiator)?;
            // If the handshake start returns a buffer, send it now.
            if let Some(buf) = handshake.start()? {
                self.io.queue_frame(buf.to_vec());
            }
            self.state.handshake = Some(handshake);
        };
        Ok(())
    }

    fn on_handshake_message(&mut self, buf: Vec<u8>) -> Result<()> {
        let mut handshake = match self.state.handshake.take() {
            Some(handshake) => handshake,
            None => return Err(anyhow!("Handshake empty and received a handshake message")),
        };

        if let Some(response_buf) = handshake.read(&buf)? {
            self.io.queue_frame(response_buf.to_vec());
        }

        self.state.handshake = Some(handshake);
        Ok(())
    }

    fn poll_inbound_read(&mut self, cx: &mut Context<'_>) -> Result<()> {
        loop {
            let msg = self.io.poll_inbound_read(cx)?;
            match msg {
                Some(frame) => match frame {
                    Frame::Raw(buf) => self.on_handshake_message(buf)?,
                    _ => unreachable!("May not receive message frames when not established"),
                },
                None => return Ok(()),
            };
        }
    }

    fn check_handshake_complete(&mut self) -> Option<Result<noise::HandshakeResult>> {
        let handshake = match self.state.handshake.take() {
            Some(handshake) => handshake,
            None => return None,
        };

        if handshake.complete() {
            Some(handshake.into_result().map_err(|err| anyhow!(err)))
        } else {
            self.state.handshake = Some(handshake);
            None
        }
    }
}

impl<T> Stream for Protocol<T, Stage>
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Item = Result<Event>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if this.state.handshake.is_none() {
            return_error!(this.init());
        }

        // Read and process incoming messages
        return_error!(this.poll_inbound_read(cx));

        // Write everything we can write.
        return_error!(this.io.poll_outbound_write(cx));

        match this.check_handshake_complete() {
            Some(result) => Poll::Ready(Some(result.map(Event::Handshake))),
            None => Poll::Pending,
        }
    }
}
