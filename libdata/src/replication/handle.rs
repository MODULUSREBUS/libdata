use anyhow::{anyhow, Result};
use std::fmt::Debug;
use tokio::sync::mpsc::UnboundedSender;

use crate::replication::ReplicaTrait;
use crate::key;

/// [Link] command.
pub enum Command {
    /// Open a new replica.
    Open(key::Public, Box<dyn ReplicaTrait + Send>),
    /// Re-open a replica.
    ReOpen(key::Discovery),
    /// Close a replica.
    Close(key::Discovery),
    /// End the [Link].
    Quit(),
}
impl Debug for Command {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Open(key, _) => write!(fmt, "Command::Open({:?})", key),
            Self::ReOpen(key) => write!(fmt, "Command::ReOpen({:?})", key),
            Self::Close(key) => write!(fmt, "Command::Close({:?})", key),
            Self::Quit() => write!(fmt, "Command::Quit()"),
        }
    }
}

/// [Link] handle.
#[derive(Debug, Clone)]
pub struct Handle {
    tx: UnboundedSender<Command>,
}
impl Handle {
    /// Create [LinkHandle].
    #[must_use]
    pub fn new(tx: UnboundedSender<Command>) -> Self {
        Self { tx }
    }

    /// Open a new channel with [ReplicaTrait].
    pub fn open(&mut self, key: &key::Public, replica: Box<dyn ReplicaTrait + Send>) -> Result<()> {
        self.send(Command::Open(*key, replica))
    }

    /// Reopen a replica.
    pub fn reopen(&mut self, key: &key::Public) -> Result<()> {
        self.send(Command::ReOpen(key::discovery(key.as_slice().try_into().unwrap())))
    }

    /// Close a channel by [key::Discovery].
    pub fn close(&mut self, key: key::Discovery) -> Result<()> {
        self.send(Command::Close(key))
    }

    /// End the [Link].
    pub fn quit(&mut self) -> Result<()> {
        self.send(Command::Quit())
    }

    #[inline]
    fn send(&mut self, cmd: Command) -> Result<()> {
        self.tx
            .send(cmd)
            .map_err(|_| anyhow!("Error sending command."))
    }
}
