use std::fs::File;
use std::io::Read;
use std::path::Path;

use datacore::{sign, verify, Core, Hash, KeyPair, Merkle, NodeTrait, Signature};
use index_access_fs::IndexAccessFs;

fn read_bytes(dir: &Path, s: &str) -> Vec<u8> {
    let mut f = File::open(dir.join(s)).unwrap();
    let mut bytes = Vec::new();
    f.read_to_end(&mut bytes).unwrap();
    bytes
}

fn hash_tree(merkle: &Merkle) -> Hash {
    let roots = merkle.roots();
    let hashes = roots.iter().map(|root| root.hash()).collect::<Vec<&Hash>>();
    let lengths = roots.iter().map(|root| root.length()).collect::<Vec<u32>>();
    Hash::from_roots(&hashes, &lengths)
}

#[tokio::test]
pub async fn replicate_manual() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();

    let keypair = KeyPair::generate();
    let keypair2 = keypair.clone();
    let keypair3 = keypair.clone();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.pk,
        Some(keypair.sk),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.pk,
        Some(keypair2.sk),
    )
    .await
    .unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    assert_eq!(core.len(), 2);

    let mut merkle = Merkle::default();
    let data_hash = Hash::from_leaf(data1).unwrap();
    let data_sign = sign(&keypair3.sk, &data_hash);
    merkle.next(data_hash.clone(), data1.len() as u32);
    verify(&keypair3.pk, &data_hash, &data_sign).unwrap();
    let tree_hash = hash_tree(&merkle);
    let tree_sign = sign(&keypair3.sk, &tree_hash);
    verify(&keypair3.pk, &tree_hash, &tree_sign).unwrap();
    let signature = Signature::new(data_sign, tree_sign);
    replica.append(data1, Some(signature)).await.unwrap();
    let data_hash = Hash::from_leaf(data2).unwrap();
    merkle.next(data_hash.clone(), data2.len() as u32);
    let signature = Signature::new(
        sign(&keypair3.sk, &data_hash),
        sign(&keypair3.sk, &hash_tree(&merkle)),
    );
    replica.append(data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    assert_eq!(read_bytes(&dir2, "0"), read_bytes(&dir, "0"));
    assert_eq!(read_bytes(&dir2, "1"), read_bytes(&dir, "1"));
    assert_eq!(read_bytes(&dir2, "2"), read_bytes(&dir, "2"));
}

#[tokio::test]
pub async fn replicate_manual_no_secret_key() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = KeyPair::generate();
    let keypair2 = KeyPair::from_slice(keypair.as_slice()).unwrap();
    let keypair3 = KeyPair::from_slice(keypair.as_slice()).unwrap();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.pk,
        Some(keypair.sk),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.pk,
        Some(keypair2.sk),
    )
    .await
    .unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    assert_eq!(core.len(), 2);

    let mut merkle = Merkle::default();
    let data_hash = Hash::from_leaf(data1).unwrap();
    merkle.next(data_hash.clone(), data1.len() as u32);
    let signature = Signature::new(
        sign(&keypair3.sk, &data_hash),
        sign(&keypair3.sk, &hash_tree(&merkle)),
    );
    replica.append(data1, Some(signature)).await.unwrap();
    let data_hash = Hash::from_leaf(data2).unwrap();
    merkle.next(data_hash.clone(), data2.len() as u32);
    let signature = Signature::new(
        sign(&keypair3.sk, &data_hash),
        sign(&keypair3.sk, &hash_tree(&merkle)),
    );
    replica.append(data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    assert_eq!(read_bytes(&dir2, "0"), read_bytes(&dir, "0"));
    assert_eq!(read_bytes(&dir2, "1"), read_bytes(&dir, "1"));
    assert_eq!(read_bytes(&dir2, "2"), read_bytes(&dir, "2"));
}

#[tokio::test]
pub async fn replicate_signatures_no_secret_key() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = KeyPair::generate();
    let keypair2 = keypair.clone();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.pk,
        Some(keypair.sk),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.pk,
        Some(keypair2.sk),
    )
    .await
    .unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    assert_eq!(core.len(), 2);

    let (data1, signature) = core.get(0).await.unwrap().unwrap();
    replica.append(&data1, Some(signature)).await.unwrap();
    let (data2, signature) = core.get(1).await.unwrap().unwrap();
    replica.append(&data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    assert_eq!(read_bytes(&dir2, "0"), read_bytes(&dir, "0"));
    assert_eq!(read_bytes(&dir2, "1"), read_bytes(&dir, "1"));
    assert_eq!(read_bytes(&dir2, "2"), read_bytes(&dir, "2"));
}

#[tokio::test]
pub async fn replicate_then_append() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = KeyPair::generate();
    let keypair2 = keypair.clone();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.pk,
        Some(keypair.sk),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.pk,
        Some(keypair2.sk),
    )
    .await
    .unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";
    let data3 = b"THIS WILL NOT BE REPLICATED";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    core.append(data3, None).await.unwrap();
    assert_eq!(core.len(), 3);

    let (data1, signature) = core.get(0).await.unwrap().unwrap();
    replica.append(&data1, Some(signature)).await.unwrap();
    let (data2, signature) = core.get(1).await.unwrap().unwrap();
    replica.append(&data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    replica.append(data3, None).await.unwrap();
    assert_eq!(replica.len(), 3);

    assert_eq!(read_bytes(&dir2, "0"), read_bytes(&dir, "0"));
    assert_eq!(read_bytes(&dir2, "1"), read_bytes(&dir, "1"));
    assert_eq!(read_bytes(&dir2, "2"), read_bytes(&dir, "2"));
    assert_eq!(read_bytes(&dir2, "3"), read_bytes(&dir, "3"));
}

#[tokio::test]
pub async fn replicate_fail_verify_then_append() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = KeyPair::generate();
    let keypair2 = keypair.clone();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.pk,
        Some(keypair.sk),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.pk,
        Some(keypair2.sk),
    )
    .await
    .unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";
    let data3 = b"THIS WILL NOT BE REPLICATED";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    core.append(data3, None).await.unwrap();
    assert_eq!(core.len(), 3);

    let (data1, signature) = core.get(0).await.unwrap().unwrap();
    replica.append(&data1, Some(signature)).await.unwrap();
    let (data2, signature) = core.get(1).await.unwrap().unwrap();
    let invalid_signature_1 = Signature::new(
        *signature.data(),
        ed25519_compact::Signature::from_slice(&[0u8; ed25519_compact::Signature::BYTES]).unwrap(),
    );
    let invalid_signature_2 = Signature::new(
        ed25519_compact::Signature::from_slice(&[0u8; ed25519_compact::Signature::BYTES]).unwrap(),
        ed25519_compact::Signature::from_slice(&[0u8; ed25519_compact::Signature::BYTES]).unwrap(),
    );
    let invalid_signature_3 = Signature::new(
        ed25519_compact::Signature::from_slice(&[0u8; ed25519_compact::Signature::BYTES]).unwrap(),
        *signature.tree(),
    );
    assert!(replica
        .append(&data2, Some(invalid_signature_1))
        .await
        .is_err());
    assert!(replica
        .append(&data2, Some(invalid_signature_2))
        .await
        .is_err());
    assert!(replica
        .append(&data2, Some(invalid_signature_3))
        .await
        .is_err());
    replica.append(&data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    replica.append(data3, None).await.unwrap();
    assert_eq!(replica.len(), 3);

    assert_eq!(read_bytes(&dir2, "1"), read_bytes(&dir, "1"));
    assert_eq!(read_bytes(&dir2, "2"), read_bytes(&dir, "2"));
}
