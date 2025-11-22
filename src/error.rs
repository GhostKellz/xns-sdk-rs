use thiserror::Error;

pub type XnsResult<T> = Result<T, XnsError>;

#[derive(Error, Debug)]
pub enum XnsError {
    #[error("Domain not found: {0}")]
    DomainNotFound(String),

    #[error("Invalid domain format: {0}")]
    InvalidDomain(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("XRPL RPC error: {0}")]
    RpcError(String),

    #[error("NFT metadata error: {0}")]
    MetadataError(String),

    #[error("Unsupported naming service: {0}")]
    UnsupportedService(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl From<reqwest::Error> for XnsError {
    fn from(err: reqwest::Error) -> Self {
        XnsError::NetworkError(err.to_string())
    }
}

impl From<serde_json::Error> for XnsError {
    fn from(err: serde_json::Error) -> Self {
        XnsError::ParseError(err.to_string())
    }
}

impl From<hex::FromHexError> for XnsError {
    fn from(err: hex::FromHexError) -> Self {
        XnsError::ParseError(format!("Hex decode error: {}", err))
    }
}
