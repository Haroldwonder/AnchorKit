use soroban_sdk::{contracttype, String};

use crate::domain_validator;
use crate::errors::Error;

/// Network type for SDK operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Network {
    Testnet,
    Mainnet,
}

/// SDK configuration for anchor operations
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SdkConfig {
    pub network: Network,
    pub timeout_seconds: u32,
    pub retry_attempts: u32,
    pub default_anchor: String,
}

// Validation constants
const MIN_TIMEOUT: u32 = 5;
const MAX_TIMEOUT: u32 = 300;
const MAX_RETRY: u32 = 10;
const MIN_ANCHOR_LEN: u32 = 3;
pub const MAX_ANCHOR_LEN: u32 = 256;

/// Default timeout for HTTP requests (10 seconds)
pub const DEFAULT_TIMEOUT_SECONDS: u32 = 10;

impl SdkConfig {
    pub fn new(
        network: Network,
        timeout_seconds: u32,
        retry_attempts: u32,
        default_anchor: String,
    ) -> Result<Self, Error> {
        let config = Self {
            network,
            timeout_seconds,
            retry_attempts,
            default_anchor,
        };
        config.validate()?;
        Ok(config)
    }

    /// Create a new SdkConfig with default timeout
    pub fn with_defaults(network: Network, default_anchor: String) -> Result<Self, Error> {
        Self::new(network, DEFAULT_TIMEOUT_SECONDS, 3, default_anchor)
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.timeout_seconds < MIN_TIMEOUT || self.timeout_seconds > MAX_TIMEOUT {
            return Err(Error::invalid_config());
        }

        if self.retry_attempts > MAX_RETRY {
            return Err(Error::invalid_config());
        }

        let anchor_len = self.default_anchor.len();
        if anchor_len < MIN_ANCHOR_LEN || anchor_len > MAX_ANCHOR_LEN {
            return Err(Error::invalid_config());
        }

        // Copy the Soroban String bytes into a plain &str and run full
        // domain-format validation (HTTPS-only, valid domain structure, no
        // path traversal, etc.).  This catches garbage values like "abc" or
        // "not a domain!!" that pass the length check but are unusable as
        // real anchor endpoints.
        let len = anchor_len as usize;
        let mut buf = [0u8; MAX_ANCHOR_LEN as usize];
        self.default_anchor.copy_into_slice(&mut buf[..len]);
        let anchor_str = core::str::from_utf8(&buf[..len]).map_err(|_| Error::invalid_config())?;
        domain_validator::validate_anchor_domain(anchor_str).map_err(|_| Error::invalid_config())?;

        Ok(())
    }
}
