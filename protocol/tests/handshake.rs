mod common;
use common::{create_duplex_pair_memory, create_pair_memory, create_pair_tcp, establish};

use anyhow::Result;
use tokio::{task, test};

use protocol::{self, Options};

#[test]
async fn test_handshake() -> Result<()> {
    let (proto_a, proto_b) = create_duplex_pair_memory();

    let b = protocol::default(proto_b, false);
    let a = protocol::default(proto_a, true);

    let task_a = task::spawn(async move { a.handshake().await.unwrap() });
    let task_b = task::spawn(async move { b.handshake().await.unwrap() });

    task_a.await?;
    task_b.await?;
    Ok(())
}

#[test]
async fn test_handshake_disabled() -> Result<()> {
    let (proto_a, proto_b) = create_duplex_pair_memory();

    let b = protocol::new(
        proto_b,
        Options {
            is_initiator: false,
            noise: false,
            ..Options::default()
        },
    );
    let a = protocol::new(
        proto_a,
        Options {
            is_initiator: true,
            noise: false,
            ..Options::default()
        },
    );

    let task_a = task::spawn(async move { a.handshake().await.unwrap() });
    let task_b = task::spawn(async move { b.handshake().await.unwrap() });

    task_a.await?;
    task_b.await?;
    Ok(())
}

#[test]
async fn test_handshake_test_helpers_memory() -> Result<()> {
    let (proto_a, proto_b) = create_pair_memory()?;
    let (_, _) = establish(proto_a, proto_b).await;
    Ok(())
}

#[test]
async fn test_handshake_test_helpers_tcp() -> Result<()> {
    let (proto_a, proto_b) = create_pair_tcp().await?;
    let (_, _) = establish(proto_a, proto_b).await;
    Ok(())
}
