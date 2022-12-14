use anyhow::{anyhow, Result};
use std::collections::VecDeque;
use std::convert::TryInto;
use std::io::{self, Error, ErrorKind};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc;
use tokio_stream::Stream;

use crate::channels::{ChannelHandle, ChannelMap};
use crate::io::IO;
use crate::message::{Packet, Frame};
use crate::schema::{Close, Data, Open, Request};
use crate::{noise, DiscoveryKey, Key, Message};

use super::Protocol;

macro_rules! return_error {
    ($msg:expr) => {
        if let Err(e) = $msg {
            return Poll::Ready(Some(Err(e)));
        }
    };
}

#[inline]
fn map_channel_err<T>(err: &mpsc::error::SendError<T>) -> Error {
    Error::new(
        ErrorKind::BrokenPipe,
        format!("Cannot forward on channel: {}", err),
    )
}

/// Concurrent channels cap.
pub const CHANNEL_CAP: usize = 1000;

/// Protocol events.
#[derive(PartialEq, Debug)]
pub enum Event {
    /// Emitted when the remote opens a channel that we did not yet open.
    DiscoveryKey(DiscoveryKey),
    /// Channel is established.
    Open(DiscoveryKey),
    /// Channel is closed.
    Close(DiscoveryKey),
    /// A new [Message] received on a channel.
    Message(DiscoveryKey, Message),
}

/// Main stage of [Protocol], contains stage-specific fields.
#[derive(Debug)]
pub struct Stage {
    handshake: Option<noise::Outcome>,
    channels: ChannelMap,
    outbound_rx: mpsc::UnboundedReceiver<Packet>,
    outbound_tx: mpsc::UnboundedSender<Packet>,
    queued_events: VecDeque<Event>,
}
impl super::Stage for Stage {}

