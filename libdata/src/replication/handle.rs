use anyhow::{anyhow, Result};
use std::fmt::Debug;
use tokio::sync::mpsc::UnboundedSender;

use crate::replication::ReplicaTrait;
use crate::{discovery_key, DiscoveryKey, PublicKey};

/// [Replication] command.
pub enum Command {
    /// Open a new replica.
    Open(PublicKey, Box<dyn ReplicaTrait + Send>),
    /// Re-open a replica.
    ReOpen(DiscoveryKey),
    /// Close a replica.
    Close(DiscoveryKey),
    /// End the [Replication].
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

/// [Replication] handle.
#[derive(Debug, Clone)]
pub struct ReplicationHandle {
    tx: UnboundedSender<Command>,
}
impl ReplicationHandle {
    /// Create [ReplicationHandle].
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

    /// End the [Replication].
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
