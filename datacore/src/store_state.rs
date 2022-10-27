use anyhow::{anyhow, ensure, Result};
use std::mem::size_of;
use std::error::Error;
use std::fmt::Debug;

use crate::merkle::NODE_SIZE;
use crate::{Merkle, Node, IndexAccess};

/// Save data to a desired storage backend.
#[derive(Debug)]
pub struct StoreState<T>
where
    T: Debug,
{
    store: T,
}
impl<T> StoreState<T>
where
    T: Debug,
{
    /// Create a new [StoreState] from storage interface.
    #[inline]
    pub fn new(store: T) -> Self {
        Self { store }
    }
}
impl<T> StoreState<T>
where
    T: IndexAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
{
    /// Write `Merkle` roots.
    #[inline]
    pub async fn write(
        &mut self,
        merkle: &Merkle,
        ) -> Result<()>
    {
        let roots = merkle.roots();
        let length = roots.len();

        let mut data = Vec::with_capacity(length * NODE_SIZE);
        for node in roots {
            data.extend_from_slice(&node.to_bytes()?);
        }

        self.store
            .write("state".to_owned(), &data)
            .await.map_err(|e| anyhow!(e))
    }

    /// Read roots and reconstruct `Merkle`.
    #[inline]
    pub async fn read(
        &mut self,
        ) -> Result<Merkle>
    {
        // try reading length
        let data = self.store
            .read("state".to_owned())
            .await.map_err(|e| anyhow!(e));

        // init [Merkle] from roots
        let roots = match data {
            // no data => no roots
            Err(_) => vec![],
            // read roots
            Ok(data) => {
                ensure!(data.len() % NODE_SIZE == 0);
                let length = data.len() / NODE_SIZE;

                let mut roots = Vec::with_capacity(
                    length as usize * size_of::<Node>());

                let mut start = 0;
                while start < data.len() {
                    let end = start + NODE_SIZE;
                    let root = Node::from_bytes(&data[start..end])?;
                    roots.push(root);
                    start = end;
                }
                roots
            },
        };
        Ok(Merkle::from_roots(roots))
    }
}

#[cfg(test)]
mod tests {
    use tokio::test;
    use index_access_memory::IndexAccessMemory;
    use crate::hash::Hash;
    use super::*;

    fn iam() -> IndexAccessMemory {
        IndexAccessMemory::new()
    }

    #[test]
    pub async fn init() -> Result<()> {
        StoreState::new(iam());
        Ok(())
    }

    #[test]
    pub async fn write_read() -> Result<()> {
        let mut store = StoreState::new(iam());
        let mut merkle = Merkle::new();
        merkle.next(Hash::from_leaf(b"a"), 1);
        merkle.next(Hash::from_leaf(b"b"), 1);
        merkle.next(Hash::from_leaf(b"c"), 1);
        store.write(&merkle).await?;
        let merkle2 = store.read().await?;
        assert_eq!(merkle.roots(), merkle2.roots());
        Ok(())
    }
}
