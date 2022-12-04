use anyhow::Result;
use tokio::test;

use index_access_memory::IndexAccessMemory;
use libdata::{Core, Cores, generate_keypair, discovery_key};

type CoreIAM =
    Core<IndexAccessMemory, IndexAccessMemory, IndexAccessMemory>;

pub fn storage_memory() -> IndexAccessMemory {
    IndexAccessMemory::new()
}
async fn new_core() -> Result<CoreIAM>
{
    let keypair = generate_keypair();
    Core::new(
        storage_memory(),
        storage_memory(),
        storage_memory(),
        keypair.public, Some(keypair.secret))
        .await
}

#[test]
async fn cores_insert_get() -> Result<()>
{
    let a = new_core().await?;
    let a_public = a.public_key().clone();
    let b = new_core().await?;

    let mut cores = Cores::default();
    cores.insert(a);

    assert!(cores.get_by_public(&a_public).is_some());
    assert!(cores.get_by_public(&b.public_key()).is_none());

    assert!(cores.get_by_discovery(
            &discovery_key(&a_public.to_bytes())).is_some());
    assert!(cores.get_by_discovery(
            &discovery_key(&b.public_key().to_bytes())).is_none());

    assert_eq!(cores.public_keys().len(), 1);
    assert_eq!(cores.discovery_keys().len(), 1);

    Ok(())
}

#[test]
async fn cores_insert_2() -> Result<()>
{
    let a = new_core().await?;
    let a_public = a.public_key().clone();
    let b = new_core().await?;
    let b_public = b.public_key().clone();

    let mut cores = Cores::default();
    cores.insert(a);
    cores.insert(b);

    assert!(cores.get_by_public(&a_public).is_some());
    assert!(cores.get_by_public(&b_public).is_some());

    assert!(cores.get_by_discovery(
            &discovery_key(&a_public.to_bytes())).is_some());
    assert!(cores.get_by_discovery(
            &discovery_key(&b_public.to_bytes())).is_some());

    assert_eq!(cores.public_keys().len(), 2);
    assert_eq!(cores.discovery_keys().len(), 2);

    Ok(())
}
