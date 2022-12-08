use anyhow::{anyhow, ensure, Result};
use std::error::Error;

use crate::block::BLOCK_LENGTH;
use crate::{Block, IndexAccess};

/// Save data to a desired storage backend.
pub struct StoreBlocks<T> {
    store: T,
}
impl<T> StoreBlocks<T> {
    /// Create a new [StoreBlocks] from storage interface.
    #[inline]
    pub fn new(store: T) -> Self {
        Self { store }
    }
}
impl<T> StoreBlocks<T>
where
    T: IndexAccess<Error = Box<dyn Error + Send + Sync>> + Send,
{
    /// Write a `Block`.
    #[inline]
    pub async fn write(
        &mut self,
        index: u32,
        block: &Block,
        ) -> Result<()>
    {
        let data = block.to_bytes()?;
        ensure!(data.len() == BLOCK_LENGTH as usize);

        self.store
            .write((index + 1).to_string(), &data)
            .await.map_err(|e| anyhow!(e))
    }

    /// Read a `Block`.
    #[inline]
    pub async fn read(
        &mut self,
        index: u32,
        ) -> Result<Block>
    {
        let data = self.store
            .read((index + 1).to_string())
            .await.map_err(|e| anyhow!(e))?;
        ensure!(data.len() == BLOCK_LENGTH as usize);

        Block::from_bytes(&data)
    }
}

#[cfg(test)]
mod tests {
    use tokio::test;
    use index_access_memory::IndexAccessMemory;
    use crate::block::{Signature, BlockSignature, SIGNATURE_LENGTH};
    use super::*;

    fn iam() -> IndexAccessMemory {
        IndexAccessMemory::new()
    }

    #[test]
    pub async fn init() -> Result<()> {
        StoreBlocks::new(iam());
        Ok(())
    }

    #[test]
    pub async fn write_read() -> Result<()> {
        let mut store = StoreBlocks::new(iam());
        let data = Signature::from_bytes(&[2u8; SIGNATURE_LENGTH])?;
        let tree = Signature::from_bytes(&[7u8; SIGNATURE_LENGTH])?;
        let signature = BlockSignature::new(data, tree);
        let block = Block::new(1, 8, signature);
        store.write(0, &block).await?;
        let block2 = store.read(0).await?;
        assert_eq!(block, block2);
        Ok(())
    }
}
