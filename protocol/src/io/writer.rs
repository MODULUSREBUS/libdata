use std::collections::VecDeque;
use std::fmt::Debug;
use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::AsyncWrite;

use crate::message::{EncodeError, Encoder, Frame};
use crate::noise::{Cipher, HandshakeResult};

const BUF_SIZE: usize = 1024 * 64;

#[derive(Debug)]
pub enum Step {
    Flushing,
    Writing,
    Processing,
}

#[derive(Debug)]
pub struct WriteState {
    queue: VecDeque<Frame>,
    buf: Vec<u8>,
    current_frame: Option<Frame>,
    start: usize,
    end: usize,
    cipher: Option<Cipher>,
    step: Step,
}
impl Default for WriteState {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            buf: vec![0u8; BUF_SIZE],
            current_frame: None,
            start: 0,
            end: 0,
            cipher: None,
            step: Step::Processing,
        }
    }
}
impl WriteState {
    pub fn upgrade_with_handshake(&mut self, handshake: &HandshakeResult) {
        self.cipher = Some(Cipher::from_handshake_tx(handshake));
    }

    pub fn queue_frame<F>(&mut self, frame: F)
    where
        F: Into<Frame>,
    {
        self.queue.push_back(frame.into())
    }

    fn try_queue_direct<T: Encoder>(
        &mut self,
        frame: &T,
    ) -> std::result::Result<bool, EncodeError> {
        let len = frame.encoded_len();
        if self.buf.len() < len {
            self.buf.resize(len, 0u8);
        }
        if len > self.remaining() {
            return Ok(false);
        }
        let len = frame.encode(&mut self.buf[self.end..])?;

        // advance
        let end = self.end + len;
        if let Some(ref mut cipher) = self.cipher {
            cipher.apply(&mut self.buf[self.end..end]);
        }
        self.end = end;

        Ok(true)
    }

    fn remaining(&self) -> usize {
        self.buf.len() - self.end
    }
    fn pending(&self) -> usize {
        self.end - self.start
    }

    pub fn poll_send<W>(&mut self, cx: &mut Context<'_>, mut writer: &mut W) -> Poll<Result<()>>
    where
        W: AsyncWrite + Unpin,
    {
        loop {
            self.step = match self.step {
                Step::Processing => {
                    if self.current_frame.is_none() && !self.queue.is_empty() {
                        self.current_frame = self.queue.pop_front();
                    }

                    if let Some(frame) = self.current_frame.take() {
                        if !self.try_queue_direct(&frame)? {
                            self.current_frame = Some(frame);
                        }
                    }
                    if self.pending() == 0 {
                        return Poll::Ready(Ok(()));
                    }
                    Step::Writing
                }
                Step::Writing => {
                    let n = match Pin::new(&mut writer)
                        .poll_write(cx, &self.buf[self.start..self.end])?
                    {
                        Poll::Ready(n) => n,
                        Poll::Pending => return Poll::Pending,
                    };
                    self.start += n;
                    if self.start == self.end {
                        self.start = 0;
                        self.end = 0;
                    }
                    Step::Flushing
                }
                Step::Flushing => {
                    match Pin::new(&mut writer).poll_flush(cx)? {
                        Poll::Ready(_) => (),
                        Poll::Pending => return Poll::Pending,
                    };
                    Step::Processing
                }
            }
        }
    }
}
