use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// XRPL network type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrplNetwork {
    Mainnet,
    Testnet,
    Devnet,
}

impl XrplNetwork {
    pub fn rpc_url(&self) -> &'static str {
        match self {
            XrplNetwork::Mainnet => "https://s1.ripple.com:51234",
            XrplNetwork::Testnet => "https://s.altnet.rippletest.net:51234",
            XrplNetwork::Devnet => "https://s.devnet.rippletest.net:51234",
        }
    }
}

/// Naming service type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NamingService {
    /// XRP Name Service (xrpns.com)
    XNS,
    /// XRP Domains (xrpdomains.xyz)
    XRPDomains,
}

impl NamingService {
    /// Get the known issuer address for this naming service
    pub fn issuer_address(&self, network: XrplNetwork) -> Option<&'static str> {
        match (self, network) {
            // XNS (xrpns.com) - Verified from ckelley.xrp NFT
            (NamingService::XNS, XrplNetwork::Mainnet) => {
                Some("rYhfynZDrde1uSvvQAYctApg6DnVE5HKm")
            }
            (NamingService::XNS, XrplNetwork::Testnet) => {
                // TODO: Find testnet issuer address
                None
            }
            // XRP Domains (xrpdomains.xyz) - From xrp.cafe research
            (NamingService::XRPDomains, XrplNetwork::Mainnet) => {
                Some("r4pM3nT7r7X1k2WMcSw5Sz8ftUu33TEfA4")
            }
            _ => None,
        }
    }
}

/// Domain information resolved from XRPL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainInfo {
    /// Domain name (e.g., "ckelley.xrp")
    pub domain: String,

    /// Owner XRPL address
    pub owner: String,

    /// NFT token ID
    pub nft_id: String,

    /// Naming service provider
    pub service: NamingService,

    /// Additional addresses (e.g., BTC, ETH)
    #[serde(default)]
    pub addresses: HashMap<String, String>,

    /// Text records
    #[serde(default)]
    pub text_records: HashMap<String, String>,

    /// Expiration timestamp (if any)
    pub expires_at: Option<u64>,

    /// Raw metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<NftMetadata>,
}

/// NFT metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMetadata {
    pub name: String,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub image: String,

    #[serde(default)]
    pub attributes: Vec<MetadataAttribute>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataAttribute {
    pub trait_type: String,
    pub value: serde_json::Value,
}

/// XRPL NFT from account_nfts response
#[derive(Debug, Clone, Deserialize)]
pub struct XrplNft {
    #[serde(rename = "NFTokenID")]
    pub nft_token_id: String,

    #[serde(rename = "URI")]
    pub uri: Option<String>,

    #[serde(rename = "Issuer")]
    pub issuer: Option<String>,
}

/// XRPL RPC request
#[derive(Debug, Serialize)]
pub struct RpcRequest {
    pub method: String,
    pub params: Vec<serde_json::Value>,
}

/// XRPL RPC response
#[derive(Debug, Deserialize)]
pub struct RpcResponse<T> {
    pub result: T,
}

/// account_nfts result
#[derive(Debug, Deserialize)]
pub struct AccountNftsResult {
    pub account: String,

    #[serde(rename = "account_nfts")]
    pub nfts: Vec<XrplNft>,

    #[serde(rename = "marker")]
    pub marker: Option<String>,
}

/// nft_info result (from Clio)
#[derive(Debug, Clone, Deserialize)]
pub struct NftInfo {
    pub nft_id: String,
    pub owner: String,
    pub is_burned: bool,

    #[serde(default)]
    pub uri: Option<String>,

    #[serde(default)]
    pub issuer: Option<String>,
}
