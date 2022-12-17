use multi_map::MultiMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{key, Core, IndexAccess};

type PublicKeyBytes = [u8; 32];

/// [Cores] is a container for storing and quickly accessing multiple [Core]s.
///
/// Stored [Core]s can be accessed by [key::Public] or [key::Discovery].
pub struct Cores<T> {
    length: usize,
    map: MultiMap<key::Discovery, PublicKeyBytes, Arc<Mutex<Core<T>>>>,
}
impl<T> Default for Cores<T> {
    fn default() -> Self {
        Self {
            length: 0,
            map: MultiMap::default(),
        }
    }
}
impl<T: IndexAccess + Send> Cores<T> {
    /// Insert a [Core].
    #[inline]
    pub fn insert(&mut self, core: Core<T>) {
        let public = *core.public_key();
        let core = Arc::new(Mutex::new(core));

        self.put(&public, core);
    }
    /// Put a [Arc<Mutex<Core>>] under [PublicKey].
    pub fn put(&mut self, public: &key::Public, core: Arc<Mutex<Core<T>>>) {
        let public = public.to_bytes();
        let discovery = key::discovery(&public);

        self.map.insert(discovery, public, core);
        self.length += 1;
    }

    /// Try getting a [Core] by [PublicKey].
    #[must_use]
    #[inline]
    pub fn get_by_public(&self, key: &key::Public) -> Option<Arc<Mutex<Core<T>>>> {
        self.map.get_alt(&key.to_bytes()).map(Arc::clone)
    }

    /// Try getting a [Core] by [DiscoveryKey].
    #[must_use]
    #[inline]
    pub fn get_by_discovery(&self, key: &key::Discovery) -> Option<Arc<Mutex<Core<T>>>> {
        self.map.get(key).map(Arc::clone)
    }

    /// Returns the number of contained [Core]s.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.length
    }

    /// Checks if [Cores] is empty.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the [PublicKey]s of all stored [Core]s in an arbitrary order.
    #[inline]
    pub fn public_keys(&self) -> impl Iterator<Item = key::Public> + '_ {
        self.map
            .iter()
            .map(|(_discovery, (public, _core))| key::Public::from_bytes(public).unwrap())
    }
    /// Get the [DiscoveryKey]s of all stored [Core]s in an arbitrary order.
    #[inline]
    pub fn discovery_keys(&self) -> impl Iterator<Item = key::Discovery> + '_ {
        self.map
            .iter()
            .map(|(discovery, (_public, _core))| discovery.clone())
    }
    /// Access the contained [Core]s.
    #[inline]
    pub fn entries(&self) -> impl Iterator<Item = (key::Public, Arc<Mutex<Core<T>>>)> + '_ {
        self.map.iter().map(|(_discovery, (public, core))| {
            (key::Public::from_bytes(public).unwrap(), Arc::clone(core))
        })
    }
}
