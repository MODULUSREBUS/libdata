mod common;
use common::storage_fs;

use insta;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tempfile;
use tokio::test;

use datacore::{Core, Keypair};

fn read_bytes(dir: &Path, s: &str) -> Vec<u8> {
    let mut f = File::open(dir.join(s)).unwrap();
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).unwrap();
    bytes
}

const KEYPAIR_BYTES: [u8; 64] = [
    86, 29, 202, 51, 72, 159, 192, 155, 76, 131, 249, 122, 241, 244, 9, 244, 157, 139, 190, 59,
    130, 201, 224, 241, 121, 161, 171, 30, 158, 108, 23, 0, 184, 16, 141, 118, 116, 37, 127, 146,
    105, 139, 107, 124, 101, 123, 166, 152, 83, 209, 170, 236, 172, 23, 111, 253, 30, 197, 241, 83,
    169, 233, 237, 77,
];

#[test]
pub async fn snapshots_append() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let keypair = Keypair::from_bytes(&KEYPAIR_BYTES).unwrap();
    let mut core = Core::new(
        storage_fs(&dir.to_path_buf().join("store")).await,
        storage_fs(&dir.to_path_buf().join("blocks")).await,
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    let data = b"abcdef";
    for &b in data {
        core.append(&[b], None).await.unwrap();
    }
    assert_eq!(core.len() as usize, data.len());

    let mut store = Vec::new();
    let mut blocks_store = Vec::new();
    for (i, _) in data.iter().enumerate() {
        let s = read_bytes(&dir, &format!("store/{}", i + 1));
        let b = read_bytes(&dir, &format!("blocks/{}", i + 1));
        store.extend(s);
        blocks_store.extend(b);
    }
    let merkle = read_bytes(&dir, &format!("store/0"));

    insta::assert_debug_snapshot!(blocks_store);
    insta::assert_debug_snapshot!(store);
    insta::assert_debug_snapshot!(merkle);
}
