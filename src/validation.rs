use soroban_sdk::Vec;
use crate::config::{AttestorConfig, ContractConfig, SessionConfig, MAX_ATTESTORS, MIN_ATTESTORS};
use crate::Error;

/// Strict pre-runtime validation utilities
/// Prevents misconfiguration bugs before contract execution

/// Validate configuration at initialization time with strict checks
pub fn validate_init_config(config: &ContractConfig) -> Result<(), Error> {
    config.validate()?;
    Ok(())
}

/// Validate attestor batch with comprehensive cross-validation
pub fn validate_attestor_batch(attestors: &Vec<AttestorConfig>) -> Result<(), Error> {
    let len = attestors.len();
    
    if len < MIN_ATTESTORS {
        return Err(Error::NoEnabledAttestors);
    }
    
    if len > MAX_ATTESTORS {
        return Err(Error::InvalidConfig);
    }
    
    let mut has_enabled = false;
    
    // Validate each attestor and check for duplicates
    for i in 0..len {
        let attestor = attestors.get(i).unwrap();
        
        // Individual validation
        attestor.validate()?;
        
        if attestor.enabled {
            has_enabled = true;
        }
        
        // Check for duplicate names and addresses
        for j in (i + 1)..len {
            let other = attestors.get(j).unwrap();
            
            if attestor.name == other.name {
                return Err(Error::DuplicateAttestor);
            }
            
            if attestor.address == other.address {
                return Err(Error::DuplicateAttestor);
            }
        }
    }
    
    if !has_enabled {
        return Err(Error::NoEnabledAttestors);
    }
    
    Ok(())
}

/// Validate session configuration with strict business rules
pub fn validate_session_config(config: &SessionConfig) -> Result<(), Error> {
    config.validate()?;
    
    // Prevent excessive operations per session (security limit)
    if config.max_operations > 5000 {
        return Err(Error::InvalidConfig);
    }
    
    // Minimum timeout for security (prevent rapid session cycling)
    if config.timeout_seconds < 60 {
        return Err(Error::InvalidConfig);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, String};
    
    #[test]
    fn test_validate_init_config_valid() {
        let env = Env::default();
        let config = ContractConfig {
            name: String::from_str(&env, "my-anchor"),
            version: String::from_str(&env, "1.0.0"),
            network: String::from_str(&env, "stellar-testnet"),
        };
        
        assert!(validate_init_config(&config).is_ok());
    }
    
    #[test]
    fn test_validate_init_config_invalid_name() {
        let env = Env::default();
        let config = ContractConfig {
            name: String::from_str(&env, ""),
            version: String::from_str(&env, "1.0.0"),
            network: String::from_str(&env, "stellar-testnet"),
        };
        
        assert_eq!(validate_init_config(&config), Err(Error::InvalidConfigName));
    }
    
    #[test]
    fn test_validate_attestor_batch_duplicates() {
        let env = Env::default();
        let mut attestors = Vec::new(&env);
        
        let attestor1 = AttestorConfig {
            name: String::from_str(&env, "attestor-1"),
            address: String::from_str(&env, "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF"),
            endpoint: String::from_str(&env, "https://example.com"),
            role: String::from_str(&env, "kyc-issuer"),
            enabled: true,
        };
        
        let attestor2 = AttestorConfig {
            name: String::from_str(&env, "attestor-1"), // Duplicate name
            address: String::from_str(&env, "GBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB"),
            endpoint: String::from_str(&env, "https://example2.com"),
            role: String::from_str(&env, "attestor"),
            enabled: true,
        };
        
        attestors.push_back(attestor1);
        attestors.push_back(attestor2);
        
        assert_eq!(validate_attestor_batch(&attestors), Err(Error::DuplicateAttestor));
    }
    
    #[test]
    fn test_validate_session_config_valid() {
        let env = Env::default();
        let config = SessionConfig {
            enable_tracking: true,
            timeout_seconds: 3600,
            max_operations: 1000,
        };
        
        assert!(validate_session_config(&config).is_ok());
    }
    
    #[test]
    fn test_validate_session_config_excessive_operations() {
        let env = Env::default();
        let config = SessionConfig {
            enable_tracking: true,
            timeout_seconds: 3600,
            max_operations: 6000, // Exceeds 5000 limit
        };
        
        assert_eq!(validate_session_config(&config), Err(Error::InvalidConfig));
    }
}
