mod common;
use common::{create_pair_memory, establish};

use anyhow::Result;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::task;
use tokio::test;
use tokio_stream::StreamExt;

use protocol::{
    discovery_key,
    main::{Event::*, Stage},
    Key, Protocol,
};

#[test]
async fn basic_protocol() -> anyhow::Result<()> {
    async fn create_protocol_handler<T>(
        key: Key,
        mut proto: Protocol<T, Stage>,
        is_initiator: bool,
    ) -> task::JoinHandle<Result<Protocol<T, Stage>>>
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        task::spawn(async move {
            let discovery = discovery_key(&key);
            if is_initiator {
                proto.open(key)?;
            }
            loop {
                let msg = proto.next().await.unwrap().unwrap();
                match msg {
                    DiscoveryKey(remote_discovery) => {
                        if remote_discovery == discovery {
                            proto.open(key)?;
                        }
                    }
                    Open(remote_discovery) => {
                        if remote_discovery == discovery {
                            proto.close(discovery)?;
                        }
                    }
                    Close(remote_discovery) => {
                        if remote_discovery == discovery {
                            proto.close(discovery)?;
                            return Ok(proto);
                        }
                    }
                    _ => (),
                }
            }
        })
    }

    let (proto_a, proto_b) = create_pair_memory()?;
    let (proto_a, proto_b) = establish(proto_a, proto_b).await;

    let key = [3u8; 32];

    let a = create_protocol_handler(key, proto_a, true);
    let b = create_protocol_handler(key, proto_b, false);

    a.await;
    b.await;

    return Ok(());
}

#[test]
async fn basic_protocol_both_open() -> anyhow::Result<()> {
    async fn create_protocol_handler<T>(
        key: Key,
        mut proto: Protocol<T, Stage>,
    ) -> task::JoinHandle<Result<Protocol<T, Stage>>>
    where
        T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        task::spawn(async move {
            let discovery = discovery_key(&key);
            proto.open(key)?;
            loop {
                let msg = proto.next().await.unwrap().unwrap();
                match msg {
                    Open(remote_discovery) => {
                        if remote_discovery == discovery {
                            proto.close(discovery)?;
                        }
                    }
                    Close(remote_discovery) => {
                        if remote_discovery == discovery {
                            proto.close(discovery)?;
                            return Ok(proto);
                        }
                    }
                    _ => (),
                }
            }
        })
    }

    let (proto_a, proto_b) = create_pair_memory()?;
    let (proto_a, proto_b) = establish(proto_a, proto_b).await;

    let key = [3u8; 32];

    let a = create_protocol_handler(key, proto_a);
    let b = create_protocol_handler(key, proto_b);

    a.await;
    b.await;

    return Ok(());
}
