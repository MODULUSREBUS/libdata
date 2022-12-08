use anyhow::{anyhow, ensure, Result};

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
    T: IndexAccess + Send,
    <T as IndexAccess>::Error: Into<anyhow::Error>,
{
    /// Write a `Block`.
    #[inline]
    pub async fn write(&mut self, index: u32, block: &Block) -> Result<()> {
        let data = block.to_bytes()?;
        ensure!(data.len() == BLOCK_LENGTH as usize);

        self.store
            .write(index + 1, &data)
            .await
            .map_err(|e| anyhow!(e))
    }

    /// Read a `Block`.
    #[inline]
    pub async fn read(&mut self, index: u32) -> Result<Option<Block>> {
        let data = self
            .store
            .read(index + 1)
            .await
            .map_err(|e| anyhow!(e))?;

        match data {
            Some(data) => {
                ensure!(data.len() == BLOCK_LENGTH as usize);
                Ok(Some(Block::from_bytes(&data)?))
            },
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::{BlockSignature, Signature, SIGNATURE_LENGTH};
    use index_access_memory::IndexAccessMemory;

    #[tokio::test]
    pub async fn init() -> Result<()> {
        StoreBlocks::new(IndexAccessMemory::default());
        Ok(())
    }

    #[tokio::test]
    pub async fn write_read() -> Result<()> {
        let mut store = StoreBlocks::new(IndexAccessMemory::default());
        let data = Signature::from_bytes(&[2u8; SIGNATURE_LENGTH])?;
        let tree = Signature::from_bytes(&[7u8; SIGNATURE_LENGTH])?;
        let signature = BlockSignature::new(data, tree);
        let block = Block::new(1, 8, signature);
        store.write(0, &block).await?;
        let block2 = store.read(0).await?.unwrap();
        assert_eq!(block, block2);
        Ok(())
    }
}
