//! Replication protocol for safely synchronizing [Datacore]s.

mod core_replica;
mod handle;
mod replica_trait;
mod link;

pub use core_replica::CoreReplica;
pub use handle::{Command, LinkHandle};
pub use protocol::{Duplex, Options};
pub use replica_trait::{Data, DataOrRequest, ReplicaTrait, Request};
pub use link::Link;
