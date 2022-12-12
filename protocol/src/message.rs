use anyhow::{bail, ensure, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use prost::Message as _;
use std::io::Write;
use std::mem::size_of;

use super::schema::{Close, Data, Open, Request};

/// Encode data into a buffer.
///
/// This trait is implemented on data frames and their components
/// (channel messages, messages, and individual message types through prost).
pub trait Encoder: Sized + std::fmt::Debug {
    /// Calculates the length that the encoded message needs.
    fn encoded_len(&self) -> usize;

    /// Encodes the message to a buffer.
    ///
    /// An error will be returned if the buffer does not have sufficient capacity.
    fn encode(&self, buf: &mut [u8]) -> Result<usize>;
}

impl Encoder for &[u8] {
    #[inline]
    fn encoded_len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn encode(&self, mut buf: &mut [u8]) -> Result<usize> {
        let len = self.encoded_len();
        ensure!(buf.len() >= len);
        buf.write_all(self)?;
        Ok(len)
    }
}

/// The type of a data frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameType {
    Raw,
    Message,
}

/// A frame of data, either a buffer or a message.
#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    /// A raw binary buffer. Used in the handshaking phase.
    Raw(Vec<u8>),
    /// A message. Used for everything after the handshake.
    Message(Packet),
}
impl From<Vec<u8>> for Frame {
    #[inline]
    fn from(m: Vec<u8>) -> Self {
        Self::Raw(m)
    }
}
impl From<Packet> for Frame {
    #[inline]
    fn from(m: Packet) -> Self {
        Self::Message(m)
    }
}
impl Frame {
    /// Decode [Frame] header.
    #[inline]
    pub fn decode_header(mut buf: &[u8]) -> Result<Option<usize>> {
        if buf.len() >= Self::header_len() {
            let body_len = buf.read_u32::<LittleEndian>()?;
            Ok(Some(usize::try_from(body_len)?))
        } else {
            Ok(None)
        }
    }
    /// Decode a [Frame] from a buffer.
    #[inline]
    pub fn decode_body(buf: &[u8], frame_type: &FrameType) -> Result<Self> {
        match frame_type {
            FrameType::Raw => Ok(Frame::Raw(buf.to_vec())),
            FrameType::Message => Ok(Frame::Message(Packet::decode(buf)?)),
        }
    }

    /// Get frame header size.
    #[inline]
    pub const fn header_len() -> usize {
        size_of::<u32>()
    }
    #[inline]
    fn body_len(&self) -> usize {
        match self {
            Self::Raw(message) => message.as_slice().encoded_len(),
            Self::Message(message) => message.encoded_len(),
        }
    }
}
impl Encoder for Frame {
    #[inline]
    fn encoded_len(&self) -> usize {
        Self::header_len() + self.body_len()
    }
    #[inline]
    fn encode(&self, mut buf: &mut [u8]) -> Result<usize> {
        let len = self.encoded_len();
        ensure!(buf.len() >= len);

        let body_len = u32::try_from(self.body_len())?;
        buf.write_u32::<LittleEndian>(body_len)?;
        match self {
            Self::Raw(ref msg) => msg.as_slice().encode(buf),
            Self::Message(ref msg) => msg.encode(buf),
        }?;
        Ok(len)
    }
}

/// A message on a channel.
#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    channel: u32,
    message: Message,
}
impl Packet {
    /// Create a new message.
    #[inline]
    pub fn new(channel: u32, message: Message) -> Self {
        Self { channel, message }
    }

    /// Consume self and return (channel, Message).
    #[inline]
    pub fn into_split(self) -> (u32, Message) {
        (self.channel, self.message)
    }
    /// Access `channel`.
    #[must_use]
    #[inline]
    pub fn channel(&self) -> u32 {
        self.channel
    }
    /// Access `message`.
    #[must_use]
    #[inline]
    pub fn message(&self) -> &Message {
        &self.message
    }

    /// Decode a channel message from a buffer.
    ///
    /// Note: `buf` has to have a valid length, and the length
    /// prefix has to be removed already.
    #[inline]
    pub fn decode(mut buf: &[u8]) -> Result<Self> {
        ensure!(!buf.is_empty());
        let channel = buf.read_u32::<LittleEndian>()?;
        let message = Message::decode(buf)?;
        Ok(Self { channel, message })
    }

