use anyhow::{anyhow, Result};
use std::fmt::Debug;
use tokio::sync::mpsc::UnboundedSender;

use crate::replication::ReplicaTrait;
use crate::{discovery_key, DiscoveryKey, PublicKey};

/// [Link] command.
pub enum Command {
    /// Open a new replica.
    Open(PublicKey, Box<dyn ReplicaTrait + Send>),
    /// Re-open a replica.
    ReOpen(DiscoveryKey),
    /// Close a replica.
    Close(DiscoveryKey),
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
pub struct LinkHandle {
    tx: UnboundedSender<Command>,
}
impl LinkHandle {
    /// Create [LinkHandle].
    pub fn new(tx: UnboundedSender<Command>) -> Self {
        Self { tx }
    }

    /// Open a new channel with [ReplicaTrait].
    pub fn open(&mut self, key: &PublicKey, replica: Box<dyn ReplicaTrait + Send>) -> Result<()> {
        self.send(Command::Open(*key, replica))
    }

    /// Reopen a replica.
    pub fn reopen(&mut self, key: &PublicKey) -> Result<()> {
        self.send(Command::ReOpen(discovery_key(key.as_bytes())))
    }

    /// Close a channel by [DiscoveryKey].
    pub fn close(&mut self, key: DiscoveryKey) -> Result<()> {
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
