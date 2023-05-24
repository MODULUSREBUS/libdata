use anyhow::Result;
use tokio::test;

use index_access_memory::IndexAccessMemory;
use libdata::{key, KeyPair, Core, Cores};

async fn new_core() -> Result<Core<IndexAccessMemory>> {
    let keypair = KeyPair::generate();
    Core::new(
        IndexAccessMemory::default(),
        keypair.pk,
        Some(keypair.sk),
    )
    .await
}

#[test]
async fn cores_insert_get() -> Result<()> {
    let a = new_core().await?;
    let a_public = a.public_key().clone();
    let b = new_core().await?;

    let mut cores = Cores::default();
    cores.insert(a);

    assert!(cores.get_by_public(&a_public).is_some());
    assert!(cores.get_by_public(&b.public_key()).is_none());

    assert!(cores
        .get_by_discovery(&key::discovery(a_public.as_slice().try_into().unwrap()))
        .is_some());
    assert!(cores
        .get_by_discovery(&key::discovery(b.public_key().as_slice().try_into().unwrap()))
        .is_none());

    assert_eq!(cores.public_keys().count(), 1);
    assert_eq!(cores.discovery_keys().count(), 1);

    Ok(())
}

#[test]
async fn cores_insert_2() -> Result<()> {
    let a = new_core().await?;
    let a_public = a.public_key().clone();
    let b = new_core().await?;
    let b_public = b.public_key().clone();

    let mut cores = Cores::default();
    cores.insert(a);
    cores.insert(b);

    assert!(cores.get_by_public(&a_public).is_some());
    assert!(cores.get_by_public(&b_public).is_some());

    assert!(cores
        .get_by_discovery(&key::discovery(&a_public.as_slice().try_into().unwrap()))
        .is_some());
    assert!(cores
        .get_by_discovery(&key::discovery(&b_public.as_slice().try_into().unwrap()))
        .is_some());

    assert_eq!(cores.public_keys().count(), 2);
    assert_eq!(cores.discovery_keys().count(), 2);

    Ok(())
}
