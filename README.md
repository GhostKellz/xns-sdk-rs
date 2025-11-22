# XNS SDK for Rust

A Rust SDK for resolving XRPL Name Service (XNS) domains (.xrp names) on the XRP Ledger.

## Features

- ✅ Resolve .xrp domains to XRPL addresses
- ✅ Reverse lookup (address → domains)
- ✅ Query NFT metadata (IPFS, HTTP, embedded JSON)
- ✅ Support for multiple naming services (XNS, XRP Domains)
- ✅ Direct XRPL RPC integration
- ✅ Built-in caching (5 min TTL)
- ✅ Rust 2024 Edition
- 🚧 WebAssembly support (optional)

## Quick Start

```rust
use xns_sdk_rs::{XnsResolver, XrplNetwork};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to XRPL mainnet
    let resolver = XnsResolver::new(XrplNetwork::Mainnet).await?;

    // Resolve domain to address
    let domain_info = resolver.resolve("ckelley.xrp").await?;
    println!("Owner: {}", domain_info.owner);
    println!("NFT ID: {}", domain_info.nft_id);

    // Reverse lookup
    let domains = resolver.reverse_lookup(&domain_info.owner).await?;
    println!("Domains owned: {:?}", domains);

    Ok(())
}
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
xns-sdk-rs = { path = "../xns-sdk-rs" }  # Local path
# or when published:
# xns-sdk-rs = "0.1"
```

## How It Works

XNS domains (.xrp) are NFTs on the XRP Ledger. This SDK:

1. Queries XRPL for NFTs from known naming service issuers
2. Parses NFT metadata (from IPFS, HTTP, or embedded data)
3. Matches domain names to find owner addresses
4. Caches results for performance

## Examples

### Resolve a Domain

```bash
cargo run --example resolve_domain
```

### Reverse Lookup

```bash
cargo run --example reverse_lookup reRDmP8LxyYunhcfmQMnSjinKXVvWtYxaw
```

## Supported Naming Services

- **XNS** - XRP Name Service (xrpns.com)
- **XRP Domains** - xrpdomains.xyz
- Extensible for future services

## Development

```bash
# Run tests
cargo test

# Build
cargo build --release

# Run example
cargo run --example resolve_domain
```

## Current Limitations

- NFT ownership tracking requires additional XRPL queries (WIP)
- Issuer addresses are placeholders (need actual XNS/XRP Domains issuer addresses)
- Metadata parsing supports standard NFT metadata format

## License

MIT - Christopher Kelley <ckelley@ghostkellz.sh>
