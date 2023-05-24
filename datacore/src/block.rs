use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Cursor, Read};
use std::mem::size_of;

/// Byte length of the [Signature].
pub const SIGNATURE_LENGTH: usize = 2 * ed25519_compact::Signature::BYTES;

/// [Signature] holds 2 [Block] [ed25519_compact::Signature]s:
/// - `data` - signature for the block data
/// - `tree` - signature for the block position in the merkle tree
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Signature {
    data: ed25519_compact::Signature,
    tree: ed25519_compact::Signature,
}
impl Signature {
    /// Create a new [Signature].
    #[must_use]
    #[inline]
    pub fn new(data: ed25519_compact::Signature, tree: ed25519_compact::Signature) -> Self {
        Self { data, tree }
    }
    /// Create a new [Signature].
    #[must_use]
    #[inline]
    pub fn from_bytes(
        data: [u8; ed25519_compact::Signature::BYTES],
        tree: [u8; ed25519_compact::Signature::BYTES],
        ) -> Self
    {
        Self {
            data: ed25519_compact::Signature::from_slice(&data).unwrap(),
            tree: ed25519_compact::Signature::from_slice(&tree).unwrap(),
        }
    }

    /// Get data [Signature].
    #[must_use]
    pub fn data(&self) -> &ed25519_compact::Signature {
        &self.data
    }

    /// Get tree [Signature].
    #[must_use]
    pub fn tree(&self) -> &ed25519_compact::Signature {
        &self.tree
    }
}

/// [Block] describes a block of data in `Core`.
/// Includes offset and length of the content data.
/// Includes data signature verifying the data content and
/// a tree signature verifying the block position in the `Core`.
#[derive(Debug, PartialEq, Eq)]
pub struct Block {
    offset: u64,
    length: u32,
    signature: Signature,
}

pub const BLOCK_LENGTH: usize = size_of::<u64>() + size_of::<u32>() + SIGNATURE_LENGTH;

impl Block {
    /// Create a new [Block].
    #[must_use]
    #[inline]
    pub fn new(offset: u64, length: u32, signature: Signature) -> Self {
        Self {
            offset,
            length,
            signature,
        }
    }

    /// Serialize [Block].
    #[inline]
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut data = Vec::with_capacity(BLOCK_LENGTH);

        data.write_u64::<LittleEndian>(self.offset)?;
        data.write_u32::<LittleEndian>(self.length)?;
        data.extend_from_slice(self.signature.data.as_slice());
        data.extend_from_slice(self.signature.tree.as_slice());

        Ok(data)
    }
    /// Deserialize [Block].
    #[inline]
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut rdr = Cursor::new(data);
        let offset = rdr.read_u64::<LittleEndian>()?;
        let length = rdr.read_u32::<LittleEndian>()?;

        let mut data_signature = [0u8; ed25519_compact::Signature::BYTES];
        rdr.read_exact(&mut data_signature)?;
        let mut tree_signature = [0u8; ed25519_compact::Signature::BYTES];
        rdr.read_exact(&mut tree_signature)?;

        let signature = Signature::new(
            ed25519_compact::Signature::from_slice(&data_signature)?,
            ed25519_compact::Signature::from_slice(&tree_signature)?,
        );

        Ok(Self {
            offset,
            length,
            signature,
        })
    }

    /// Get the offset of the content of this [Block].
    #[must_use]
    #[inline]
    pub fn offset(&self) -> u64 {
        self.offset
    }
    /// Get the length of content of this [Block].
    #[must_use]
    #[inline]
    pub fn length(&self) -> u32 {
        self.length
    }
    /// Get the [BlockSignature] of this [Block].
    #[must_use]
    #[inline]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn to_bytes_from_bytes() -> Result<()> {
        let signature = Signature::new(
            ed25519_compact::Signature::from_slice(&[2u8; ed25519_compact::Signature::BYTES])?,
            ed25519_compact::Signature::from_slice(&[7u8; ed25519_compact::Signature::BYTES])?,
        );
        let block = Block::new(1, 8, signature);
        let block2 = Block::from_bytes(&block.to_bytes()?)?;
        assert_eq!(block2, block);
        Ok(())
    }
    #[test]
    pub fn from_bytes_fails_on_incomplete_input() -> Result<()> {
        let signature = Signature::new(
            ed25519_compact::Signature::from_slice(&[2u8; ed25519_compact::Signature::BYTES])?,
            ed25519_compact::Signature::from_slice(&[7u8; ed25519_compact::Signature::BYTES])?,
        );
        let block = Block::new(1, 8, signature);
        let result = Block::from_bytes(&block.to_bytes()?[1..]);
        assert!(result.is_err());
        Ok(())
    }
    #[test]
    pub fn get_signatures() -> Result<()> {
        let data = ed25519_compact::Signature::from_slice(&[2u8; ed25519_compact::Signature::BYTES])?;
        let tree = ed25519_compact::Signature::from_slice(&[7u8; ed25519_compact::Signature::BYTES])?;
        let signature = Signature::new(data, tree);
        assert_eq!(*signature.data(), data);
        assert_eq!(*signature.tree(), tree);
        Ok(())
    }
}
