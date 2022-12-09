use anyhow::{ensure, Result};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read};
use std::mem::size_of;

use crate::hash::{Hash, HASH_SIZE};
use crate::merkle_tree_stream::{HashMethods, MerkleTreeStream};

pub use crate::merkle_tree_stream::Node as NodeTrait;

pub const NODE_SIZE: usize = size_of::<u64>() + size_of::<u32>() + HASH_SIZE;

/// [Merkle] node.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Node {
    index: u64,
    length: u32,
    hash: Hash,
}

impl Node {
    /// Deserialize [Node].
    #[inline]
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut rdr = Cursor::new(data);
        let index = rdr.read_u64::<LittleEndian>()?;
        let length = rdr.read_u32::<LittleEndian>()?;
        let mut hash_bytes = [0u8; HASH_SIZE];
        rdr.read_exact(&mut hash_bytes)?;
        let hash = Hash::from_bytes(&hash_bytes)?;
        Ok(Self {
            index,
            length,
            hash,
        })
    }

    /// Serialize [Node].
    #[inline]
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(NODE_SIZE);
        data.write_u64::<LittleEndian>(self.index)?;
        data.write_u32::<LittleEndian>(self.length)?;
        data.extend_from_slice(self.hash.as_bytes());
        ensure!(data.len() == NODE_SIZE);
        Ok(data)
    }
}

impl NodeTrait<Hash> for Node {
    #[inline]
    fn new(index: u64, hash: Hash, length: u32) -> Self {
        Self {
            index,
            length,
            hash,
        }
    }
    #[inline]
    fn index(&self) -> u64 {
        self.index as u64
    }
    #[inline]
    fn hash(&self) -> &Hash {
        &self.hash
    }
    #[inline]
    fn length(&self) -> u32 {
        self.length
    }
}

#[derive(Debug, Clone)]
struct H;

impl HashMethods for H {
    type Hash = Hash;
    type Node = Node;

    #[inline]
    fn parent(&self, left: &Self::Node, right: &Self::Node) -> Self::Hash {
        let length = left.length + right.length;
        Hash::from_hashes(&left.hash, &right.hash, length)
    }
}

/// MerkleTreeStream for [Core].
///
/// [Core]: crate::core::Core
#[derive(Debug, Clone)]
pub struct Merkle {
    stream: MerkleTreeStream<H>,
}
impl Default for Merkle {
    fn default() -> Self {
        Self::from_roots(vec![])
    }
}
impl Merkle {
    /// Create a [Merkle] from root [Node]s.
    #[must_use]
    #[inline]
    pub fn from_roots(roots: Vec<Node>) -> Self {
        Self {
            stream: MerkleTreeStream::new(H, roots),
        }
    }

    /// Access the next item.
    #[inline]
    pub fn next(&mut self, data: Hash, length: u32) {
        self.stream.next(data, length);
    }

    /// Get the roots vector.
    #[must_use]
    #[inline]
    pub fn roots(&self) -> &Vec<Node> {
        self.stream.roots()
    }

    /// Get a vector of roots `Hash`'s'.
    #[must_use]
    #[inline]
    pub fn roots_hashes(&self) -> Vec<&Hash> {
        self.stream.roots().iter().map(|node| &node.hash).collect()
    }

    /// Get number of blocks.
    #[must_use]
    #[inline]
    pub fn blocks(&self) -> u32 {
        self.stream.blocks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init() {
        Merkle::default();
    }

    #[test]
    fn node() {
        let mut merkle = Merkle::default();
        merkle.next(Hash::from_leaf("a".as_bytes()).unwrap(), 1);
        let node = merkle.roots().get(0).unwrap();
        let node2 = Node::from_bytes(&node.to_bytes().unwrap()).unwrap();
        assert_eq!(node2, *node);
    }

    #[test]
    fn next() {
        let mut merkle = Merkle::default();
        merkle.next(Hash::from_leaf("a".as_bytes()).unwrap(), 1);
        merkle.next(Hash::from_leaf("b".as_bytes()).unwrap(), 1);
        merkle.next(Hash::from_leaf("c".as_bytes()).unwrap(), 1);
        assert_eq!(merkle.blocks(), 3);
    }

    #[test]
    fn next_long_data() {
        let mut merkle = Merkle::default();
        let data1 = "hello_world".as_bytes();
        let data2 = vec![7u8; 1024];
        merkle.next(Hash::from_leaf(data1).unwrap(), data1.len() as u32);
        merkle.next(Hash::from_leaf(&data2).unwrap(), data2.len() as u32);
        assert_eq!(merkle.blocks(), 2);
    }

    #[test]
    fn roots_full() {
        let mut merkle = Merkle::default();
        merkle.next(Hash::from_leaf("a".as_bytes()).unwrap(), 1);
        merkle.next(Hash::from_leaf("b".as_bytes()).unwrap(), 1);
        merkle.next(Hash::from_leaf("c".as_bytes()).unwrap(), 1);
        merkle.next(Hash::from_leaf("d".as_bytes()).unwrap(), 1);
        let roots = merkle.roots();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots.get(0).unwrap().index(), 3);
    }
    #[test]
    fn roots() {
        let mut merkle = Merkle::default();
        merkle.next(Hash::from_leaf("a".as_bytes()).unwrap(), 1);
        merkle.next(Hash::from_leaf("b".as_bytes()).unwrap(), 1);
        merkle.next(Hash::from_leaf("c".as_bytes()).unwrap(), 1);
        let roots = merkle.roots();
        assert_eq!(roots.len(), 2);
        assert_eq!(roots.get(0).unwrap().index(), 1);
        assert_eq!(roots.get(1).unwrap().index(), 4);
    }
}
