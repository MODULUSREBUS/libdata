//! Replication protocol for safely synchronizing logs.

pub use protocol::{Duplex, Options};

mod replication;
pub use replication::Replication;

mod handle;
pub use handle::{Command, ReplicationHandle};

mod replica_trait;
pub use replica_trait::{Data, DataOrRequest, ReplicaTrait, Request};

mod core_replica;
pub use core_replica::CoreReplica;
