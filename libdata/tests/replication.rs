use anyhow::Result;
use async_compat::{Compat, CompatExt};
use futures_lite::future::zip;
use sluice::pipe::{pipe, PipeReader, PipeWriter};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::{task, test, time};

use index_access_memory::IndexAccessMemory;
use libdata::replication::{CoreReplica, Duplex, Handle, Link, Options};
use libdata::{key, keypair, Core};

async fn new_core() -> Result<Core<IndexAccessMemory>> {
    let keypair = keypair::generate();
    Core::new(
        IndexAccessMemory::default(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
}
async fn new_replica(key: key::Public) -> Result<Core<IndexAccessMemory>> {
    Core::new(IndexAccessMemory::default(), key, None).await
}

type Transfer = Duplex<Compat<PipeReader>, Compat<PipeWriter>>;
type Replication = (Link<Transfer>, Handle);

fn create_duplex_pair_memory() -> (Transfer, Transfer) {
    let (ar, bw) = pipe();
    let (br, aw) = pipe();
    (
        Duplex::new(ar.compat(), aw.compat()),
        Duplex::new(br.compat(), bw.compat()),
    )
}
async fn create_replication_pair_memory() -> (Replication, Replication) {
    const KEEPALIVE_MS: u64 = 500;

    let (a_stream, b_stream) = create_duplex_pair_memory();
    let (a, b) = zip(
        task::spawn(async move {
            Link::with_options(
                a_stream,
                Options {
                    is_initiator: false,
                    keepalive_ms: Some(KEEPALIVE_MS),
                    ..Options::default()
                },
            )
            .await
            .unwrap()
        }),
        task::spawn(async move {
            Link::with_options(
                b_stream,
                Options {
                    is_initiator: true,
                    keepalive_ms: Some(KEEPALIVE_MS),
                    ..Options::default()
                },
            )
            .await
            .unwrap()
        }),
    )
    .await;
    (a.unwrap(), b.unwrap())
}

#[test]
async fn replication_core_replica() -> Result<()> {
    let mut a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;

    let data = b"hello world";
    a.append(data, None).await?;

    let a_replica = Box::new(CoreReplica::new(Arc::new(Mutex::new(a))));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));

    let ((a_replication, mut a_handle), (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    let (ra, rb) = zip(
        task::spawn(async move {
            a_handle.open(&public, a_replica).unwrap();
            a_replication.run().await.unwrap();
        }),
        task::spawn(async move {
            b_handle.open(&public, b_replica).unwrap();
            b_replication.run().await.unwrap();
        }),
    )
    .await;
    ra?;
    rb?;

    let mut b = b.lock().await;
    assert_eq!(b.get(0).await?.unwrap().0, data);
    Ok(())
}
#[test]
async fn replication_core_replica_async_open() -> Result<()> {
    let mut a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;

    let data = b"hello world";
    a.append(data, None).await?;

    let a_replica = Box::new(CoreReplica::new(Arc::new(Mutex::new(a))));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));

    let ((a_replication, mut a_handle), (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    let ((ra, rb), (rc, rd)) = zip(
        zip(
            task::spawn(async move {
                a_replication.run().await.unwrap();
            }),
            task::spawn(async move {
                b_replication.run().await.unwrap();
            }),
        ),
        zip(
            task::spawn(async move {
                a_handle.open(&public, a_replica).unwrap();
            }),
            task::spawn(async move {
                b_handle.open(&public, b_replica).unwrap();
            }),
        ),
    )
    .await;
    ra?;
    rb?;
    rc?;
    rd?;

    let mut b = b.lock().await;
    assert_eq!(b.get(0).await?.unwrap().0, data);
    Ok(())
}

#[test]
async fn replication_core_replica_multiple_blocks() -> Result<()> {
    let mut a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;

    let data = b"hello world";
    for &d in data.into_iter() {
        a.append(&[d], None).await?;
    }

    let a_replica = Box::new(CoreReplica::new(Arc::new(Mutex::new(a))));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));

    let ((a_replication, mut a_handle), (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    let (ra, rb) = zip(
        task::spawn(async move {
            a_handle.open(&public, a_replica).unwrap();
            a_replication.run().await
        }),
        task::spawn(async move {
            b_handle.open(&public, b_replica).unwrap();
            b_replication.run().await
        }),
    )
    .await;
    ra??;
    rb??;

    let mut b = b.lock().await;
    for (i, &d) in data.into_iter().enumerate() {
        assert_eq!(b.get(i as u32).await?.unwrap().0[0], d);
    }
    Ok(())
}

#[test]
async fn replication_core_replica_multiple_blocks_live() -> Result<()> {
    let a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;

    let data = b"hello world";

    let a = Arc::new(Mutex::new(a));
    let a_replica = Box::new(CoreReplica::new(Arc::clone(&a)));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));

    let ((a_replication, mut a_handle), (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    let ((ra, rb), (rc, rd)) = zip(
        zip(
            task::spawn(async move {
                a_replication.run().await.unwrap();
            }),
            task::spawn(async move {
                b_replication.run().await.unwrap();
            }),
        ),
        zip(
            task::spawn(async move {
                a_handle.open(&public, a_replica).unwrap();
                for &d in data.into_iter() {
                    let mut a = a.lock().await;
                    a.append(&[d], None).await.unwrap();
                    a_handle.reopen(&public).unwrap();
                    time::sleep(Duration::from_millis(10)).await;
                }
            }),
            task::spawn(async move {
                b_handle.open(&public, b_replica).unwrap();
            }),
        ),
    )
    .await;
    ra?;
    rb?;
    rc?;
    rd?;

    let mut b = b.lock().await;
    for (i, &d) in data.into_iter().enumerate() {
        assert_eq!(b.get(i as u32).await?.unwrap().0[0], d);
    }
    Ok(())
}

#[test]
async fn replication_core_replica_of_replica() -> Result<()> {
    let mut a = new_core().await?;
    let public = a.public_key().clone();
    let b = new_replica(public.clone()).await?;
    let c = new_replica(public.clone()).await?;

    let data = b"hello world";
    a.append(data, None).await?;

    let a_replica = Box::new(CoreReplica::new(Arc::new(Mutex::new(a))));
    let b = Arc::new(Mutex::new(b));
    let b_replica = Box::new(CoreReplica::new(Arc::clone(&b)));
    let b2_replica = Box::new(CoreReplica::new(Arc::clone(&b)));
    let c = Arc::new(Mutex::new(c));
    let c_replica = Box::new(CoreReplica::new(Arc::clone(&c)));

    let ((a_replication, mut a_handle), (b_replication, mut b_handle)) =
        create_replication_pair_memory().await;
    let (ra, rb) = zip(
        task::spawn(async move {
            a_handle.open(&public, a_replica).unwrap();
            a_replication.run().await.unwrap();
        }),
        task::spawn(async move {
            b_handle.open(&public, b_replica).unwrap();
            b_replication.run().await.unwrap();
        }),
    )
    .await;
    ra?;
    rb?;

    let ((b2_replication, mut b2_handle), (c_replication, mut c_handle)) =
        create_replication_pair_memory().await;
    let (ra, rb) = zip(
        task::spawn(async move {
            b2_handle.open(&public, b2_replica).unwrap();
            b2_replication.run().await.unwrap();
        }),
        task::spawn(async move {
            c_handle.open(&public, c_replica).unwrap();
            c_replication.run().await.unwrap();
        }),
    )
    .await;
    ra?;
    rb?;

    let mut c = c.lock().await;
    assert_eq!(c.get(0).await?.unwrap().0, data);
    Ok(())
}
