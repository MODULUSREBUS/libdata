#![cfg_attr(test, allow(dead_code))]

use std::path::Path;

use datacore::Keypair;
use index_access_fs::IndexAccessFs;
use index_access_memory::IndexAccessMemory;

pub fn storage_memory() -> IndexAccessMemory {
    IndexAccessMemory::default()
}
pub async fn storage_fs(dir: &Path) -> IndexAccessFs {
    IndexAccessFs::new(dir).await.unwrap()
}

pub fn copy_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_bytes(&keypair.to_bytes()).unwrap()
}
