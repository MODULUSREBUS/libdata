//! [Core] interface.
//! Defines all the common [Core] interactions.

use anyhow::Result;
use async_trait::async_trait;
use crate::Signature;

/// Common [Core] interactions.
#[async_trait]
pub trait CoreTrait {
    /// Append an entry to the [Core].
    ///
    /// If `signature` is supplied, the caller is responsible for verifying its
    /// integrity and consistency with the `data`.
    async fn append(&mut self, data: &[u8], signature: Option<Signature>) -> Result<()>;

    /// Retrieve data for a block at an index.
    async fn get(&mut self, index: u32) -> Result<Option<(Vec<u8>, Signature)>>;
}
