use std::fs::File;
use std::io::Read;
use std::path::Path;
use tempfile;
use tokio::test;

use datacore::{
    generate_keypair, sign, verify, BlockSignature, Core, Hash, Keypair, Merkle, NodeTrait,
    Signature, SIGNATURE_LENGTH,
};
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
    let lengths = roots.iter().map(|root| root.length()).collect::<Vec<u64>>();
    Hash::from_roots(&hashes, &lengths)
}

#[test]
pub async fn replicate_manual() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = Keypair::from_bytes(&keypair.to_bytes()).unwrap();
    let keypair3 = Keypair::from_bytes(&keypair.to_bytes()).unwrap();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.public,
        Some(keypair2.secret),
    )
    .await
    .unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    assert_eq!(core.len(), 2);

    let mut merkle = Merkle::default();
    let data_hash = Hash::from_leaf(data1);
    let data_sign = sign(&keypair3.public, &keypair3.secret, &data_hash);
    merkle.next(data_hash.clone(), data1.len() as u64);
    verify(&keypair3.public, &data_hash, &data_sign).unwrap();
    let tree_hash = hash_tree(&merkle);
    let tree_sign = sign(&keypair3.public, &keypair3.secret, &tree_hash);
    verify(&keypair3.public, &tree_hash, &tree_sign).unwrap();
    let signature = BlockSignature::new(data_sign, tree_sign);
    replica.append(data1, Some(signature)).await.unwrap();
    let data_hash = Hash::from_leaf(data2);
    merkle.next(data_hash.clone(), data2.len() as u64);
    let signature = BlockSignature::new(
        sign(&keypair3.public, &keypair3.secret, &data_hash),
        sign(&keypair3.public, &keypair3.secret, &hash_tree(&merkle)),
    );
    replica.append(data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    assert_eq!(read_bytes(&dir2, "0"), read_bytes(&dir, "0"));
    assert_eq!(read_bytes(&dir2, "1"), read_bytes(&dir, "1"));
    assert_eq!(read_bytes(&dir2, "2"), read_bytes(&dir, "2"));
}

#[test]
pub async fn replicate_manual_no_secret_key() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = Keypair::from_bytes(&keypair.to_bytes()).unwrap();
    let keypair3 = Keypair::from_bytes(&keypair.to_bytes()).unwrap();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.public,
        Some(keypair2.secret),
    )
    .await
    .unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();
    assert_eq!(core.len(), 2);

    let mut merkle = Merkle::default();
    let data_hash = Hash::from_leaf(data1);
    merkle.next(data_hash.clone(), data1.len() as u64);
    let signature = BlockSignature::new(
        sign(&keypair3.public, &keypair3.secret, &data_hash),
        sign(&keypair3.public, &keypair3.secret, &hash_tree(&merkle)),
    );
    replica.append(data1, Some(signature)).await.unwrap();
    let data_hash = Hash::from_leaf(data2);
    merkle.next(data_hash.clone(), data2.len() as u64);
    let signature = BlockSignature::new(
        sign(&keypair3.public, &keypair3.secret, &data_hash),
        sign(&keypair3.public, &keypair3.secret, &hash_tree(&merkle)),
    );
    replica.append(data2, Some(signature)).await.unwrap();
    assert_eq!(replica.len(), 2);

    assert_eq!(read_bytes(&dir2, "0"), read_bytes(&dir, "0"));
    assert_eq!(read_bytes(&dir2, "1"), read_bytes(&dir, "1"));
    assert_eq!(read_bytes(&dir2, "2"), read_bytes(&dir, "2"));
}

#[test]
pub async fn replicate_signatures_no_secret_key() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = Keypair::from_bytes(&keypair.to_bytes()).unwrap();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.public,
        Some(keypair2.secret),
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

#[test]
pub async fn replicate_then_append() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = Keypair::from_bytes(&keypair.to_bytes()).unwrap();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.public,
        Some(keypair2.secret),
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

#[test]
pub async fn replicate_fail_verify_then_append() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let dir2 = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = Keypair::from_bytes(&keypair.to_bytes()).unwrap();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();
    let mut replica = Core::new(
        IndexAccessFs::new(&dir2).await.unwrap(),
        keypair2.public,
        Some(keypair2.secret),
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
    let invalid_signature_1 = BlockSignature::new(
        signature.data(),
        Signature::from_bytes(&[0u8; SIGNATURE_LENGTH]).unwrap(),
    );
    let invalid_signature_2 = BlockSignature::new(
        Signature::from_bytes(&[0u8; SIGNATURE_LENGTH]).unwrap(),
        Signature::from_bytes(&[0u8; SIGNATURE_LENGTH]).unwrap(),
    );
    let invalid_signature_3 = BlockSignature::new(
        Signature::from_bytes(&[0u8; SIGNATURE_LENGTH]).unwrap(),
        signature.tree(),
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
