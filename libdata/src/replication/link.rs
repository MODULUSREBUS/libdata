use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio_stream::{Stream, StreamExt};

use crate::replication::{
    Command, Data, DataOrRequest, Options, ReplicaTrait, Handle, Request,
};
use crate::key;
use protocol::main::{Event as ProtocolEvent, Stage};
use protocol::{self, Message, Protocol};

/// [Link] event.
#[derive(Debug)]
pub enum Event {
    Command(Command),
    Event(Result<ProtocolEvent>),
}

/// Replication protocol main abstraction: handle handshake, multiplexing, failures.
///
/// Concrete behavior is specified in [ReplicaTrait].
pub struct Link<T: 'static>
where
    T: AsyncWrite + AsyncRead + Send + Unpin,
{
    protocol: Protocol<T, Stage>,
    command_rx: UnboundedReceiver<Command>,
    replicas: HashMap<key::Discovery, Box<dyn ReplicaTrait + Send>>,
}
impl<T: 'static> Link<T>
where
    T: AsyncWrite + AsyncRead + Send + Unpin,
{
    /// Create [Link] and wait for protocol handshake.
    pub async fn new(stream: T, is_initiator: bool) -> Result<(Self, Handle)> {
        Self::with_options(
            stream,
            Options {
                is_initiator,
                ..Options::default()
            },
        )
        .await
    }

    /// Create [Link] with [Options] and wait for protocol handshake.
    pub async fn with_options(stream: T, options: Options) -> Result<(Self, Handle)> {
        let (tx, rx) = unbounded_channel();
        let handle = Handle::new(tx);

        let handshake = protocol::new(stream, options);
        let protocol = handshake.handshake().await?;

        let replication = Self {
            protocol,
            command_rx: rx,
            replicas: HashMap::new(),
        };

        Ok((replication, handle))
    }

    /// Run the replication loop to completion.
    pub async fn run(self) -> Result<()> {
        let on_discovery = |_| async move { Ok(()) };
        self.run_with_discovery_hook(on_discovery).await
    }
    /// Run the replication loop to completion
    /// with an `on_discovery` hook: handle [ProtocolEvent::DiscoveryKey].
    pub async fn run_with_discovery_hook<F>(
        mut self,
        on_discovery: impl Fn(key::Discovery) -> F + Copy,
    ) -> Result<()>
    where
        F: Future<Output = Result<()>>,
    {
        loop {
            match self.next().await.ok_or_else(|| anyhow!("broken link"))? {
                Event::Command(cmd) => {
                    if !self.handle_command(cmd).await? {
                        return Ok(());
                    }
                }
                Event::Event(event) => {
                    if !self.handle_event(event, on_discovery).await? {
                        return Ok(());
                    }
                }
            };
        }
    }
    async fn handle_command(&mut self, command: Command) -> Result<bool> {
        #[cfg(test)]
        println!("handle_command {:?}", command);

        match command {
            Command::Open(key, replica) => {
                let discovery = key::discovery(&key.as_slice().try_into().unwrap());
                self.replicas.insert(discovery, replica);
                self.protocol.open(*key)?;
                Ok(true)
            }
            Command::ReOpen(key) => {
                self.replica_on_open(&key).await?;
                Ok(true)
            }
            Command::Close(key) => {
                self.protocol.close(key)?;
                self.replicas.remove(&key);
                Ok(true)
            }
            Command::Quit() => {
                let mut is_error = false;
                for replica in self.replicas.values_mut() {
                    is_error |= replica.on_close().await.is_err();
                }
                if is_error {
                    Err(anyhow!("Quit before replication finished."))
                } else {
                    Ok(false)
                }
            }
        }
    }
    async fn handle_event<F>(
        &mut self,
        event: Result<ProtocolEvent>,
        on_discovery: impl FnOnce(key::Discovery) -> F,
    ) -> Result<bool>
    where
        F: Future<Output = Result<()>>,
    {
        #[cfg(test)]
        println!("handle_event {:?}", event);

        let msg = match event {
            Ok(msg) => msg,
            Err(err) => {
                let mut is_error = false;
                for replica in self.replicas.values_mut() {
                    is_error |= replica.on_close().await.is_err();
                }
                return if is_error {
                    Err(err)
                } else {
                    Ok(false)
                }
            }
        };

        match msg {
            ProtocolEvent::DiscoveryKey(discovery) => {
                on_discovery(discovery).await?;
            }
            ProtocolEvent::Open(discovery) => {
                self.replica_on_open(&discovery).await?;
            }
            ProtocolEvent::Close(discovery) => {
                self.replica_on_close(&discovery).await?;
            }
            ProtocolEvent::Message(discovery, msg) => match msg {
                Message::Request(request) => {
                    self.replica_on_request(&discovery, request).await?;
                }
                Message::Data(data) => {
                    self.replica_on_data(&discovery, data).await?;
                }
                _ => {}
            },
        };
        Ok(true)
    }

    async fn replica_on_open(&mut self, key: &key::Discovery) -> Result<()> {
        if let Some(replica) = self.replicas.get_mut(key) {
            let request = replica.on_open().await?;
            if let Some(request) = request {
                self.protocol.request(key, request)?;
            }
        }
        Ok(())
    }

    async fn replica_on_close(&mut self, key: &key::Discovery) -> Result<()> {
        if let Some(replica) = self.replicas.get_mut(key) {
            replica.on_close().await?;
        }
        self.replicas.remove(key);
        Ok(())
    }

    async fn replica_on_request(&mut self, key: &key::Discovery, request: Request) -> Result<()> {
        if let Some(replica) = self.replicas.get_mut(key) {
            let msg = replica.on_request(request).await?;
            match msg {
                Some(DataOrRequest::Data(data)) => self.protocol.data(key, data)?,
                Some(DataOrRequest::Request(request)) => self.protocol.request(key, request)?,
                None => {}
            };
        }
        Ok(())
    }

    async fn replica_on_data(&mut self, key: &key::Discovery, data: Data) -> Result<()> {
        if let Some(replica) = self.replicas.get_mut(key) {
            let request = replica.on_data(data).await?;
            if let Some(request) = request {
                self.protocol.request(key, request)?;
            }
        }
        Ok(())
    }
}
impl<T: 'static> Stream for Link<T>
where
    T: AsyncWrite + AsyncRead + Send + Unpin,
{
    type Item = Event;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if let Poll::Ready(Some(t)) = this.command_rx.poll_recv(cx) {
            return Poll::Ready(Some(Event::Command(t)));
        }
        if let Poll::Ready(Some(t)) = Pin::new(&mut this.protocol).poll_next(cx) {
            return Poll::Ready(Some(Event::Event(t)));
        }
        Poll::Pending
    }
}
