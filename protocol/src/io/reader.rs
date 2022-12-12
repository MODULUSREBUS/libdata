use anyhow::{anyhow, Result};
use futures_timer::Delay;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, ReadBuf};

use crate::message::{Frame, FrameType};
use crate::noise::{Cipher, Outcome};
use crate::MAX_MESSAGE_SIZE;

#[derive(Debug)]
enum Step {
    Header,
    Body { header_len: usize, body_len: usize },
}

#[derive(Debug)]
pub struct ReadState {
    /// The read buffer.
    buf: Vec<u8>,
    /// The end of the not-yet-processed byte range in the read buffer.
    end: usize,
    /// The logical state of the reading (either header or body).
    step: Step,
    /// The timeout after which the connection is closed.
    timeout: Option<Delay>,
    /// Timeout duration.
    timeout_duration: Option<Duration>,
    /// Optional encryption cipher.
    cipher: Option<Cipher>,
    /// The frame type to be passed to the decoder.
    frame_type: FrameType,
}

impl ReadState {
    #[inline]
    pub fn new(timeout_ms: Option<u64>) -> Self {
        let timeout_duration = timeout_ms.map(Duration::from_millis);
        Self {
            buf: vec![0u8; MAX_MESSAGE_SIZE],
            end: 0,
            step: Step::Header,
            timeout: timeout_duration.map(Delay::new),
            timeout_duration,
            cipher: None,
            frame_type: FrameType::Raw,
        }
    }

    #[inline]
    pub fn upgrade_with_handshake(&mut self, handshake: &Outcome) {
        let mut cipher = Cipher::from_handshake_rx(handshake);
        cipher.apply(&mut self.buf[..self.end]);
        self.cipher = Some(cipher);
    }
    #[inline]
    pub fn set_frame_type(&mut self, frame_type: FrameType) {
        self.frame_type = frame_type;
    }

    #[inline]
    pub fn poll_reader<R>(
        &mut self,
        cx: &mut Context<'_>,
        mut reader: &mut R,
    ) -> Poll<Result<Frame>>
    where
        R: AsyncRead + Unpin,
    {
        loop {
            match self.process() {
                Err(e) => return Poll::Ready(Err(e)),
                Ok(Some(result)) => return Poll::Ready(Ok(result)),
                Ok(None) => (),
            }

            let mut buf = ReadBuf::new(&mut self.buf[self.end..]);
            let n0 = buf.filled().len();
            let n = match Pin::new(&mut reader).poll_read(cx, &mut buf) {
                Poll::Ready(Ok(())) if (buf.filled().len() - n0) > 0 => buf.filled().len() - n0,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(anyhow!(e))),
                // If the reader is pending, poll the timeout.
                Poll::Pending | Poll::Ready(Ok(_)) => {
                    // Return Pending if the timeout is pending, or an error if the
                    // timeout expired (i.e. returned Poll::Ready).
                    return match self.timeout.as_mut() {
                        None => Poll::Pending,
                        Some(mut timeout) => match Pin::new(&mut timeout).poll(cx) {
                            Poll::Pending => Poll::Pending,
                            Poll::Ready(_) => Poll::Ready(Err(anyhow!("remote timed out"))),
                        },
                    };
                }
            };

            let end = self.end + n;
            if let Some(ref mut cipher) = self.cipher {
                cipher.apply(&mut self.buf[self.end..end]);
            }
            self.end = end;

            // reset timeout
            match self.timeout_duration {
                None => None,
                Some(timeout_duration) => self.timeout.as_mut().map(|t| t.reset(timeout_duration)),
            };
        }
    }

    #[inline]
    fn process(&mut self) -> Result<Option<Frame>> {
        if self.end == 0 {
            return Ok(None);
        }

        loop {
            match self.step {
                Step::Header => {
                    let header_len = Frame::header_len();
                    let body_len = match Frame::decode_header(&self.buf[..self.end]) {
                        Ok(Some(body_len)) => body_len,
                        Ok(None) => return Ok(None),
                        Err(e) => return Err(e),
                    };
                    let message_len = header_len + body_len;

                    if message_len > MAX_MESSAGE_SIZE {
                        return Err(anyhow!("message length above max length"));
                    }
                    self.step = Step::Body {
                        header_len,
                        body_len,
                    };
                }
                Step::Body {
                    header_len,
                    body_len,
                } => {
                    let message_len = header_len + body_len;
                    if self.end < message_len {
                        return Ok(None);
                    }

                    let frame = Frame::decode_body(&self.buf[header_len..message_len], &self.frame_type)?;
                    if self.end > message_len {
                        self.buf.copy_within(message_len..self.end, 0);
                    }
                    self.end -= message_len;
                    self.step = Step::Header;
                    return Ok(Some(frame));
                }
            }
        }
    }
}
