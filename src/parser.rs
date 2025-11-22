use crate::error::{XnsError, XnsResult};
use crate::models::NftMetadata;
use reqwest::Client;

/// NFT metadata parser
pub struct MetadataParser {
    client: Client,
}

impl MetadataParser {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Parse NFT URI and fetch metadata
    pub async fn parse_uri(&self, uri_hex: &str) -> XnsResult<NftMetadata> {
        // Decode hex-encoded URI
        let uri_bytes = hex::decode(uri_hex)?;
        let uri = String::from_utf8(uri_bytes).map_err(|e| {
            XnsError::ParseError(format!("Invalid UTF-8 in URI: {}", e))
        })?;

        tracing::debug!("Parsing NFT URI: {}", uri);

        // Determine URI type and fetch metadata
        if uri.starts_with("ipfs://") {
            self.fetch_from_ipfs(&uri).await
        } else if uri.starts_with("http://") || uri.starts_with("https://") {
            self.fetch_from_http(&uri).await
        } else if uri.starts_with("{") || uri.starts_with("[") {
            // Embedded JSON
            self.parse_embedded_json(&uri)
        } else {
            Err(XnsError::MetadataError(format!(
                "Unsupported URI format: {}",
                uri
            )))
        }
    }

    /// Fetch metadata from IPFS
    async fn fetch_from_ipfs(&self, uri: &str) -> XnsResult<NftMetadata> {
        // Convert ipfs:// to https gateway
        let cid = uri.strip_prefix("ipfs://").ok_or_else(|| {
            XnsError::MetadataError("Invalid IPFS URI".to_string())
        })?;

        // Use public IPFS gateways
        let gateways = [
            format!("https://ipfs.io/ipfs/{}", cid),
            format!("https://gateway.pinata.cloud/ipfs/{}", cid),
            format!("https://cloudflare-ipfs.com/ipfs/{}", cid),
        ];

        let mut last_error = None;

        for gateway_url in &gateways {
            match self.fetch_from_http(gateway_url).await {
                Ok(metadata) => return Ok(metadata),
                Err(e) => {
                    tracing::warn!("IPFS gateway {} failed: {}", gateway_url, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            XnsError::MetadataError("All IPFS gateways failed".to_string())
        }))
    }

    /// Fetch metadata from HTTP(S) URL
    async fn fetch_from_http(&self, url: &str) -> XnsResult<NftMetadata> {
        tracing::debug!("Fetching metadata from HTTP: {}", url);

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(XnsError::NetworkError(format!(
                "HTTP {}: Failed to fetch metadata",
                response.status()
            )));
        }

        let text = response.text().await?;
        self.parse_embedded_json(&text)
    }

    /// Parse embedded JSON metadata
    fn parse_embedded_json(&self, json_str: &str) -> XnsResult<NftMetadata> {
        serde_json::from_str(json_str).map_err(|e| {
            XnsError::ParseError(format!("Failed to parse metadata JSON: {}", e))
        })
    }

    /// Extract domain name from metadata
    pub fn extract_domain_name(metadata: &NftMetadata) -> Option<String> {
        // Try name field first
        if metadata.name.ends_with(".xrp") {
            return Some(metadata.name.clone());
        }

        // Check attributes
        for attr in &metadata.attributes {
            if attr.trait_type == "domain" || attr.trait_type == "name" {
                if let Some(domain) = attr.value.as_str() {
                    if domain.ends_with(".xrp") {
                        return Some(domain.to_string());
                    }
                }
            }
        }

        // Check extra fields
        if let Some(domain) = metadata.extra.get("domain") {
            if let Some(domain_str) = domain.as_str() {
                if domain_str.ends_with(".xrp") {
                    return Some(domain_str.to_string());
                }
            }
        }

        None
    }
}

impl Default for MetadataParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain_from_name() {
        let metadata = NftMetadata {
            name: "ckelley.xrp".to_string(),
            description: String::new(),
            image: String::new(),
            attributes: vec![],
            extra: Default::default(),
        };

        assert_eq!(
            MetadataParser::extract_domain_name(&metadata),
            Some("ckelley.xrp".to_string())
        );
    }

    #[test]
    fn test_hex_decode() {
        let hex_uri = hex::encode("https://example.com/metadata.json");
        let bytes = hex::decode(&hex_uri).unwrap();
        let uri = String::from_utf8(bytes).unwrap();
        assert_eq!(uri, "https://example.com/metadata.json");
    }
}
