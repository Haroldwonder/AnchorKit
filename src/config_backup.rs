use soroban_sdk::{contracttype, String, Vec};

use crate::Error;

/// Validated configuration for contract initialization
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractConfig {
    pub name: String,
    pub version: String,
    pub network: String,
}

/// Compile-time constants for validation
pub const MAX_NAME_LEN: u32 = 64;
pub const MIN_NAME_LEN: u32 = 1;
pub const MAX_VERSION_LEN: u32 = 16;
pub const MIN_VERSION_LEN: u32 = 1;
pub const MAX_NETWORK_LEN: u32 = 32;
pub const MIN_NETWORK_LEN: u32 = 1;
pub const MAX_ENDPOINT_LEN: u32 = 256;
pub const MIN_ENDPOINT_LEN: u32 = 8;
pub const STELLAR_ADDR_MIN: u32 = 54;
pub const STELLAR_ADDR_MAX: u32 = 56;
pub const MAX_ATTESTORS: u32 = 100;
pub const MIN_ATTESTORS: u32 = 1;
pub const MAX_SESSION_TIMEOUT: u64 = 86400;
pub const MIN_SESSION_TIMEOUT: u64 = 60;
pub const MAX_OPERATIONS: u64 = 10000;
pub const MIN_OPERATIONS: u64 = 1;
pub const MAX_ROLE_LEN: u32 = 32;
pub const MIN_ROLE_LEN: u32 = 1;

/// Validated attestor configuration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttestorConfig {
    pub name: String,
    pub address: String,
    pub endpoint: String,
    pub role: String,
    pub enabled: bool,
}

/// Validated session configuration
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionConfig {
    pub enable_tracking: bool,
    pub timeout_seconds: u64,
    pub max_operations: u64,
}

impl ContractConfig {
    /// Strict validation with detailed error checking
    pub fn validate(&self) -> Result<(), Error> {
        // Name validation
        let name_len = self.name.len();
        if name_len < MIN_NAME_LEN || name_len > MAX_NAME_LEN {
            return Err(Error::InvalidConfigName);
        }
        
        if !Self::is_valid_name(&self.name) {
            return Err(Error::InvalidConfigName);
        }
        
        // Version validation
        let version_len = self.version.len();
        if version_len < MIN_VERSION_LEN || version_len > MAX_VERSION_LEN {
            return Err(Error::InvalidConfigVersion);
        }
        
        if !Self::is_valid_version(&self.version) {
            return Err(Error::InvalidConfigVersion);
        }
        
        // Network validation
        let network_len = self.network.len();
        if network_len < MIN_NETWORK_LEN || network_len > MAX_NETWORK_LEN {
            return Err(Error::InvalidConfigNetwork);
        }
        
        if !Self::is_valid_network(&self.network) {
            return Err(Error::InvalidConfigNetwork);
        }
        
        Ok(())
    }
    
    fn is_valid_name(name: &String) -> bool {
        let s = name.to_string();
        if s.is_empty() {
            return false;
        }
        
        // Must start with lowercase letter
        if !s.chars().next().unwrap().is_ascii_lowercase() {
            return false;
        }
        
        // Only lowercase, digits, and hyphens allowed
        s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }
    
    fn is_valid_version(version: &String) -> bool {
        let s = version.to_string();
        let parts: Vec<&str> = s.split('.').collect();
        
        if parts.len() != 3 {
            return false;
        }
        
        // Each part must be non-empty and numeric
        for part in parts {
            if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
            
            // Prevent leading zeros (except "0" itself)
            if part.len() > 1 && part.starts_with('0') {
                return false;
            }
        }
        
        true
    }
    
    fn is_valid_network(network: &String) -> bool {
        let s = network.to_string();
        matches!(s.as_str(), "stellar-testnet" | "stellar-mainnet" | "stellar-futurenet")
    }
}

impl AttestorConfig {
    pub fn validate(&self) -> Result<(), Error> {
        let name_len = self.name.len();
        if name_len < MIN_NAME_LEN || name_len > MAX_NAME_LEN {
            return Err(Error::InvalidAttestorName);
        }
        
        if !Self::is_valid_name(&self.name) {
            return Err(Error::InvalidAttestorName);
        }
        
        let addr_len = self.address.len();
        if addr_len < STELLAR_ADDR_MIN || addr_len > STELLAR_ADDR_MAX {
            return Err(Error::InvalidAttestorAddress);
        }
        
        if !Self::is_valid_stellar_address(&self.address) {
            return Err(Error::InvalidAttestorAddress);
        }
        
        let endpoint_len = self.endpoint.len();
        if endpoint_len < MIN_ENDPOINT_LEN || endpoint_len > MAX_ENDPOINT_LEN {
            return Err(Error::InvalidEndpointFormat);
        }
        
        if !Self::is_valid_endpoint(&self.endpoint) {
            return Err(Error::InvalidEndpointFormat);
        }
        
        let role_len = self.role.len();
        if role_len < MIN_ROLE_LEN || role_len > MAX_ROLE_LEN {
            return Err(Error::InvalidAttestorRole);
        }
        
        if !Self::is_valid_role(&self.role) {
            return Err(Error::InvalidAttestorRole);
        }
        
        Ok(())
    }
    
    fn is_valid_name(name: &String) -> bool {
        let s = name.to_string();
        !s.is_empty() && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }
    
    fn is_valid_stellar_address(addr: &String) -> bool {
        let s = addr.to_string();
        if !s.starts_with('G') {
            return false;
        }
        s.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    }
    
    fn is_valid_endpoint(endpoint: &String) -> bool {
        let s = endpoint.to_string();
        (s.starts_with("http://") || s.starts_with("https://")) && s.len() >= MIN_ENDPOINT_LEN as usize
    }
    
    fn is_valid_role(role: &String) -> bool {
        let s = role.to_string();
        matches!(s.as_str(), "kyc-issuer" | "transfer-verifier" | "compliance-approver" | "rate-provider" | "attestor")
    }
}

impl SessionConfig {
    pub fn validate(&self) -> Result<(), Error> {
        if self.timeout_seconds < MIN_SESSION_TIMEOUT || self.timeout_seconds > MAX_SESSION_TIMEOUT {
            return Err(Error::InvalidConfig);
        }
        
        if self.max_operations < MIN_OPERATIONS || self.max_operations > MAX_OPERATIONS {
            return Err(Error::InvalidConfig);
        }
        
        Ok(())
    }
}

/// Batch validation for multiple attestors
pub fn validate_attestors(attestors: &Vec<AttestorConfig>) -> Result<(), Error> {
    let len = attestors.len();
    if len < MIN_ATTESTORS as usize || len > MAX_ATTESTORS as usize {
        return Err(Error::InvalidConfig);
    }
    
    let mut names = Vec::new();
    let mut addresses = Vec::new();
    let mut has_enabled = false;
    
    for i in 0..len {
        let attestor = attestors.get(i).unwrap();
        attestor.validate()?;
        
        // Check for duplicate names
        for j in 0..names.len() {
            if names.get(j).unwrap() == &attestor.name {
                return Err(Error::DuplicateAttestor);
            }
        }
        names.push_back(attestor.name.clone());
        
        // Check for duplicate addresses
        for j in 0..addresses.len() {
            if addresses.get(j).unwrap() == &attestor.address {
                return Err(Error::DuplicateAttestor);
            }
        }
        addresses.push_back(attestor.address.clone());
        
        if attestor.enabled {
            has_enabled = true;
        }
    }
    
    if !has_enabled {
        return Err(Error::NoEnabledAttestors);
    }
    
    Ok(())
}
