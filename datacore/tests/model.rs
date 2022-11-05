mod common;
use common::storage_memory;

use quickcheck::{Arbitrary, Gen};
use quickcheck_async;

use datacore::{Core, generate_keypair};

#[derive(Clone, Debug)]
enum Op {
    Get { index: u32 },
    Append { data: Vec<u8> },
}

impl Arbitrary for Op {
    fn arbitrary(g: &mut Gen) -> Self {
        let choices = [0, 1];
        match *g.choose(&choices).unwrap() {
            0 => {
                let indexes = [0, 1, 2, 3, 4, 5, 50, 1000, 100000];
                let index: u32 = *g.choose(&indexes).unwrap();
                Op::Get { index }
            }
            1 => {
                let lengths = [1, 2, 10, 50];
                let length = *g.choose(&lengths).unwrap();
                let mut data = Vec::with_capacity(length);
                for _ in 0..length {
                    data.push(u8::arbitrary(g));
                }
                Op::Append { data }
            }
            err => panic!("Invalid choice {}", err),
        }
    }
}

#[quickcheck_async::tokio]
async fn implementation_matches_model(ops: Vec<Op>) -> bool {
    let keypair = generate_keypair();
    let mut core = Core::new(
        storage_memory(),
        storage_memory(),
        storage_memory(),
        keypair.public, Some(keypair.secret))
        .await.unwrap();
    let mut model = vec![];

    for op in ops {
        match op {
            Op::Append { data } => {
                core.append(&data, None)
                    .await.expect("Append should be successful");
                model.push(data);
            },
            Op::Get { index } => {
                let data = core.get(index)
                    .await.expect("Get should be successful");
                if index >= core.len() {
                    assert_eq!(data, None);
                } else {
                    let (data, _) = data.unwrap();
                    assert_eq!(data, model[index as usize].clone());
                }
            },
        }
    }
    core.len() as usize == model.len()
}
