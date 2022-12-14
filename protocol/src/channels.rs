use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};

use crate::{discovery_key, DiscoveryKey, Key};

#[inline]
fn error<T>(kind: ErrorKind, msg: &str) -> Result<T> {
    Err(anyhow!(Error::new(kind, msg)))
}

#[derive(Clone, Debug)]
struct LocalState {
    local_id: u32,
    key: Key,
}

#[derive(Clone, Debug)]
struct RemoteState {
    remote_id: u32,
    remote_capability: Option<Vec<u8>>,
}

/// The handle for a channel that lives with the main Protocol.
#[derive(Clone, Debug)]
pub struct ChannelHandle {
    discovery_key: DiscoveryKey,
    local_state: Option<LocalState>,
    remote_state: Option<RemoteState>,
}

impl ChannelHandle {
    #[inline]
    fn new(discovery_key: DiscoveryKey) -> Self {
        Self {
            discovery_key,
            local_state: None,
            remote_state: None,
        }
    }
    #[inline]
    fn new_local(local_id: u32, discovery_key: DiscoveryKey, key: Key) -> Self {
        let mut this = Self::new(discovery_key);
        this.attach_local(local_id, key);
        this
    }
    #[inline]
    fn new_remote(
        remote_id: u32,
        discovery_key: DiscoveryKey,
        remote_capability: Option<Vec<u8>>,
    ) -> Self {
        let mut this = Self::new(discovery_key);
        this.attach_remote(remote_id, remote_capability);
        this
    }

    #[inline]
    pub fn attach_local(&mut self, local_id: u32, key: Key) {
        let local_state = LocalState { local_id, key };
        self.local_state = Some(local_state);
    }
    #[inline]
    pub fn attach_remote(&mut self, remote_id: u32, remote_capability: Option<Vec<u8>>) {
        let remote_state = RemoteState {
            remote_id,
            remote_capability,
        };
        self.remote_state = Some(remote_state);
    }

    #[inline]
    pub fn discovery_key(&self) -> &[u8; 32] {
        &self.discovery_key
    }
    #[inline]
    pub fn local_id(&self) -> Option<u32> {
        self.local_state.as_ref().map(|s| s.local_id)
    }
    #[inline]
    pub fn remote_id(&self) -> Option<u32> {
        self.remote_state.as_ref().map(|s| s.remote_id)
    }

    #[inline]
    pub fn is_connected(&self) -> bool {
        self.local_state.is_some() && self.remote_state.is_some()
    }

    #[inline]
    pub fn prepare_to_verify(&self) -> Result<(&Key, Option<&Vec<u8>>)> {
        if !self.is_connected() {
            return error(
                ErrorKind::NotConnected,
                "Channel is not opened from both local and remote",
            );
        }
        // Safe because of the `is_connected()` check above.
        let local_state = self.local_state.as_ref().unwrap();
        let remote_state = self.remote_state.as_ref().unwrap();
        Ok((&local_state.key, remote_state.remote_capability.as_ref()))
    }
}

/// The ChannelMap maintains a list of open channels
/// and their local (tx) and remote (rx) channel IDs.
#[derive(Debug)]
pub struct ChannelMap {
    channels: HashMap<String, ChannelHandle>,
    local_id: Vec<Option<String>>,
    remote_id: Vec<Option<String>>,
}

impl ChannelMap {
    pub fn new() -> Self {
        Self {
            channels: HashMap::new(),
            // Add a first None value to local_id to start ids at 1.
            // This makes sure that 0 may be used for stream-level extensions.
            local_id: vec![None],
            remote_id: vec![],
        }
    }

    pub fn attach_local(&mut self, key: Key) -> Result<&ChannelHandle> {
        let discovery_key = discovery_key(&key);
        let discovery_key_hex = hex::encode(&discovery_key);
        let local_id_raw = self.alloc_local();
        let local_id = u32::try_from(local_id_raw)?;

        self.channels
            .entry(discovery_key_hex.clone())
            .and_modify(|channel| channel.attach_local(local_id, key))
            .or_insert_with(|| ChannelHandle::new_local(local_id, discovery_key, key));

        self.local_id[local_id_raw] = Some(discovery_key_hex.clone());
        self.channels
            .get(&discovery_key_hex)
            .ok_or_else(|| anyhow!("no channel for id"))
    }

    pub fn attach_remote(
        &mut self,
        discovery_key: DiscoveryKey,
        remote_id: u32,
        remote_capability: Option<Vec<u8>>,
    ) -> Result<&ChannelHandle> {
        let discovery_key_hex = hex::encode(&discovery_key);
        let remote_id_raw = usize::try_from(remote_id)?;
        self.alloc_remote(remote_id_raw);

        self.channels
            .entry(discovery_key_hex.clone())
            .and_modify(|channel| channel.attach_remote(remote_id, remote_capability.clone()))
            .or_insert_with(|| {
                ChannelHandle::new_remote(remote_id, discovery_key, remote_capability)
            });

        self.remote_id[remote_id_raw] = Some(discovery_key_hex.clone());
        self.channels
            .get(&discovery_key_hex)
            .ok_or_else(|| anyhow!("no channel for id"))
    }

    pub fn get(&self, discovery_key: &DiscoveryKey) -> Option<&ChannelHandle> {
        let discovery_key_hex = hex::encode(&discovery_key);
        self.channels.get(&discovery_key_hex)
    }
    pub fn get_remote(&self, remote_id: usize) -> Option<&ChannelHandle> {
        if let Some(Some(discovery_key_hex)) = self.remote_id.get(remote_id).as_ref() {
            self.channels.get(discovery_key_hex)
        } else {
            None
        }
    }
    pub fn get_local(&self, local_id: usize) -> Option<&ChannelHandle> {
        if let Some(Some(discovery_key_hex)) = self.local_id.get(local_id).as_ref() {
            self.channels.get(discovery_key_hex)
        } else {
            None
        }
    }

    pub fn remove(&mut self, discovery_key: &[u8]) {
        let discovery_key_hex = hex::encode(discovery_key);
        let channel = self.channels.get(&discovery_key_hex);
        if let Some(channel) = channel {
            if let Some(local_id) = channel.local_id() {
                let local_id_raw = usize::try_from(local_id).unwrap();
                self.local_id[local_id_raw] = None;
            }
            if let Some(remote_id) = channel.remote_id() {
                let remote_id_raw = usize::try_from(remote_id).unwrap();
                self.remote_id[remote_id_raw] = None;
            }
        }
        self.channels.remove(&discovery_key_hex);
    }

    pub fn prepare_to_verify(&self, local_id: u32) -> Result<(&Key, Option<&Vec<u8>>)> {
        let local_id_raw = usize::try_from(local_id)?;
        let channel_handle = match self.get_local(local_id_raw) {
            None => return error(ErrorKind::NotFound, "Channel not found"),
            Some(handle) => handle,
        };
        channel_handle.prepare_to_verify()
    }

    fn alloc_local(&mut self) -> usize {
        let id = self.local_id.iter().skip(1).position(Option::is_none);
        if let Some(empty_id) = id {
            empty_id
        } else {
            self.local_id.push(None);
            self.local_id.len() - 1
        }
    }
    fn alloc_remote(&mut self, id: usize) {
        if self.remote_id.len() > id {
            self.remote_id[id] = None;
        } else {
            self.remote_id.resize(id + 1, None);
        }
    }
}
