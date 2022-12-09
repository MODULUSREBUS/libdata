use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::Mutex;

use crate::{key, Core, DiscoveryKey, IndexAccess, PublicKey};

type PublicKeyBytes = [u8; 32];

#[derive(Default)]
/// [Cores] is a container for storing and quickly accessing multiple [Core]s.
///
/// Stored [Core]s can be accessed by [PublicKey] or [DiscoveryKey].
pub struct Cores<T> {
    by_public: HashMap<PublicKeyBytes, Arc<Mutex<Core<T>>>>,
    by_discovery: HashMap<DiscoveryKey, Weak<Mutex<Core<T>>>>,
}
impl<T: IndexAccess + Send> Cores<T> {
    /// Insert a new [Core].
    #[inline]
    pub fn insert(&mut self, core: Core<T>) {
        let public = *core.public_key();
        let core = Arc::new(Mutex::new(core));

        self.put(&public, core);
    }
    /// Put a [Arc<Mutex<Core>>] under [PublicKey].
    pub fn put(&mut self, public: &PublicKey, core: Arc<Mutex<Core<T>>>) {
        let public = public.to_bytes();
        let discovery = key::discovery(&public);

        self.by_discovery.insert(discovery, Arc::downgrade(&core));
        self.by_public.insert(public, core);
    }

    /// Try getting a [Core] by [PublicKey].
    #[must_use]
    #[inline]
    pub fn get_by_public(&self, key: &PublicKey) -> Option<Arc<Mutex<Core<T>>>> {
        self.by_public.get(&key.to_bytes()).map(Arc::clone)
    }

    /// Try getting a [Core] by [DiscoveryKey].
    #[must_use]
    #[inline]
    pub fn get_by_discovery(&self, key: &DiscoveryKey) -> Option<Arc<Mutex<Core<T>>>> {
        self.by_discovery
            .get(key)
            .and_then(std::sync::Weak::upgrade)
    }

    /// Returns the number of contained [Core]s.
    #[must_use]
    #[inline]
    pub fn len(&self) -> usize {
        self.by_public.len()
    }

    /// Checks if [Cores] is empty.
    #[must_use]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.by_public.len() == 0
    }

    /// Get the [PublicKey]s of all stored [Core]s in an arbitrary order.
    #[must_use]
    #[inline]
    pub fn public_keys(&self) -> Vec<PublicKey> {
        self.by_public
            .keys()
            .map(|bytes| PublicKey::from_bytes(bytes).unwrap())
            .collect()
    }

    /// Get the [DiscoveryKey]s of all stored [Core]s in an arbitrary order.
    #[must_use]
    #[inline]
    pub fn discovery_keys(&self) -> Vec<DiscoveryKey> {
        self.by_public.keys().map(key::discovery).collect()
    }

    /// Access the contained [Core]s.
    #[must_use]
    #[inline]
    pub fn entries(&self) -> Vec<(PublicKey, Arc<Mutex<Core<T>>>)> {
        self.by_public
            .iter()
            .map(|(bytes, core)| (PublicKey::from_bytes(bytes).unwrap(), Arc::clone(core)))
            .collect()
    }
}