    #[inline]
    const fn header_len() -> usize {
        size_of::<u32>()
    }
}
impl Encoder for Packet {
    #[inline]
    fn encoded_len(&self) -> usize {
        Self::header_len() + self.message.encoded_len()
    }
    #[inline]
    fn encode(&self, mut buf: &mut [u8]) -> Result<usize> {
        let header_len = Self::header_len();
        let body_len = self.message.encoded_len();
        let len = header_len + body_len;
        ensure!(buf.len() >= len);

        buf.write_u32::<LittleEndian>(self.channel)?;
        self.message.encode(&mut buf[..body_len])?;
        Ok(len)
    }
}

/// A protocol message.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// Open a channel.
    Open(Open),
    /// Close a channel.
    Close(Close),
    /// Request a block.
    Request(Request),
    /// Send a Data block.
    Data(Data),
}
impl Message {
    /// Decode a message from a buffer.
    #[inline]
    pub fn decode(mut buf: &[u8]) -> Result<Self> {
        let kind = buf.read_u8()?;
        match kind {
            0 => Ok(Self::Open(Open::decode(buf)?)),
            1 => Ok(Self::Close(Close::decode(buf)?)),
            2 => Ok(Self::Request(Request::decode(buf)?)),
            3 => Ok(Self::Data(Data::decode(buf)?)),
            _ => bail!("invalid message kind"),
        }
    }
    /// Wire kind of this message.
    #[inline]
    fn kind(&self) -> u8 {
        match self {
            Self::Open(_) => 0,
            Self::Close(_) => 1,
            Self::Request(_) => 2,
            Self::Data(_) => 3,
        }
    }

    #[inline]
    const fn header_len() -> usize {
        size_of::<u8>()
    }
    #[inline]
    fn body_len(&self) -> usize {
        match self {
            Self::Open(ref message) => message.encoded_len(),
            Self::Close(ref message) => message.encoded_len(),
            Self::Request(ref message) => message.encoded_len(),
            Self::Data(ref message) => message.encoded_len(),
        }
    }
}
impl Encoder for Message {
    #[inline]
    fn encoded_len(&self) -> usize {
        Self::header_len() + self.body_len()
    }
    #[inline]
    fn encode(&self, mut buf: &mut [u8]) -> Result<usize> {
        buf.write_u8(self.kind())?;
        match self {
            Self::Open(ref message) => encode_prost_message(message, buf),
            Self::Close(ref message) => encode_prost_message(message, buf),
            Self::Request(ref message) => encode_prost_message(message, buf),
            Self::Data(ref message) => encode_prost_message(message, buf),
        }
    }
}
#[inline]
fn encode_prost_message(msg: &impl prost::Message, mut buf: &mut [u8]) -> Result<usize> {
    let len = msg.encoded_len();
    msg.encode(&mut buf)?;
    Ok(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use getrandom::getrandom;

    #[test]
    fn encode_decode_raw() {
        let expected_len = 256;
        let mut data = vec![0u8; expected_len];
        getrandom(&mut data).expect("Could not getrandom");
        let frame = Frame::from(data);
        let mut buf = vec![0u8; frame.encoded_len()];
        println!("buffer size: {}", buf.len());
        let n = frame.encode(&mut buf[..]).unwrap();
        let header_len = Frame::header_len();
        assert_eq!(expected_len + header_len, n);
        let body_len = Frame::decode_header(&buf[..]).unwrap().unwrap();
        let message_len = header_len + body_len;
        let decoded = Frame::decode_body(&buf[header_len..message_len], &FrameType::Raw).unwrap();
        assert_eq!(expected_len, body_len);
        assert_eq!(frame, decoded);
    }

    macro_rules! message_enc_dec {
        ($( $msg:expr ),*) => {
            $(
                let mut channel: [u8; 1] = Default::default();
                getrandom(&mut channel)
                    .expect("Could not getrandom");
                let channel = u32::from(channel[0]);
                let channel_message = Packet::new(channel, $msg);
                let mut buf = vec![0u8; channel_message.encoded_len()];
                let n = channel_message.encode(&mut buf[..])
                    .expect("Failed to encode message");
                let decoded = Packet::decode(&buf[..n])
                    .expect("Failed to decode message")
                    .into_split();
                assert_eq!(channel, decoded.0);
                assert_eq!($msg, decoded.1);
            )*
        }
    }

    #[test]
    fn encode_decode_message() {
        message_enc_dec! {
            Message::Open(Open {
                discovery_key: vec![2u8; 20],
                capability: None
            }),
            Message::Close(Close {
                discovery_key: vec![1u8; 10]
            }),
            Message::Request(Request {
                index: 0,
            }),
            Message::Data(Data {
                index: 1,
                data: vec![0u8; 10],
                data_signature: vec![1u8; 32],
                tree_signature: vec![2u8; 32],
            })
        };
    }
}
