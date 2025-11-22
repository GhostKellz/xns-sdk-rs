use crate::client::XrplClient;
use crate::error::{XnsError, XnsResult};
use crate::models::{DomainInfo, NamingService, XrplNetwork};
use crate::parser::{MetadataParser};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;

/// XNS Resolver - main entry point for resolving .xrp domains
pub struct XnsResolver {
    client: Arc<XrplClient>,
    parser: Arc<MetadataParser>,
    cache: Cache<String, DomainInfo>,
    network: XrplNetwork,
}

impl XnsResolver {
    /// Create a new resolver for the given network
    pub async fn new(network: XrplNetwork) -> XnsResult<Self> {
        let client = Arc::new(XrplClient::new(network));
        let parser = Arc::new(MetadataParser::new());

        // Cache with 5 min TTL and 1000 entry limit
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(300))
            .build();

        Ok(Self {
            client,
            parser,
            cache,
            network,
        })
    }

    /// Create with custom RPC URL
    pub async fn with_url(network: XrplNetwork, rpc_url: String) -> XnsResult<Self> {
        let client = Arc::new(XrplClient::with_url(network, rpc_url));
        let parser = Arc::new(MetadataParser::new());

        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(300))
            .build();

        Ok(Self {
            client,
            parser,
            cache,
            network,
        })
    }

    /// Resolve a .xrp domain to its owner and metadata
    pub async fn resolve(&self, domain: &str) -> XnsResult<DomainInfo> {
        // Validate domain format
        if !domain.ends_with(".xrp") {
            return Err(XnsError::InvalidDomain(format!(
                "Domain must end with .xrp: {}",
                domain
            )));
        }

        // Check cache first
        if let Some(cached) = self.cache.get(domain).await {
            tracing::debug!("Cache hit for domain: {}", domain);
            return Ok(cached);
        }

        tracing::info!("Resolving domain: {}", domain);

        // Try each naming service
        let services = [NamingService::XNS, NamingService::XRPDomains];

        for service in &services {
            match self.resolve_from_service(domain, *service).await {
                Ok(domain_info) => {
                    // Cache the result
                    self.cache.insert(domain.to_string(), domain_info.clone()).await;
                    return Ok(domain_info);
                }
                Err(e) => {
                    tracing::debug!("Service {:?} failed for {}: {}", service, domain, e);
                    continue;
                }
            }
        }

        Err(XnsError::DomainNotFound(domain.to_string()))
    }

    /// Resolve from a specific naming service
    async fn resolve_from_service(
        &self,
        domain: &str,
        service: NamingService,
    ) -> XnsResult<DomainInfo> {
        // Get issuer address for this service
        let issuer = service
            .issuer_address(self.network)
            .ok_or_else(|| XnsError::UnsupportedService(format!("{:?}", service)))?;

        tracing::debug!("Querying {:?} issuer: {}", service, issuer);

        // Query all NFTs from this issuer
        let nfts = self.client.account_nfts(issuer).await?;

        tracing::debug!("Found {} NFTs from {:?}", nfts.len(), service);

        // Search for matching domain
        for nft in nfts {
            if let Some(uri_hex) = &nft.uri {
                match self.parser.parse_uri(uri_hex).await {
                    Ok(metadata) => {
                        if let Some(nft_domain) = MetadataParser::extract_domain_name(&metadata) {
                            if nft_domain.eq_ignore_ascii_case(domain) {
                                tracing::info!("Found domain {} in NFT {}", domain, nft.nft_token_id);

                                // Get the actual owner (might not be issuer)
                                let owner = self.get_nft_owner(&nft.nft_token_id).await?;

                                return Ok(DomainInfo {
                                    domain: nft_domain,
                                    owner,
                                    nft_id: nft.nft_token_id,
                                    service,
                                    addresses: Default::default(), // TODO: Parse from metadata
                                    text_records: Default::default(), // TODO: Parse from metadata
                                    expires_at: None, // TODO: Parse from metadata
                                    metadata: Some(metadata),
                                });
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse NFT metadata: {}", e);
                        continue;
                    }
                }
            }
        }

        Err(XnsError::DomainNotFound(domain.to_string()))
    }

    /// Get current owner of an NFT
    async fn get_nft_owner(&self, nft_id: &str) -> XnsResult<String> {
        // For now, we need to find who owns this NFT
        // This requires querying account_nfts for all possible accounts
        // which is not practical. Instead, we'll use the issuer as placeholder
        // TODO: Implement proper NFT ownership tracking

        tracing::warn!("NFT ownership lookup not yet implemented, using placeholder");
        Ok("rUnknownOwner".to_string())
    }

    /// Reverse lookup: find domains owned by an address
    pub async fn reverse_lookup(&self, address: &str) -> XnsResult<Vec<String>> {
        tracing::info!("Reverse lookup for address: {}", address);

        let nfts = self.client.account_nfts(address).await?;
        let mut domains = Vec::new();

        for nft in nfts {
            if let Some(uri_hex) = &nft.uri {
                if let Ok(metadata) = self.parser.parse_uri(uri_hex).await {
                    if let Some(domain) = MetadataParser::extract_domain_name(&metadata) {
                        domains.push(domain);
                    }
                }
            }
        }

        Ok(domains)
    }

    /// Clear the cache
    pub async fn clear_cache(&self) {
        self.cache.invalidate_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resolver_creation() {
        let resolver = XnsResolver::new(XrplNetwork::Mainnet).await;
        assert!(resolver.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_domain() {
        let resolver = XnsResolver::new(XrplNetwork::Mainnet).await.unwrap();
        let result = resolver.resolve("invalid.com").await;
        assert!(matches!(result, Err(XnsError::InvalidDomain(_))));
    }
}
