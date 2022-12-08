use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::Mutex;

use crate::{discovery_key, Core, DiscoveryKey, IndexAccess, PublicKey};

type PublicKeyBytes = [u8; 32];

/// [Cores] is a container for storing and quickly accessing multiple [Core]s.
///
/// Stored [Core]s can be accessed by [PublicKey] or [DiscoveryKey].
pub struct Cores<T, B> {
    by_public: HashMap<PublicKeyBytes, Arc<Mutex<Core<T, B>>>>,
    by_discovery: HashMap<DiscoveryKey, Weak<Mutex<Core<T, B>>>>,
}
impl<T, B> Default for Cores<T, B> {
    fn default() -> Self {
        Self {
            by_public: HashMap::new(),
            by_discovery: HashMap::new(),
        }
    }
}
impl<T, B> Cores<T, B>
where
    T: IndexAccess + Send,
    B: IndexAccess + Send,
{
    /// Insert a new [Core].
    #[inline]
    pub fn insert(&mut self, core: Core<T, B>) {
        let public = *core.public_key();
        let core = Arc::new(Mutex::new(core));

        self.put(&public, core);
    }
    /// Put a [Arc<Mutex<Core>>] under [PublicKey].
    pub fn put(&mut self, public: &PublicKey, core: Arc<Mutex<Core<T, B>>>) {
        let public = public.to_bytes();
        let discovery = discovery_key(&public);

        self.by_discovery.insert(discovery, Arc::downgrade(&core));
        self.by_public.insert(public, core);
    }

    /// Try getting a [Core] by [PublicKey].
    #[inline]
    pub fn get_by_public(&self, key: &PublicKey) -> Option<Arc<Mutex<Core<T, B>>>> {
        self.by_public.get(&key.to_bytes()).map(Arc::clone)
    }

    /// Try getting a [Core] by [DiscoveryKey].
    #[inline]
    pub fn get_by_discovery(&self, key: &DiscoveryKey) -> Option<Arc<Mutex<Core<T, B>>>> {
        self.by_discovery.get(key).and_then(|weak| weak.upgrade())
    }

    /// Returns the number of contained [Core]s.
    #[inline]
    pub fn len(&self) -> usize {
        self.by_public.len()
    }

    /// Checks if [Cores] is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.by_public.len() == 0
    }

    /// Get the [PublicKey]s of all stored [Core]s in an arbitrary order.
    #[inline]
    pub fn public_keys(&self) -> Vec<PublicKey> {
        self.by_public
            .keys()
            .map(|bytes| PublicKey::from_bytes(bytes).unwrap())
            .collect()
    }

    /// Get the [DiscoveryKey]s of all stored [Core]s in an arbitrary order.
    #[inline]
    pub fn discovery_keys(&self) -> Vec<DiscoveryKey> {
        self.by_public.keys().map(discovery_key).collect()
    }

    /// Access the contained [Core]s.
    #[inline]
    pub fn entries(&self) -> Vec<(PublicKey, Arc<Mutex<Core<T, B>>>)> {
        self.by_public
            .iter()
            .map(|(bytes, core)| (PublicKey::from_bytes(bytes).unwrap(), Arc::clone(core)))
            .collect()
    }
}
