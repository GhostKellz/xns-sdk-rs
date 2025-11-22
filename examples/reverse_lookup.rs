use xns_sdk_rs::{XnsResolver, XrplNetwork};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get address from command line or use default
    let address = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "reRDmP8LxyYunhcfmQMnSjinKXVvWtYxaw".to_string());

    println!("Creating XNS resolver for XRPL Mainnet...");
    let resolver = XnsResolver::new(XrplNetwork::Mainnet).await?;

    println!("\nReverse lookup for address: {}", address);

    match resolver.reverse_lookup(&address).await {
        Ok(domains) => {
            if domains.is_empty() {
                println!("No .xrp domains found for this address");
            } else {
                println!("\n✓ Found {} domain(s):", domains.len());
                for domain in domains {
                    println!("  - {}", domain);
                }
            }
        }
        Err(e) => {
            eprintln!("\n✗ Error: {}", e);
        }
    }

    Ok(())
}
