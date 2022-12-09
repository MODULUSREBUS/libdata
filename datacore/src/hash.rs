use anyhow::{ensure, Result};
use blake3::Hasher;
use byteorder::{LittleEndian, WriteBytesExt};
use std::mem::size_of;
use std::ops::Deref;

const HASH_LENGTH: usize = 32;

// https://en.wikipedia.org/wiki/Merkle_tree#Second_preimage_attack
const LEAF_TYPE: [u8; 1] = [0x00];
const PARENT_TYPE: [u8; 1] = [0x01];
const ROOT_TYPE: [u8; 1] = [0x02];

pub const HASH_SIZE: usize = HASH_LENGTH;

/// `BLAKE2b` hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hash {
    hash: [u8; HASH_SIZE],
}

impl Hash {
    /// Hash data to form a leaf `Hash`.
    #[inline]
    pub fn from_leaf(data: &[u8]) -> Result<Self> {
        let length = u32::try_from(data.len())?;

        let mut hasher = Hasher::new();
        hasher.update(&LEAF_TYPE);
        hasher.update(&u32_to_bytes(length));
        hasher.update(data);
        let hash = hasher.finalize().into();

        Ok(Self { hash })
    }

    /// Hash two `Hash` together to form a parent `Hash`.
    #[must_use]
    #[inline]
    pub fn from_hashes(left: &Hash, right: &Hash, length: u32) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&PARENT_TYPE);
        hasher.update(&u32_to_bytes(length));
        hasher.update(&left.hash);
        hasher.update(&right.hash);
        let hash = hasher.finalize().into();

        Self { hash }
    }

    /// Hash a vector of `Root` nodes.
    #[must_use]
    #[inline]
    pub fn from_roots(roots: &[&Hash], lengths: &[u32]) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&ROOT_TYPE);

        for (node, length) in roots.iter().zip(lengths.iter()) {
            hasher.update(&u32_to_bytes(*length));
            hasher.update(&node.hash);
        }
        let hash = hasher.finalize().into();

        Self { hash }
    }

    /// Returns a byte slice of this `Hash`.
    #[must_use]
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.hash
    }

    /// Create `Hash` from hash bytes and supplied length.
    #[inline]
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        ensure!(data.len() == HASH_SIZE);
        let hash: [u8; HASH_SIZE] = data.try_into()?;
        Ok(Self { hash })
    }
}

impl Deref for Hash {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_bytes()
    }
}

#[inline]
fn u32_to_bytes(n: u32) -> [u8; size_of::<u32>()] {
    let mut size = [0u8; size_of::<u32>()];
    size.as_mut().write_u32::<LittleEndian>(n).unwrap();
    size
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex;

    fn hex_bytes(hex: &str) -> Vec<u8> {
        hex::decode(hex).unwrap()
    }
    fn check_hash(hash: Hash, hex: &str) {
        println!("{}", hex::encode(hash.as_bytes()));
        assert_eq!(hash.as_bytes(), &hex_bytes(hex)[..]);
    }

    #[test]
    fn leaf_hash() {
        check_hash(
            Hash::from_leaf(&[]).unwrap(),
            "cdc96eca844d7912acdbb3dca677757d0db5747a1df61166339cfc7156d4880f",
        );
        check_hash(
            Hash::from_leaf(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]).unwrap(),
            "54c4c0f1453c53df34e2d2962f452a3d454296cadb1506c5e0019278003cb795",
        );
    }

    #[test]
    fn parent_hash() {
        let data1 = [0, 1, 2, 3, 4];
        let data2 = [42, 43, 44, 45, 46, 47, 48];
        let hash1 = Hash::from_leaf(&data1).unwrap();
        let hash2 = Hash::from_leaf(&data2).unwrap();
        let length = data1.len() as u32 + data2.len() as u32;
        check_hash(
            Hash::from_hashes(&hash1, &hash2, length),
            "939eb04de4f3039ec2e550ec890707232caab963c58c10edfea857f46862eb86",
        );
        check_hash(
            Hash::from_hashes(&hash2, &hash1, length),
            "0cbf73291fb0eeb81ad37f1e515ece705dd56932760bc948111ff6e3ca8f7fde",
        );
    }

    #[test]
    fn root_hash() {
        let data1 = [0, 1, 2, 3, 4];
        let data2 = [42, 43, 44, 45, 46, 47, 48];
        let hash1 = Hash::from_leaf(&data1).unwrap();
        let hash2 = Hash::from_leaf(&data2).unwrap();
        check_hash(
            Hash::from_roots(&[&hash1, &hash2], &[data1.len() as u32, data2.len() as u32]),
            "5c36f2176399be6bcfc3b8e387070155cc962bbad8e58d132e989349fc8bed27",
        );
        check_hash(
            Hash::from_roots(&[&hash2, &hash1], &[data2.len() as u32, data1.len() as u32]),
            "e57033e3148175562cdb3fc6904d6fa9bb8cdccb5bb32373872a494277633cc9",
        );
    }
}
