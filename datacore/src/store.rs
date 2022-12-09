use anyhow::{anyhow, ensure, Result};
use std::mem::size_of;

use crate::block::BLOCK_LENGTH;
use crate::merkle::NODE_SIZE;
use crate::{Block, IndexAccess, Merkle, Node};

const STATE_INDEX: u32 = 0;

/// Save data to a desired storage backend.
pub struct Store<T> {
    store: T,
}
impl<T> Store<T> {
    /// Create a new [Store] from storage interface.
    #[inline]
    pub fn new(store: T) -> Self {
        Self { store }
    }
}
impl<T> Store<T>
where
    T: IndexAccess + Send,
    <T as IndexAccess>::Error: Into<anyhow::Error>,
{
    /// Write data for a `Block`.
    #[inline]
    pub async fn write(&mut self, index: u32, data: &[u8], block: &Block) -> Result<()> {
        let mut bytes = Vec::with_capacity(data.len() + BLOCK_LENGTH);
        bytes.extend_from_slice(data);
        bytes.extend_from_slice(&block.to_bytes()?);
        self.store
            .write(index + 1, &bytes)
            .await
            .map_err(|e| anyhow!(e))
    }

    /// Read data for a `Block`.
    #[inline]
    pub async fn read(&mut self, index: u32) -> Result<Option<(Vec<u8>, Block)>> {
        Ok(
            match self.store.read(index + 1).await.map_err(|e| anyhow!(e))? {
                None => None,
                Some(mut raw) => {
                    ensure!(raw.len() > BLOCK_LENGTH);
                    let block = raw.split_off(raw.len() - BLOCK_LENGTH);
                    Some((raw, Block::from_bytes(&block)?))
                }
            },
        )
    }

    /// Write `Merkle` roots.
    #[inline]
    pub async fn write_merkle(&mut self, merkle: &Merkle) -> Result<()> {
        let roots = merkle.roots();
        let length = roots.len();

        let mut data = Vec::with_capacity(length * NODE_SIZE);
        for node in roots {
            data.extend_from_slice(&node.to_bytes()?);
        }

        self.store
            .write(STATE_INDEX, &data)
            .await
            .map_err(|e| anyhow!(e))
    }

    /// Read roots and reconstruct `Merkle`.
    #[inline]
    pub async fn read_merkle(&mut self) -> Result<Merkle> {
        // try reading length
        let data = self.store.read(STATE_INDEX).await.map_err(|e| anyhow!(e))?;

        // init [Merkle] from roots
        match data {
            // no data => no roots
            None => Ok(Merkle::default()),
            // read roots
            Some(data) => {
                ensure!(data.len() % NODE_SIZE == 0);
                let length = data.len() / NODE_SIZE;

                let mut roots = Vec::with_capacity(length as usize * size_of::<Node>());
                let mut start = 0;
                while start < data.len() {
                    let end = start + NODE_SIZE;
                    let root = Node::from_bytes(&data[start..end])?;
                    roots.push(root);
                    start = end;
                }

                Ok(Merkle::from_roots(roots))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::Signature;
    use crate::ed25519_dalek;
    use crate::hash::Hash;
    use index_access_memory::IndexAccessMemory;

    #[tokio::test]
    async fn init() -> Result<()> {
        Store::new(IndexAccessMemory::default());
        Ok(())
    }

    #[tokio::test]
    async fn data() -> Result<()> {
        let mut store = Store::new(IndexAccessMemory::default());
        let data = b"hello world";
        let signature = Signature::new(
            ed25519_dalek::Signature::from_bytes(&[2u8; ed25519_dalek::SIGNATURE_LENGTH])?,
            ed25519_dalek::Signature::from_bytes(&[7u8; ed25519_dalek::SIGNATURE_LENGTH])?,
        );
        let block = Block::new(1, 8, signature);
        store.write(0, data, &block).await?;
        let (data2, block2) = store.read(0).await?.unwrap();
        assert_eq!(data2, data);
        assert_eq!(block2, block);
        Ok(())
    }

    #[tokio::test]
    async fn merkle() -> Result<()> {
        let mut store = Store::new(IndexAccessMemory::default());
        let mut merkle = Merkle::default();
        merkle.next(Hash::from_leaf(b"a")?, 1);
        merkle.next(Hash::from_leaf(b"b")?, 1);
        merkle.next(Hash::from_leaf(b"c")?, 1);
        store.write_merkle(&merkle).await?;
        let merkle2 = store.read_merkle().await?;
        assert_eq!(merkle.roots(), merkle2.roots());
        Ok(())
    }
}
