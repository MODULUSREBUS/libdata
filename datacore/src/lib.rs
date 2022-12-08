#![forbid(unsafe_code, bad_style, nonstandard_style, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2021_compatibility)]
#![deny(missing_docs)]
#![cfg_attr(test, allow(dead_code))]
#![cfg_attr(test, deny(warnings))]

//! ## Introduction
//! Datacore is a secure, append-only, single-writer log.
//! It is meant for sharing large datasets and streams of real time data.
//! The primary way to use this crate is through the [Core] struct.
//!
//! ## Example
//! ```rust
//! # use futures_lite::future::FutureExt;
//!
//! use index_access_memory::IndexAccessMemory;
//! use datacore::{Core, generate_keypair};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let keypair = generate_keypair();
//! let mut core = Core::new(
//!     IndexAccessMemory::new(),
//!     IndexAccessMemory::new(),
//!     keypair.public, Some(keypair.secret)
//!     ).await?;
//!
//! core.append(b"hello", None).await?;
//! core.append(b"world", None).await?;
//!
//! assert_eq!(core.len(), 2);
//! assert_eq!(
//!     core.get(0).await?.unwrap().0,
//!     b"hello");
//! assert_eq!(
//!     core.get(1).await?.unwrap().0,
//!     b"world");
//! # Ok(())
//! # }
//! ```

mod block;
mod core;
mod hash;
mod keys;
mod merkle;
mod merkle_tree_stream;
mod store;
mod store_blocks;

pub use self::core::{Core, MAX_BLOCK_SIZE, MAX_CORE_LENGTH};
pub use block::{Block, BlockSignature, Signature, SIGNATURE_LENGTH};
pub use hash::Hash;
pub use index_access_storage::IndexAccess;
pub use keys::{generate_keypair, sign, verify, Keypair, PublicKey, SecretKey};
pub use merkle::{Merkle, Node, NodeTrait};
