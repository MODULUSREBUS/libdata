use anyhow::{anyhow, ensure, Result};
use std::mem::size_of;

use crate::merkle::NODE_SIZE;
use crate::{IndexAccess, Merkle, Node};

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
    pub async fn write(&mut self, index: u32, data: &[u8]) -> Result<()> {
        self.store
            .write(index + 1, data)
            .await
            .map_err(|e| anyhow!(e))
    }

    /// Read data for a `Block`.
    #[inline]
    pub async fn read(&mut self, index: u32) -> Result<Option<Vec<u8>>> {
        self.store.read(index + 1).await.map_err(|e| anyhow!(e))
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
        store.write(0, data).await?;
        let read = store.read(0).await?.unwrap();
        assert_eq!(read, data);
        Ok(())
    }

    #[tokio::test]
    async fn merkle() -> Result<()> {
        let mut store = Store::new(IndexAccessMemory::default());
        let mut merkle = Merkle::default();
        merkle.next(Hash::from_leaf(b"a"), 1);
        merkle.next(Hash::from_leaf(b"b"), 1);
        merkle.next(Hash::from_leaf(b"c"), 1);
        store.write_merkle(&merkle).await?;
        let merkle2 = store.read_merkle().await?;
        assert_eq!(merkle.roots(), merkle2.roots());
        Ok(())
    }
}
