use crate::error::{XnsError, XnsResult};
use crate::models::*;
use reqwest::Client;
use serde_json::json;

pub use crate::models::XrplNetwork;

/// XRPL RPC client
pub struct XrplClient {
    client: Client,
    rpc_url: String,
    network: XrplNetwork,
}

impl XrplClient {
    /// Create a new XRPL client
    pub fn new(network: XrplNetwork) -> Self {
        Self {
            client: Client::new(),
            rpc_url: network.rpc_url().to_string(),
            network,
        }
    }

    /// Create with custom RPC URL
    pub fn with_url(network: XrplNetwork, rpc_url: String) -> Self {
        Self {
            client: Client::new(),
            rpc_url,
            network,
        }
    }

    /// Get network type
    pub fn network(&self) -> XrplNetwork {
        self.network
    }

    /// Query NFTs for an account
    pub async fn account_nfts(&self, account: &str) -> XnsResult<Vec<XrplNft>> {
        let mut all_nfts = Vec::new();
        let mut marker: Option<String> = None;

        loop {
            let mut params = json!({
                "account": account,
                "limit": 400,
                "ledger_index": "validated"
            });

            if let Some(m) = &marker {
                params["marker"] = json!(m);
            }

            let request = RpcRequest {
                method: "account_nfts".to_string(),
                params: vec![params],
            };

            tracing::debug!("Querying XRPL: account_nfts for {}", account);

            let response = self
                .client
                .post(&self.rpc_url)
                .json(&request)
                .send()
                .await?;

            if !response.status().is_success() {
                return Err(XnsError::RpcError(format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                )));
            }

            let rpc_response: RpcResponse<AccountNftsResult> = response.json().await?;
            all_nfts.extend(rpc_response.result.nfts);

            marker = rpc_response.result.marker;
            if marker.is_none() {
                break;
            }
        }

        Ok(all_nfts)
    }

    /// Get account info
    pub async fn account_info(&self, account: &str) -> XnsResult<serde_json::Value> {
        let request = RpcRequest {
            method: "account_info".to_string(),
            params: vec![json!({
                "account": account,
                "ledger_index": "validated"
            })],
        };

        let response = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(XnsError::RpcError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let rpc_response: RpcResponse<serde_json::Value> = response.json().await?;
        Ok(rpc_response.result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = XrplClient::new(XrplNetwork::Mainnet);
        assert_eq!(client.network(), XrplNetwork::Mainnet);
        assert_eq!(client.rpc_url, "https://s1.ripple.com:51234");
    }
}
