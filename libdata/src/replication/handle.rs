use anyhow::{Result, anyhow};
use std::fmt::Debug;
use tokio::sync::mpsc::UnboundedSender;

use crate::{DiscoveryKey, PublicKey, discovery_key};
use crate::replication::ReplicaTrait;

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
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>)
        -> Result<(), std::fmt::Error>
    {
        match self {
            Self::Open(key, _) =>
                write!(fmt, "Command::Open({:?})", key),
            Self::ReOpen(key) =>
                write!(fmt, "Command::ReOpen({:?})", key),
            Self::Close(key) =>
                write!(fmt, "Command::Close({:?})", key),
            Self::Quit() =>
                write!(fmt, "Command::Quit()"),
        }
    }
}

/// [Replication] handle.
#[derive(Debug, Clone)]
pub struct ReplicationHandle {
    pub(crate) tx: UnboundedSender<Command>,
}
impl ReplicationHandle {
    /// Open a new channel with [ReplicaTrait].
    pub fn open(
        &mut self,
        key: &PublicKey,
        replica: Box<dyn ReplicaTrait + Send>,
        ) -> Result<()>
    {
        let cmd = Command::Open(*key, replica);
        self.tx.send(cmd).map_err(|_| anyhow!("Error sending command."))
    }

    /// Reopen a replica.
    pub fn reopen(&mut self, key: &PublicKey) -> Result<()> {
        let cmd = Command::ReOpen(discovery_key(key.as_bytes()));
        self.tx.send(cmd).map_err(|_| anyhow!("Error sending command."))
    }

    /// Close a channel by [DiscoveryKey].
    pub fn close(&mut self, key: DiscoveryKey) -> Result<()> {
        let cmd = Command::Close(key);
        self.tx.send(cmd).map_err(|_| anyhow!("Error sending command."))
    }

    /// End the [Replication].
    pub fn quit(&mut self) -> Result<()> {
        let cmd = Command::Quit();
        self.tx.send(cmd).map_err(|_| anyhow!("Error sending command."))
    }
}
