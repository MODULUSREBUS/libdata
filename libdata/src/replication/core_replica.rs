use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::replication::{Data, DataOrRequest, ReplicaTrait, Request};
use crate::{ed25519_dalek, Signature, Core, IndexAccess, MAX_CORE_LENGTH};

/// CoreReplica describes eager, full, and sequential synchronization logic
/// for replicating [Core] over [Link].
pub struct CoreReplica<T> {
    core: Arc<Mutex<Core<T>>>,
    remote_index: Option<u32>,
}

impl<T> CoreReplica<T> {
    /// Create a new [CoreReplica].
    #[must_use]
    pub fn new(core: Arc<Mutex<Core<T>>>) -> Self {
        Self {
            core,
            remote_index: None,
        }
    }

    fn update_remote_index(&mut self, index: u32) {
        if let Some(old_index) = self.remote_index {
            if index <= old_index {
                return;
            }
        }
        self.remote_index = Some(index);
    }
}
#[async_trait]
impl<T> ReplicaTrait for CoreReplica<T>
where
    T: IndexAccess + Send,
    <T as IndexAccess>::Error: Into<anyhow::Error>,
{
    async fn on_open(&mut self) -> Result<Option<Request>> {
        let core = self.core.lock().await;
        let request = Request { index: core.len() };
        Ok(Some(request))
    }
    async fn on_request(&mut self, request: Request) -> Result<Option<DataOrRequest>> {
        self.update_remote_index(request.index);

        let mut core = self.core.lock().await;
        let data = core.get(request.index).await?;
        Ok(
            if let Some((data, signature)) = data {
                let response = Data {
                    index: request.index,
                    data,
                    data_signature: signature.data().to_bytes().to_vec(),
                    tree_signature: signature.tree().to_bytes().to_vec(),
                };
                Some(DataOrRequest::Data(response))
            } else {
                let index = core.len();
                let remote_index = self.remote_index.unwrap_or(0);
                if index as usize >= MAX_CORE_LENGTH || remote_index <= index {
                    None
                } else {
                    let response = Request { index };
                    Some(DataOrRequest::Request(response))
                }
            }
        )
    }
    async fn on_data(&mut self, data: Data) -> Result<Option<Request>> {
        let mut core = self.core.lock().await;
        let len = core.len();
        if data.index == len {
            let signature = Signature::new(
                ed25519_dalek::Signature::from_bytes(&data.data_signature).unwrap(),
                ed25519_dalek::Signature::from_bytes(&data.tree_signature).unwrap(),
            );
            core.append(&data.data, Some(signature)).await?;

            if core.len() as usize >= MAX_CORE_LENGTH {
                Ok(None)
            } else {
                Ok(Some(Request {
                    index: data.index + 1,
                }))
            }
        } else {
            Ok(Some(Request { index: len }))
        }
    }
    async fn on_close(&mut self) -> Result<()> {
        if let Some(index) = self.remote_index {
            let core = self.core.lock().await;
            let len = core.len();

            if len < index {
                return Err(anyhow!("Not synced; remote has more data."));
            }
        }
        Ok(())
    }
}
