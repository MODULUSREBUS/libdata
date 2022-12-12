use datacore::{generate_keypair, sign, Core, Hash, Keypair, Merkle, NodeTrait, Signature};
use index_access_fs::IndexAccessFs;
use index_access_memory::IndexAccessMemory;

#[tokio::test]
async fn core_init() {
    let keypair = generate_keypair();
    let core = Core::new(
        IndexAccessMemory::default(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    assert_eq!(core.len(), 0);
}

#[tokio::test]
async fn core_append() {
    let keypair = generate_keypair();
    let mut core = Core::new(
        IndexAccessMemory::default(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    core.append(br#"{"hello":"world"}"#, None).await.unwrap();
    core.append(br#"{"hello":"mundo"}"#, None).await.unwrap();
    core.append(br#"{"hello":"welt"}"#, None).await.unwrap();

    assert_eq!(core.len(), 3);
    assert_eq!(
        core.get(0).await.unwrap().unwrap().0,
        br#"{"hello":"world"}"#,
    );
    assert_eq!(
        core.get(1).await.unwrap().unwrap().0,
        br#"{"hello":"mundo"}"#,
    );
    assert_eq!(
        core.get(2).await.unwrap().unwrap().0,
        br#"{"hello":"welt"}"#,
    );
}

#[tokio::test]
async fn core_signatures() {
    let keypair = generate_keypair();
    let keypair2 = Keypair::from_bytes(&keypair.to_bytes()).unwrap();
    let mut core = Core::new(
        IndexAccessMemory::default(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    let data1 = b"hello world";
    let data2 = b"this is datacore";

    core.append(data1, None).await.unwrap();
    core.append(data2, None).await.unwrap();

    let mut merkle = Merkle::default();
    merkle.next(Hash::from_leaf(data1).unwrap(), data1.len() as u32);
    let signature1 = Signature::new(
        sign(
            &keypair2.public,
            &keypair2.secret,
            &Hash::from_leaf(data1).unwrap(),
        ),
        sign(&keypair2.public, &keypair2.secret, &hash_tree(&merkle)),
    );
    merkle.next(Hash::from_leaf(data2).unwrap(), data2.len() as u32);
    let signature2 = Signature::new(
        sign(
            &keypair2.public,
            &keypair2.secret,
            &Hash::from_leaf(data2).unwrap(),
        ),
        sign(&keypair2.public, &keypair2.secret, &hash_tree(&merkle)),
    );

    assert_eq!(core.len(), 2);
    assert_eq!(
        core.get(0).await.unwrap(),
        Some((data1.to_vec(), signature1))
    );
    assert_eq!(
        core.get(1).await.unwrap(),
        Some((data2.to_vec(), signature2))
    );
}

#[tokio::test]
async fn core_get_head() {
    let keypair = generate_keypair();
    let mut core = Core::new(
        IndexAccessMemory::default(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    assert_eq!(core.len(), 0);
    assert_eq!(core.head().await.unwrap(), None);

    core.append(br#"{"hello":"world"}"#, None).await.unwrap();
    core.append(br#"{"hello":"mundo"}"#, None).await.unwrap();
    core.append(br#"{"hello":"welt"}"#, None).await.unwrap();

    assert_eq!(core.len(), 3);
    assert_eq!(
        core.get(1).await.unwrap().unwrap().0,
        br#"{"hello":"mundo"}"#,
    );
    assert_eq!(
        core.get(2).await.unwrap().unwrap().0,
        br#"{"hello":"welt"}"#,
    );
    assert_eq!(
        core.head().await.unwrap().unwrap().0,
        br#"{"hello":"welt"}"#,
    );
}

#[tokio::test]
async fn core_append_no_secret_key() {
    let keypair = generate_keypair();
    let mut core = Core::new(IndexAccessMemory::default(), keypair.public, None)
        .await
        .unwrap();

    assert!(core.append(b"hello", None).await.is_err());
    assert_eq!(core.len(), 0);
}

#[tokio::test]
async fn core_disk_append() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    core.append(b"hello world", None).await.unwrap();
    core.append(b"this is datacore", None).await.unwrap();

    assert_eq!(core.len(), 2);
    assert_eq!(core.get(0).await.unwrap().unwrap().0, b"hello world",);
    assert_eq!(core.get(1).await.unwrap().unwrap().0, b"this is datacore",);
}

#[tokio::test]
async fn core_disk_persists() {
    let dir = tempfile::tempdir().unwrap().into_path();
    let keypair = generate_keypair();
    let keypair2 = Keypair::from_bytes(&keypair.to_bytes()).unwrap();
    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap();

    core.append(b"hello world", None).await.unwrap();
    core.append(b"this is datacore", None).await.unwrap();

    let mut core = Core::new(
        IndexAccessFs::new(&dir).await.unwrap(),
        keypair2.public,
        Some(keypair2.secret),
    )
    .await
    .unwrap();

    assert_eq!(core.len(), 2);
    assert_eq!(core.get(0).await.unwrap().unwrap().0, b"hello world",);
    assert_eq!(core.get(1).await.unwrap().unwrap().0, b"this is datacore",);
}

fn hash_tree(merkle: &Merkle) -> Hash {
    let roots = merkle.roots();
    let hashes = roots.iter().map(|root| root.hash()).collect::<Vec<&Hash>>();
    let lengths = roots.iter().map(|root| root.length()).collect::<Vec<u32>>();
    Hash::from_roots(&hashes, &lengths)
}
