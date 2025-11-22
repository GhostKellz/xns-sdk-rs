use crate::client::XrplClient;
use crate::error::{XnsError, XnsResult};
use crate::models::{DomainInfo, NamingService, XrplNetwork};
use crate::parser::{MetadataParser};
use moka::future::Cache;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;

/// XNS Resolver - main entry point for resolving .xrp domains
#[derive(Clone)]
pub struct XnsResolver {
    client: Arc<XrplClient>,
    parser: Arc<MetadataParser>,
    cache: Cache<String, DomainInfo>,
    network: XrplNetwork,
    /// Rate limiter: max 10 concurrent metadata requests
    metadata_semaphore: Arc<Semaphore>,
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
            metadata_semaphore: Arc::new(Semaphore::new(10)),
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
            metadata_semaphore: Arc::new(Semaphore::new(10)),
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

        // OPTIMIZATION: Try Clio's nfts_by_issuer first (more efficient)
        let nfts = match self.client.nfts_by_issuer(issuer, None).await {
            Ok(nfts) => {
                tracing::debug!("Using Clio nfts_by_issuer: found {} NFTs from {:?}", nfts.len(), service);
                nfts
            }
            Err(e) => {
                tracing::warn!("Clio nfts_by_issuer failed ({}), falling back to account_nfts", e);
                // Fallback to account_nfts on issuer
                self.client.account_nfts(issuer).await?
            }
        };

        tracing::debug!("Processing {} NFTs from {:?}", nfts.len(), service);

        // OPTIMIZATION: Search for matching domain with throttling and early exit
        let target_domain_lower = domain.to_lowercase();

