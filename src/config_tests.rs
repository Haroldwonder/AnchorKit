#![cfg(test)]

use crate::{config::*, validation::validate_attestor_batch, Error};
use soroban_sdk::{Env, String, Vec};

#[test]
fn test_contract_config_validation() {
    let env = Env::default();

    let valid = ContractConfig {
        name: String::from_str(&env, "test-anchor"),
        version: String::from_str(&env, "1.0.0"),
        network: String::from_str(&env, "testnet"),
    };
    assert!(valid.validate().is_ok());

    let empty_name = ContractConfig {
        name: String::from_str(&env, ""),
        version: String::from_str(&env, "1.0.0"),
        network: String::from_str(&env, "testnet"),
    };
    assert_eq!(empty_name.validate(), Err(Error::InvalidConfigName));

    let long_name = ContractConfig {
        name: String::from_str(&env, "a".repeat(65).as_str()),
        version: String::from_str(&env, "1.0.0"),
        network: String::from_str(&env, "testnet"),
    };
    assert_eq!(long_name.validate(), Err(Error::InvalidConfigName));
}

#[test]
fn test_attestor_config_validation() {
    let env = Env::default();

    let valid = AttestorConfig {
        name: String::from_str(&env, "kyc-provider"),
        address: String::from_str(&env, "GBBD6A7KNZF5WNWQEPZP5DYJD2AYUTLXRB6VXJ4RCX4RTNPPQVNF3GQ"),
        endpoint: String::from_str(&env, "https://api.example.com/verify"),
        role: String::from_str(&env, "kyc-issuer"),
        enabled: true,
    };
    assert!(valid.validate().is_ok());

    let invalid_address = AttestorConfig {
        name: String::from_str(&env, "kyc-provider"),
        address: String::from_str(&env, "INVALID"),
        endpoint: String::from_str(&env, "https://api.example.com/verify"),
        role: String::from_str(&env, "kyc-issuer"),
        enabled: true,
    };
    assert_eq!(invalid_address.validate(), Err(Error::InvalidAttestorAddress));

    let invalid_endpoint = AttestorConfig {
        name: String::from_str(&env, "kyc-provider"),
        address: String::from_str(&env, "GBBD6A7KNZF5WNWQEPZP5DYJD2AYUTLXRB6VXJ4RCX4RTNPPQVNF3GQ"),
        endpoint: String::from_str(&env, "bad"),
        role: String::from_str(&env, "kyc-issuer"),
        enabled: true,
    };
    assert_eq!(invalid_endpoint.validate(), Err(Error::InvalidEndpointFormat));
}

#[test]
fn test_session_config_validation() {
    let env = Env::default();

    let valid = SessionConfig {
        enable_tracking: true,
        timeout_seconds: 3600,
        max_operations: 1000,
    };
    assert!(valid.validate().is_ok());

    let zero_timeout = SessionConfig {
        enable_tracking: true,
        timeout_seconds: 0,
        max_operations: 1000,
    };
    assert_eq!(zero_timeout.validate(), Err(Error::InvalidConfig));

    let excessive_timeout = SessionConfig {
        enable_tracking: true,
        timeout_seconds: 86401,
        max_operations: 1000,
    };
    assert_eq!(excessive_timeout.validate(), Err(Error::InvalidConfig));

    let zero_operations = SessionConfig {
        enable_tracking: true,
        timeout_seconds: 3600,
        max_operations: 0,
    };
    assert_eq!(zero_operations.validate(), Err(Error::InvalidConfig));
}

#[test]
fn test_batch_attestor_validation() {
    let env = Env::default();

    let mut valid_attestors = Vec::new(&env);
    valid_attestors.push_back(AttestorConfig {
        name: String::from_str(&env, "attestor1"),
        address: String::from_str(&env, "GBBD6A7KNZF5WNWQEPZP5DYJD2AYUTLXRB6VXJ4RCX4RTNPPQVNF3GQ"),
        endpoint: String::from_str(&env, "https://api1.example.com"),
        role: String::from_str(&env, "issuer"),
        enabled: true,
    });
    assert!(validate_attestor_batch(&valid_attestors).is_ok());

    let empty_attestors = Vec::new(&env);
    assert_eq!(validate_attestor_batch(&empty_attestors), Err(Error::NoEnabledAttestors));
}
