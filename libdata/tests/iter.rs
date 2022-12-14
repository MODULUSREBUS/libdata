use anyhow::Result;
use futures_lite::stream::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::test;

use index_access_memory::IndexAccessMemory;
use libdata::{keypair, Core, CoreIterator};

#[test]
async fn iter_simple() -> Result<()> {
    let keypair = keypair::generate();
    let mut core = Core::new(
        IndexAccessMemory::default(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    let data = vec![1, 2, 3];
    for d in data {
        core.append(&[d], None).await.unwrap();
    }

    let mut iter = CoreIterator::new(Arc::new(Mutex::new(core)), 0);
    assert_eq!(iter.next().await.unwrap(), (0, vec![1]));
    assert_eq!(iter.next().await.unwrap(), (1, vec![2]));
    assert_eq!(iter.next().await.unwrap(), (2, vec![3]));
    assert_eq!(iter.next().await, None);
    Ok(())
}

#[test]
async fn iter_offset() -> Result<()> {
    let keypair = keypair::generate();
    let mut core = Core::new(
        IndexAccessMemory::default(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    let data = vec![1, 2, 3];
    for d in data {
        core.append(&[d], None).await.unwrap();
    }

    let mut iter = CoreIterator::new(Arc::new(Mutex::new(core)), 1);
    assert_eq!(iter.next().await.unwrap(), (1, vec![2]));
    assert_eq!(iter.next().await.unwrap(), (2, vec![3]));
    assert_eq!(iter.next().await, None);
    Ok(())
}

#[test]
async fn iter_out_of_bounds() -> Result<()> {
    let keypair = keypair::generate();
    let mut core = Core::new(
        IndexAccessMemory::default(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    let data = vec![1, 2, 3];
    for d in data {
        core.append(&[d], None).await.unwrap();
    }

    let mut iter = CoreIterator::new(Arc::new(Mutex::new(core)), 100);
    assert_eq!(iter.next().await, None);
    Ok(())
}
