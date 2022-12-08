use anyhow::Result;
use futures_lite::future::FutureExt;
use futures_lite::stream::Stream;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;

use crate::{BlockSignature, Core, IndexAccess};

/// Async [Stream] iterator over [Core].
pub struct CoreIterator<T, B>
where
    T: Send,
    B: Send,
{
    core: Arc<Mutex<Core<T, B>>>,
    task: Pin<Box<dyn Future<Output = (u32, Option<Vec<u8>>)>>>,
}
impl<T: 'static, B: 'static> CoreIterator<T, B>
where
    T: IndexAccess<Error = Box<dyn Error + Send + Sync>> + Send,
    B: IndexAccess<Error = Box<dyn Error + Send + Sync>> + Send,
{
    /// Create a new [CoreIterator].
    pub fn new(core: Arc<Mutex<Core<T, B>>>, index: u32) -> Self {
        let task = Self::create_read_task(Arc::clone(&core), index);
        Self { core, task }
    }

    #[inline]
    fn create_read_task(
        core: Arc<Mutex<Core<T, B>>>,
        index: u32,
    ) -> Pin<Box<dyn Future<Output = (u32, Option<Vec<u8>>)>>> {
        async move {
            let result: Result<Option<(Vec<u8>, BlockSignature)>>;
            {
                let mut core = core.lock().await;
                result = core.get(index).await;
            }
            if let Ok(Some(data)) = result {
                (index, Some(data.0))
            } else {
                (index, None)
            }
        }
        .boxed()
    }
}
impl<T: 'static, B: 'static> Stream for CoreIterator<T, B>
where
    T: IndexAccess<Error = Box<dyn Error + Send + Sync>> + Send,
    B: IndexAccess<Error = Box<dyn Error + Send + Sync>> + Send,
{
    type Item = (u32, Vec<u8>);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if let Poll::Ready((index, data)) = Pin::new(&mut this.task).poll(cx) {
            this.task = Self::create_read_task(Arc::clone(&this.core), index + 1);
            return Poll::Ready(data.map(|data| (index, data)));
        }
        Poll::Pending
    }
}
