use xns_sdk_rs::{XnsResolver, XrplNetwork};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Connect to XRPL mainnet
    println!("Creating XNS resolver for XRPL Mainnet...");
    let resolver = XnsResolver::new(XrplNetwork::Mainnet).await?;

    // Resolve a domain
    let domain = "ckelley.xrp";
    println!("\nResolving domain: {}", domain);

    match resolver.resolve(domain).await {
        Ok(info) => {
            println!("\n✓ Domain found!");
            println!("  Domain: {}", info.domain);
            println!("  Owner: {}", info.owner);
            println!("  NFT ID: {}", info.nft_id);
            println!("  Service: {:?}", info.service);

            if !info.addresses.is_empty() {
                println!("\n  Addresses:");
                for (chain, addr) in &info.addresses {
                    println!("    {}: {}", chain, addr);
                }
            }

            if !info.text_records.is_empty() {
                println!("\n  Text Records:");
                for (key, value) in &info.text_records {
                    println!("    {}: {}", key, value);
                }
            }

            if let Some(metadata) = &info.metadata {
                println!("\n  Metadata:");
                println!("    Name: {}", metadata.name);
                if !metadata.description.is_empty() {
                    println!("    Description: {}", metadata.description);
                }
                if !metadata.image.is_empty() {
                    println!("    Image: {}", metadata.image);
                }
            }
        }
        Err(e) => {
            eprintln!("\n✗ Error resolving domain: {}", e);
        }
    }

    Ok(())
}