impl<T> Protocol<T, Stage>
where
    T: AsyncWrite + AsyncRead + Send + Unpin + 'static,
{
    /// Create a new [Protocol] after completing the handshake.
    pub fn new(io: IO<T>, result: Option<noise::Outcome>) -> Self {
        // setup channels
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        Self {
            io,
            state: Stage {
                handshake: result,
                channels: ChannelMap::new(),
                outbound_tx,
                outbound_rx,
                queued_events: VecDeque::new(),
            },
        }
    }

    /// Open a new protocol channel.
    pub fn open(&mut self, key: Key) -> Result<()> {
        // Create a new channel.
        let channel_handle = self.state.channels.attach_local(key)?;
        // Safe because attach_local always puts Some(local_id)
        let local_id = channel_handle
            .local_id()
            .ok_or_else(|| anyhow!("channel is missing a local id"))?;
        let discovery_key = *channel_handle.discovery_key();

        // If the channel was already opened from the remote end, verify,
        // and if verification is ok, push a channel open event.
        if channel_handle.is_connected() {
            let (key, remote_capability) = self.state.channels.prepare_to_verify(local_id)?;
            self.verify_remote_capability(remote_capability.cloned(), key)?;
            self.queue_event(Event::Open(discovery_key));
        }

        // Tell the remote end about the new channel.
        let capability = self.capability(&key);
        let message = Message::Open(Open {
            discovery_key: discovery_key.to_vec(),
            capability,
        });
        let channel_message = Packet::new(local_id as u32, message);
        self.io.queue_frame(Frame::Message(channel_message));
        Ok(())
    }

    /// Close a protocol channel.
    pub fn close(&mut self, discovery_key: DiscoveryKey) -> Result<()> {
        self.send(
            &discovery_key,
            Message::Close(Close {
                discovery_key: discovery_key.to_vec(),
            }),
        )
    }

    /// Send a [Message] on a channel.
    fn send(&mut self, discovery_key: &DiscoveryKey, msg: Message) -> Result<()> {
        match self.state.channels.get(discovery_key) {
            None => Ok(()),
            Some(channel) => {
                if channel.is_connected() {
                    let local_id = channel
                        .local_id()
                        .ok_or_else(|| anyhow!("no local id for channel"))?;
                    let msg = Packet::new(local_id, msg);
                    self.state
                        .outbound_tx
                        .send(msg)
                        .map_err(|e| map_channel_err(&e))?;
                }
                Ok(())
            }
        }
    }
    /// Send a [Message::Request] on a channel.
    pub fn request(&mut self, discovery_key: &DiscoveryKey, msg: Request) -> Result<()> {
        self.send(discovery_key, Message::Request(msg))
    }
    /// Send a [Message::Data] on a channel.
    pub fn data(&mut self, discovery_key: &DiscoveryKey, msg: Data) -> Result<()> {
        self.send(discovery_key, Message::Data(msg))
    }

    fn poll_inbound_read(&mut self, cx: &mut Context<'_>) -> Result<()> {
        loop {
            let msg = match self.io.poll_inbound_read(cx) {
                Err(err) => return Err(err),
                Ok(msg) => msg,
            };
            match msg {
                Some(frame) => match frame {
                    Frame::Message(msg) => self.on_inbound_message(msg)?,
                    Frame::Raw(_) => unreachable!("May not receive raw frames after handshake"),
                },
                None => return Ok(()),
            };
        }
    }

    fn poll_outbound_write(&mut self, cx: &mut Context<'_>) -> Result<()> {
        loop {
            self.io.poll_outbound_write(cx)?;

            match Pin::new(&mut self.state.outbound_rx).poll_recv(cx) {
                Poll::Ready(Some(message)) => {
                    self.on_outbound_message(&message);
                    self.io.queue_frame(Frame::Message(message));
                }
                Poll::Ready(None) => unreachable!("Channel closed before end"),
                Poll::Pending => return Ok(()),
            }
        }
    }

    fn on_outbound_message(&mut self, message: &Packet) {
        // If message is close, close the local channel.
        if let Message::Close(_) = message.message() {
            self.close_local(message.channel() as usize);
        }
    }

    fn on_inbound_message(&mut self, channel_message: Packet) -> Result<()> {
        let (remote_id, message) = channel_message.into_split();
        match remote_id {
            // Id 0 means stream-level
            0 => {}
            // Any other Id is a regular channel message.
            _ => match message {
                Message::Open(msg) => self.on_open(remote_id, msg)?,
                Message::Close(msg) => self.on_close(remote_id as usize, &msg),
                _ => {
                    // Emit [Event::Message].
                    let discovery_key = self
                        .state
                        .channels
                        .get_remote(remote_id as usize)
                        .map(ChannelHandle::discovery_key);
                    if let Some(discovery_key) = discovery_key {
                        self.queue_event(Event::Message(*discovery_key, message));
                    }
                }
            },
        }
        Ok(())
    }

    fn on_open(&mut self, channel_id: u32, msg: Open) -> Result<()> {
        let discovery_key: DiscoveryKey = parse_key(&msg.discovery_key)?;
        let channel_handle =
            self.state
                .channels
                .attach_remote(discovery_key, channel_id, msg.capability)?;

        if channel_handle.is_connected() {
            let local_id = channel_handle.local_id().unwrap();
            let (key, remote_capability) = self.state.channels.prepare_to_verify(local_id)?;
            self.verify_remote_capability(remote_capability.cloned(), key)?;
            self.queue_event(Event::Open(discovery_key));
        } else {
            self.queue_event(Event::DiscoveryKey(discovery_key));
        }

        Ok(())
    }

    fn close_local(&mut self, local_id: usize) {
        let channel = self.state.channels.get_local(local_id);
        if let Some(channel) = channel {
            let discovery_key = *channel.discovery_key();
            self.state.channels.remove(&discovery_key);
            self.queue_event(Event::Close(discovery_key));
        }
    }

    fn on_close(&mut self, remote_id: usize, msg: &Close) {
        let remote = self.state.channels.get_remote(remote_id);
        if let Some(channel_handle) = remote {
            let discovery_key = *channel_handle.discovery_key();
            if msg.discovery_key == discovery_key {
                self.state.channels.remove(&discovery_key);
                self.queue_event(Event::Close(discovery_key));
            }
        }
    }

    fn queue_event(&mut self, event: Event) {
        self.state.queued_events.push_back(event);
    }

    fn capability(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.state
            .handshake
            .as_ref()
            .map(|handshake| handshake.capability(key))
    }

    fn verify_remote_capability(&self, capability: Option<Vec<u8>>, key: &[u8]) -> Result<()> {
        match self.state.handshake.as_ref() {
            Some(handshake) => handshake
                .verify_remote_capability(capability, key)
                .map_err(|err| anyhow!(err)),
            None => Err(anyhow!(Error::new(
                ErrorKind::PermissionDenied,
                "Missing handshake state for capability verification",
            ))),
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

        // Drain queued events first
        if let Some(event) = this.state.queued_events.pop_front() {
            return Poll::Ready(Some(Ok(event)));
        }

        // Read and process incoming messages
        return_error!(this.poll_inbound_read(cx));

        // Write everything we can write
        return_error!(this.poll_outbound_write(cx));

        // Check if any events are enqueued
        if let Some(event) = this.state.queued_events.pop_front() {
            Poll::Ready(Some(Ok(event)))
        } else {
            Poll::Pending
        }
    }
}

fn parse_key(key: &[u8]) -> io::Result<[u8; 32]> {
    key.try_into()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Key must be 32 bytes long"))
}
