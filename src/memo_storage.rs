///! Memo-based on-chain address storage for XNS domains
///!
///! This module provides functionality to store and retrieve blockchain addresses
///! for .xrp domains using XRPL transaction memos. This is a decentralized storage
///! solution that doesn't require external infrastructure.
///!
///! ## How It Works
///!
///! 1. User signs an XRPL transaction with a memo containing their address mappings
///! 2. Transaction is sent to self (1 drop XRP payment)
///! 3. Memo contains JSON: `{"BTC":"bc1q...", "ETH":"0x...", ...}`
///! 4. SDK queries account transactions and finds latest XNS_ADDRESSES memo
///!
///! ## Example
///!
///! ```no_run
///! use xns_sdk_rs::{XnsResolver, XrplNetwork};
///! use std::collections::HashMap;
///!
///! #[tokio::main]
///! async fn main() -> Result<(), Box<dyn std::error::Error>> {
///!     let resolver = XnsResolver::new(XrplNetwork::Mainnet).await?;
///!
///!     // Get addresses stored in memos
///!     let addresses = resolver.get_memo_addresses("reRDmP8LxyYunhcfmQMnSjinKXV6duss6").await?;
///!
///!     println!("BTC: {:?}", addresses.get("BTC"));
///!     println!("ETH: {:?}", addresses.get("ETH"));
///!
///!     Ok(())
///! }
///! ```

use crate::error::{XnsError, XnsResult};
use crate::client::XrplClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memo type identifier for XNS address records
pub const XNS_ADDRESSES_MEMO_TYPE: &str = "XNS_ADDRESSES";

/// Address record stored in XRPL memo
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressRecord {
    /// Blockchain symbol (BTC, ETH, SOL, etc.)
    pub symbol: String,

    /// Address on that blockchain
    pub address: String,

    /// Optional label/description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Transaction memo structure
#[derive(Debug, Serialize)]
pub struct TransactionMemo {
    #[serde(rename = "Memo")]
    pub memo: MemoData,
}

#[derive(Debug, Serialize)]
pub struct MemoData {
    #[serde(rename = "MemoType")]
    pub memo_type: String,

    #[serde(rename = "MemoData")]
    pub memo_data: String,
}

/// Transaction for storing address records
#[derive(Debug, Serialize)]
pub struct AddressStorageTransaction {
    #[serde(rename = "TransactionType")]
    pub transaction_type: String,

    #[serde(rename = "Account")]
    pub account: String,

    #[serde(rename = "Destination")]
    pub destination: String,

    #[serde(rename = "Amount")]
    pub amount: String,

    #[serde(rename = "Memos")]
    pub memos: Vec<TransactionMemo>,
}

impl AddressStorageTransaction {
    /// Create a new transaction for storing addresses
    pub fn new(account: String, addresses: HashMap<String, String>) -> XnsResult<Self> {
        // Convert addresses to JSON
        let addresses_json = serde_json::to_string(&addresses)
            .map_err(|e| XnsError::InvalidInput(format!("Failed to serialize addresses: {}", e)))?;

        // Hex-encode the JSON (XRPL requirement)
        let memo_data_hex = hex::encode(addresses_json.as_bytes());
        let memo_type_hex = hex::encode(XNS_ADDRESSES_MEMO_TYPE.as_bytes());

        Ok(Self {
            transaction_type: "Payment".to_string(),
            account: account.clone(),
            destination: account, // Self-payment
            amount: "1".to_string(), // 1 drop XRP (0.000001 XRP)
            memos: vec![TransactionMemo {
                memo: MemoData {
                    memo_type: memo_type_hex,
                    memo_data: memo_data_hex,
                },
            }],
        })
    }
}

/// Memo storage handler
pub struct MemoStorage {
    client: XrplClient,
}

impl MemoStorage {
    /// Create a new memo storage handler
    pub fn new(client: XrplClient) -> Self {
        Self { client }
    }

    /// Build an unsigned transaction for storing addresses
    ///
    /// This transaction should be signed by the user's wallet and submitted to XRPL
    pub fn build_storage_transaction(
        &self,
        account: &str,
        addresses: HashMap<String, String>,
    ) -> XnsResult<String> {
        let tx = AddressStorageTransaction::new(account.to_string(), addresses)?;
        let tx_json = serde_json::to_string_pretty(&tx)
            .map_err(|e| XnsError::InvalidInput(format!("Failed to serialize transaction: {}", e)))?;

        Ok(tx_json)
    }

    /// Query account transactions to find latest XNS_ADDRESSES memo
    pub async fn get_addresses(&self, account: &str) -> XnsResult<HashMap<String, String>> {
        // Query account transactions
        let tx_response = self.client.account_info(account).await?;

        // For now, return empty - full implementation would:
        // 1. Use account_tx RPC method to get transactions
        // 2. Parse transaction memos
        // 3. Find latest XNS_ADDRESSES memo
        // 4. Decode hex and parse JSON

        tracing::warn!("Memo address retrieval not yet fully implemented");
        Ok(HashMap::new())
    }

    /// Decode a hex-encoded memo
    pub fn decode_memo(memo_hex: &str) -> XnsResult<String> {
        let bytes = hex::decode(memo_hex)
            .map_err(|e| XnsError::InvalidInput(format!("Invalid hex memo: {}", e)))?;

        String::from_utf8(bytes)
            .map_err(|e| XnsError::InvalidInput(format!("Invalid UTF-8 in memo: {}", e)))
    }

    /// Parse address data from decoded memo
    pub fn parse_addresses(memo_data: &str) -> XnsResult<HashMap<String, String>> {
        serde_json::from_str(memo_data)
            .map_err(|e| XnsError::InvalidInput(format!("Invalid address JSON: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_transaction() {
        let mut addresses = HashMap::new();
        addresses.insert("BTC".to_string(), "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh".to_string());
        addresses.insert("ETH".to_string(), "0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb".to_string());

        let tx = AddressStorageTransaction::new("reRDmP8LxyYunhcfmQMnSjinKXV6duss6".to_string(), addresses);
        assert!(tx.is_ok());

        let tx = tx.unwrap();
        assert_eq!(tx.transaction_type, "Payment");
        assert_eq!(tx.amount, "1");
        assert_eq!(tx.account, tx.destination);
    }

    #[test]
    fn test_decode_memo() {
        let data = r#"{"BTC":"bc1q...","ETH":"0x..."}"#;
        let hex = hex::encode(data.as_bytes());

        let decoded = MemoStorage::decode_memo(&hex).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_parse_addresses() {
        let json = r#"{"BTC":"bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh","ETH":"0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb"}"#;
        let addresses = MemoStorage::parse_addresses(json).unwrap();

        assert_eq!(addresses.len(), 2);
        assert!(addresses.contains_key("BTC"));
        assert!(addresses.contains_key("ETH"));
    }
}
