use anyhow::{anyhow, Result};
use std::error::Error;
use std::fmt::Debug;

use crate::IndexAccess;

/// Save data to a desired storage backend.
#[derive(Debug)]
pub struct StoreData<T>
where
    T: Debug,
{
    store: T,
}
impl<T> StoreData<T>
where
    T: Debug,
{
    /// Create a new [StoreData] from storage interface.
    #[inline]
    pub fn new(store: T) -> Self {
        Self { store }
    }
}
impl<T> StoreData<T>
where
    T: IndexAccess<Error = Box<dyn Error + Send + Sync>> + Debug + Send,
{
    /// Write data for a `Block`.
    #[inline]
    pub async fn write(
        &mut self,
        index: u32,
        data: &[u8],
        ) -> Result<()>
    {
        self.store
            .write(index.to_string(), &data)
            .await.map_err(|e| anyhow!(e))
    }

    /// Read data for a `Block`.
    #[inline]
    pub async fn read(
        &mut self,
        index: u32,
        ) -> Result<Vec<u8>>
    {
        self.store
            .read(index.to_string())
            .await.map_err(|e| anyhow!(e))
    }
}

#[cfg(test)]
mod tests {
    use tokio::test;
    use index_access_memory::IndexAccessMemory;
    use super::*;

    fn iam() -> IndexAccessMemory {
        IndexAccessMemory::new()
    }

    #[test]
    pub async fn init() -> Result<()> {
        StoreData::new(iam());
        Ok(())
    }

    #[test]
    pub async fn write_read() -> Result<()> {
        let mut store = StoreData::new(iam());
        let msg = "hello world".as_bytes();
        store.write(0, msg).await?;
        let msg2 = store.read(0).await?;
        assert_eq!(msg, msg2);
        Ok(())
    }
}
