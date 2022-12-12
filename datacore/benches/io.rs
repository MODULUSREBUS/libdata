use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;

use datacore::{generate_keypair, Core};
use index_access_memory::IndexAccessMemory;

async fn init() -> Core<IndexAccessMemory> {
    let keypair = generate_keypair();
    Core::new(
        IndexAccessMemory::default(),
        keypair.public,
        Some(keypair.secret),
    )
    .await
    .unwrap()
}

async fn hypercore_append(mut core: Core<IndexAccessMemory>, blocks: u32) {
    for _ in 0..blocks {
        core.append(b"hello world", None).await.unwrap();
    }
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("append 1K blocks", |b| {
        let rt = Runtime::new().unwrap();
        b.to_async(rt).iter(|| async {
            let core = init().await;
            hypercore_append(black_box(core), black_box(1_000)).await;
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