        for (idx, nft) in nfts.iter().enumerate() {
            if let Some(uri_hex) = &nft.uri {
                // Acquire semaphore permit for rate limiting
                let _permit = self.metadata_semaphore.acquire().await
                    .map_err(|e| XnsError::InternalError(format!("Semaphore error: {}", e)))?;

                // Small delay to avoid overwhelming the metadata server
                if idx > 0 && idx % 10 == 0 {
                    sleep(Duration::from_millis(100)).await;
                }

                match self.parser.parse_uri(uri_hex).await {
                    Ok(metadata) => {
                        if let Some(nft_domain) = MetadataParser::extract_domain_name(&metadata) {
                            if nft_domain.eq_ignore_ascii_case(&target_domain_lower) {
                                tracing::info!("✓ Found domain {} in NFT {} (checked {} NFTs)",
                                    domain, nft.nft_token_id, idx + 1);

                                // Get the actual owner (might not be issuer)
                                let owner = self.get_nft_owner(&nft.nft_token_id).await?;

                                let mut domain_info = DomainInfo {
                                    domain: nft_domain,
                                    owner,
                                    nft_id: nft.nft_token_id.clone(),
                                    service,
                                    addresses: Default::default(),
                                    text_records: Default::default(),
                                    expires_at: None, // TODO: Parse expiration from metadata
                                    metadata: Some(metadata),
                                };

                                // Enhance with API data if available
                                let _ = self.enhance_with_xrp_domains_api(&mut domain_info).await;

                                return Ok(domain_info);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Skipping NFT {}: {}", nft.nft_token_id, e);
                        continue;
                    }
                }
            }
        }

        tracing::warn!("Domain {} not found after checking {} NFTs from {:?}",
            domain, nfts.len(), service);

        Err(XnsError::DomainNotFound(domain.to_string()))
    }

    /// Get current owner of an NFT via Clio
    async fn get_nft_owner(&self, nft_id: &str) -> XnsResult<String> {
        match self.client.nft_info(nft_id).await {
            Ok(nft_info) => {
                if nft_info.is_burned {
                    return Err(XnsError::DomainNotFound(
                        "NFT has been burned".to_string(),
                    ));
                }
                Ok(nft_info.owner)
            }
            Err(e) => {
                tracing::warn!("Failed to get NFT owner via Clio: {}", e);
                // Fallback: return placeholder
                Ok("rUnknownOwner".to_string())
            }
        }
    }

    /// Fetch additional data from XRP Domains API if available
    async fn enhance_with_xrp_domains_api(&self, domain_info: &mut DomainInfo) -> XnsResult<()> {
        if domain_info.service != NamingService::XRPDomains {
            return Ok(());
        }

        let api_url = format!(
            "https://app.xrpdomains.xyz/api/xrplnft/getAddress?domain={}",
            domain_info.domain
        );

        tracing::debug!("Fetching XRP Domains API data for {}", domain_info.domain);

        match self.client.client.get(&api_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(api_data) = response.json::<serde_json::Value>().await {
                        if let Some(data) = api_data.get("data") {
                            // Extract addresses
                            if let Some(addresses) = data.get("addresses").and_then(|a| a.as_array()) {
                                for addr in addresses {
                                    if let (Some(symbol), Some(address)) = (
                                        addr.get("symbol").and_then(|s| s.as_str()),
                                        addr.get("address").and_then(|a| a.as_str()),
                                    ) {
                                        domain_info.addresses.insert(
                                            symbol.to_lowercase(),
                                            address.to_string(),
                                        );
                                    }
                                }
                            }

                            // Extract profile text records
                            if let Some(profile) = data.get("profile_info") {
                                if let Some(email) = profile.get("email").and_then(|e| e.as_str()) {
                                    domain_info.text_records.insert("email".to_string(), email.to_string());
                                }
                                if let Some(twitter) = profile.get("twitter").and_then(|t| t.as_str()) {
                                    domain_info.text_records.insert("twitter".to_string(), twitter.to_string());
                                }
                                if let Some(github) = profile.get("github").and_then(|g| g.as_str()) {
                                    domain_info.text_records.insert("github".to_string(), github.to_string());
                                }
                                if let Some(website) = profile.get("website").and_then(|w| w.as_str()) {
                                    domain_info.text_records.insert("website".to_string(), website.to_string());
                                }
                            }

                            tracing::info!("Enhanced {} with XRP Domains API data", domain_info.domain);
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                tracing::debug!("XRP Domains API failed: {}", e);
                Ok(()) // Don't fail if API is unavailable
            }
        }
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

    /// Build an unsigned transaction for storing blockchain addresses in XRPL memos
    ///
    /// This creates a transaction that the user must sign with their wallet.
    /// Once signed and submitted to XRPL, the addresses will be stored on-chain.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use xns_sdk_rs::{XnsResolver, XrplNetwork};
    /// use std::collections::HashMap;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let resolver = XnsResolver::new(XrplNetwork::Mainnet).await?;
    ///
    ///     let mut addresses = HashMap::new();
    ///     addresses.insert("BTC".to_string(), "bc1q...".to_string());
    ///     addresses.insert("ETH".to_string(), "0x...".to_string());
    ///
    ///     let tx_json = resolver.build_address_storage_tx(
    ///         "reRDmP8LxyYunhcfmQMnSjinKXV6duss6",
    ///         addresses
    ///     )?;
    ///
    ///     println!("Sign this transaction: {}", tx_json);
    ///     // User signs with XUMM, Crossmark, etc.
    ///
    ///     Ok(())
    /// }
    /// ```
    pub fn build_address_storage_tx(
        &self,
        account: &str,
        addresses: std::collections::HashMap<String, String>,
    ) -> XnsResult<String> {
        let memo_storage = crate::memo_storage::MemoStorage::new(
            (*self.client).clone()
        );

        memo_storage.build_storage_transaction(account, addresses)
    }

    /// Get addresses stored in XRPL memos for an account
    ///
    /// This queries the account's transaction history to find the latest
    /// XNS_ADDRESSES memo and returns the stored blockchain addresses.
    ///
    /// Note: This is a prototype implementation. Full functionality requires
    /// additional RPC methods.
    pub async fn get_memo_addresses(
        &self,
        account: &str,
    ) -> XnsResult<std::collections::HashMap<String, String>> {
        let memo_storage = crate::memo_storage::MemoStorage::new(
            (*self.client).clone()
        );

        memo_storage.get_addresses(account).await
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
