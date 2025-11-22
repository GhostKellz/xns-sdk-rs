//! # XNS SDK for Rust
//!
//! A Rust SDK for resolving XRPL Name Service (XNS) domains (.xrp names) on the XRP Ledger.
//!
//! ## Quick Start
//!
//! ```no_run
//! use xns_sdk_rs::{XnsResolver, XrplNetwork};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Connect to XRPL mainnet
//!     let resolver = XnsResolver::new(XrplNetwork::Mainnet).await?;
//!
//!     // Resolve domain to address
//!     let domain_info = resolver.resolve("ckelley.xrp").await?;
//!     println!("Owner: {}", domain_info.owner);
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod error;
pub mod models;
pub mod parser;
pub mod resolver;

// Re-exports
pub use client::{XrplClient, XrplNetwork};
pub use error::{XnsError, XnsResult};
pub use models::{DomainInfo, NamingService, NftMetadata};
pub use resolver::XnsResolver;
