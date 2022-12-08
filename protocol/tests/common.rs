#![cfg_attr(test, allow(dead_code))]

use anyhow::Result;
use async_compat::{Compat, CompatExt};
use sluice::pipe::{pipe, PipeReader, PipeWriter};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::task;
use tokio_stream::StreamExt;

use protocol::{
    handshake, main, new_protocol, new_protocol_with_defaults, Duplex, Options, Protocol,
};

pub fn create_duplex_pair_memory() -> (
    Duplex<Compat<PipeReader>, Compat<PipeWriter>>,
    Duplex<Compat<PipeReader>, Compat<PipeWriter>>,
) {
    let (ar, bw) = pipe();
    let (br, aw) = pipe();

    (
        Duplex::new(ar.compat(), aw.compat()),
        Duplex::new(br.compat(), bw.compat()),
    )
}

pub type MemoryProtocol =
    Protocol<Duplex<Compat<PipeReader>, Compat<PipeWriter>>, handshake::Stage>;
pub fn create_pair_memory() -> Result<(MemoryProtocol, MemoryProtocol)> {
    create_pair_memory_keepalive(Some(1_000))
}
pub fn create_pair_memory_keepalive(
    keepalive_ms: Option<u64>,
) -> Result<(MemoryProtocol, MemoryProtocol)> {
    let (a, b) = create_duplex_pair_memory();
    let b = new_protocol(
        b,
        Options {
            is_initiator: false,
            keepalive_ms,
            ..Options::default()
        },
    );
    let a = new_protocol(
        a,
        Options {
            is_initiator: true,
            keepalive_ms,
            ..Options::default()
        },
    );
    Ok((a, b))
}

pub async fn establish<T>(
    a: Protocol<T, handshake::Stage>,
    b: Protocol<T, handshake::Stage>,
) -> (Protocol<T, main::Stage>, Protocol<T, main::Stage>)
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    let task_a = task::spawn(async move { a.handshake().await.unwrap() });
    let task_b = task::spawn(async move { b.handshake().await.unwrap() });
    let a = task_a.await.unwrap();
    let b = task_b.await.unwrap();
    (a, b)
}

pub async fn next_event<T>(
    mut proto: Protocol<T, main::Stage>,
) -> (main::Event, Protocol<T, main::Stage>)
where
    T: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    let task = task::spawn(async move {
        let e1 = proto.next().await.unwrap();
        (e1.unwrap(), proto)
    });
    task.await.unwrap()
}

pub type TcpProtocol = Protocol<TcpStream, handshake::Stage>;
pub async fn create_pair_tcp() -> Result<(TcpProtocol, TcpProtocol)> {
    let (stream_a, stream_b) = tcp::pair().await?;
    let b = new_protocol_with_defaults(stream_b, false);
    let a = new_protocol_with_defaults(stream_a, true);
    Ok((a, b))
}

pub mod tcp {
    use std::io::Result;
    use tokio::net::{TcpListener, TcpStream};
    use tokio::task;

    pub async fn pair() -> Result<(TcpStream, TcpStream)> {
        let address = "localhost:9999";
        let listener = TcpListener::bind(&address).await?;

        let connect_task = task::spawn(async move { TcpStream::connect(&address).await });

        let (server_stream, _) = listener.accept().await?;
        let client_stream = connect_task.await??;
        Ok((server_stream, client_stream))
    }
}
