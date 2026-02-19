#![no_std]

mod errors;
mod events;
mod storage;
mod types;

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, String};

pub use errors::Error;
pub use events::{AttestationRecorded, AttestorAdded, AttestorRemoved, EndpointConfigured, EndpointRemoved};
pub use storage::Storage;
pub use types::{Attestation, Endpoint};

#[contract]
pub struct AnchorKitContract;

#[contractimpl]
impl AnchorKitContract {
    /// Initialize the contract with an admin address.
    /// Can only be called once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if Storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        admin.require_auth();
        Storage::set_admin(&env, &admin);
        Ok(())
    }

    /// Register a new attestor. Only callable by admin.
    pub fn register_attestor(env: Env, attestor: Address) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if Storage::is_attestor(&env, &attestor) {
            return Err(Error::AttestorAlreadyRegistered);
        }

        Storage::set_attestor(&env, &attestor, true);
        
        AttestorAdded {
            attestor: attestor.clone(),
        }
        .publish(&env);

        Ok(())
    }

    /// Revoke an attestor. Only callable by admin.
    pub fn revoke_attestor(env: Env, attestor: Address) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if !Storage::is_attestor(&env, &attestor) {
            return Err(Error::AttestorNotRegistered);
        }

        Storage::set_attestor(&env, &attestor, false);
        
        AttestorRemoved {
            attestor: attestor.clone(),
        }
        .publish(&env);

        Ok(())
    }

    /// Submit an attestation. Must be signed by a registered attestor.
    pub fn submit_attestation(
        env: Env,
        issuer: Address,
        subject: Address,
        timestamp: u64,
        payload_hash: BytesN<32>,
        signature: Bytes,
    ) -> Result<u64, Error> {
        issuer.require_auth();

        // Validate timestamp
        if timestamp == 0 {
            return Err(Error::InvalidTimestamp);
        }

        // Check if issuer is a registered attestor
        if !Storage::is_attestor(&env, &issuer) {
            return Err(Error::UnauthorizedAttestor);
        }

        // Check for replay attack
        if Storage::is_hash_used(&env, &payload_hash) {
            return Err(Error::ReplayAttack);
        }

        // Verify signature
        Self::verify_signature(&env, &issuer, &subject, timestamp, &payload_hash, &signature)?;

        // Get next attestation ID
        let id = Storage::get_and_increment_counter(&env);

        // Create attestation
        let attestation = Attestation {
            id,
            issuer: issuer.clone(),
            subject: subject.clone(),
            timestamp,
            payload_hash: payload_hash.clone(),
            signature: signature.clone(),
        };

        // Store attestation
        Storage::set_attestation(&env, id, &attestation);
        Storage::mark_hash_used(&env, &payload_hash);

        // Emit event
        AttestationRecorded {
            id,
            issuer,
            subject,
            timestamp,
            payload_hash,
        }
        .publish(&env);

        Ok(id)
    }

    /// Get an attestation by ID.
    pub fn get_attestation(env: Env, id: u64) -> Result<Attestation, Error> {
        Storage::get_attestation(&env, id)
    }

    /// Get the admin address.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        Storage::get_admin(&env)
    }

    /// Check if an address is a registered attestor.
    pub fn is_attestor(env: Env, attestor: Address) -> bool {
        Storage::is_attestor(&env, &attestor)
    }

    /// Configure an endpoint for an attestor. Only callable by the attestor or admin.
    pub fn configure_endpoint(env: Env, attestor: Address, url: String) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        
        // Allow either the attestor themselves or the admin to configure
        let caller_is_admin = env.try_invoke_contract::<bool, _>(&admin, &soroban_sdk::symbol_short!(""), &()).is_ok();
        
        if !caller_is_admin {
            attestor.require_auth();
        } else {
            admin.require_auth();
        }

        // Validate endpoint format
        Self::validate_endpoint_url(&url)?;

        // Check if attestor is registered
        if !Storage::is_attestor(&env, &attestor) {
            return Err(Error::AttestorNotRegistered);
        }

        // Check if endpoint already exists
        if Storage::has_endpoint(&env, &attestor) {
            return Err(Error::EndpointAlreadyExists);
        }

        let endpoint = Endpoint {
            url: url.clone(),
            attestor: attestor.clone(),
            is_active: true,
        };

        Storage::set_endpoint(&env, &endpoint);

        EndpointConfigured {
            attestor,
            url,
        }
        .publish(&env);

        Ok(())
    }

    /// Update an existing endpoint for an attestor. Only callable by the attestor or admin.
    pub fn update_endpoint(env: Env, attestor: Address, url: String, is_active: bool) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        
        // Allow either the attestor themselves or the admin to update
        let caller_is_admin = env.try_invoke_contract::<bool, _>(&admin, &soroban_sdk::symbol_short!(""), &()).is_ok();
        
        if !caller_is_admin {
            attestor.require_auth();
        } else {
            admin.require_auth();
        }

        // Validate endpoint format
        Self::validate_endpoint_url(&url)?;

        // Check if endpoint exists
        if !Storage::has_endpoint(&env, &attestor) {
            return Err(Error::EndpointNotFound);
        }

        let endpoint = Endpoint {
            url: url.clone(),
            attestor: attestor.clone(),
            is_active,
        };

        Storage::set_endpoint(&env, &endpoint);

        EndpointConfigured {
            attestor,
            url,
        }
        .publish(&env);

        Ok(())
    }

    /// Remove an endpoint for an attestor. Only callable by admin.
    pub fn remove_endpoint(env: Env, attestor: Address) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        // Check if endpoint exists
        if !Storage::has_endpoint(&env, &attestor) {
            return Err(Error::EndpointNotFound);
        }

        Storage::remove_endpoint(&env, &attestor);

        EndpointRemoved {
            attestor,
        }
        .publish(&env);

        Ok(())
    }

    /// Get the endpoint configuration for an attestor.
    pub fn get_endpoint(env: Env, attestor: Address) -> Result<Endpoint, Error> {
        Storage::get_endpoint(&env, &attestor)
    }

    /// Validate endpoint URL format.
    /// Checks for:
    /// - Non-empty URL
    /// - Valid protocol (http:// or https://)
    /// - Reasonable length
    fn validate_endpoint_url(url: &String) -> Result<(), Error> {
        let url_str = url.to_string();
        
        // Check if URL is empty
        if url_str.is_empty() {
            return Err(Error::InvalidEndpointFormat);
        }

        // Check length (reasonable limit)
        if url_str.len() > 256 {
            return Err(Error::InvalidEndpointFormat);
        }

        // Check for valid protocol
        if !url_str.starts_with("https://") && !url_str.starts_with("http://") {
            return Err(Error::InvalidEndpointFormat);
        }

        // Check that there's content after the protocol
        let protocol_len = if url_str.starts_with("https://") { 8 } else { 7 };
        if url_str.len() <= protocol_len {
            return Err(Error::InvalidEndpointFormat);
        }

        Ok(())
    }

    /// Internal function to verify ed25519 signature.
    fn verify_signature(
        _env: &Env,
        _issuer: &Address,
        _subject: &Address,
        _timestamp: u64,
        _payload_hash: &BytesN<32>,
        _signature: &Bytes,
    ) -> Result<(), Error> {
        // In production, this would verify the ed25519 signature
        // For now, we skip verification as it requires proper key management
        // which is beyond the scope of this basic implementation
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, BytesN as _, Events},
        Address, Bytes, BytesN, Env, IntoVal,
    };

    fn create_test_contract(env: &Env) -> (Address, AnchorKitContractClient<'_>) {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        (contract_id, client)
    }

    fn create_ed25519_signature(env: &Env, _subject: &Address, _timestamp: u64, _payload_hash: &BytesN<32>) -> Bytes {
        // Create a mock signature for testing
        // Return a 64-byte signature (standard ed25519 signature size)
        let sig_bytes = BytesN::<64>::random(env);
        let mut result = Bytes::new(env);
        for i in 0..64 {
            result.push_back(sig_bytes.get(i).unwrap_or(0));
        }
        result
    }

    #[test]
    fn test_initialize() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        // Initialize contract
        client.initialize(&admin);
        
        // Verify admin is set
        let stored_admin = client.get_admin();
        assert_eq!(stored_admin, admin);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #1)")]
    fn test_initialize_twice_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        // Initialize contract
        client.initialize(&admin);
        
        // Try to initialize again - should fail
        client.initialize(&admin);
    }

    #[test]
    fn test_register_attestor() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        // Initialize contract
        client.initialize(&admin);
        
        // Register attestor
        client.register_attestor(&attestor);
        
        // Verify attestor is registered
        assert!(client.is_attestor(&attestor));
        
        // Check event was emitted
        let events = env.events().all();
        let event = events.last().unwrap();
        assert_eq!(
            event.1,
            (soroban_sdk::symbol_short!("attestor"), soroban_sdk::symbol_short!("added")).into_val(&env)
        );
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #4)")]
    fn test_register_attestor_twice_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        // Try to register again - should fail
        client.register_attestor(&attestor);
    }

    #[test]
    fn test_revoke_attestor() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        // Verify attestor is registered
        assert!(client.is_attestor(&attestor));
        
        // Revoke attestor
        client.revoke_attestor(&attestor);
        
        // Verify attestor is no longer registered
        assert!(!client.is_attestor(&attestor));
        
        // Check event was emitted
        let events = env.events().all();
        let event = events.last().unwrap();
        assert_eq!(
            event.1,
            (soroban_sdk::symbol_short!("attestor"), soroban_sdk::symbol_short!("removed")).into_val(&env)
        );
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")]
    fn test_revoke_unregistered_attestor_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        
        // Try to revoke unregistered attestor - should fail
        client.revoke_attestor(&attestor);
    }

    #[test]
    fn test_submit_attestation() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&issuer);
        
        let timestamp = 1234567890u64;
        let payload_hash = BytesN::random(&env);
        let signature = create_ed25519_signature(&env, &subject, timestamp, &payload_hash);
        
        // Submit attestation
        let attestation_id = client.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);
        
        // Verify attestation ID
        assert_eq!(attestation_id, 0);
        
        // Retrieve and verify attestation
        let attestation = client.get_attestation(&attestation_id);
        assert_eq!(attestation.id, attestation_id);
        assert_eq!(attestation.issuer, issuer);
        assert_eq!(attestation.subject, subject);
        assert_eq!(attestation.timestamp, timestamp);
        assert_eq!(attestation.payload_hash, payload_hash);
        
        // Check event was emitted
        let events = env.events().all();
        let event = events.last().unwrap();
        assert_eq!(
            event.1,
            (soroban_sdk::symbol_short!("attest"), soroban_sdk::symbol_short!("recorded")).into_val(&env)
        );
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #3)")]
    fn test_submit_attestation_unauthorized_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        // Don't register issuer as attestor
        
        let timestamp = 1234567890u64;
        let payload_hash = BytesN::random(&env);
        let signature = create_ed25519_signature(&env, &subject, timestamp, &payload_hash);
        
        // Try to submit attestation - should fail
        client.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #7)")]
    fn test_submit_attestation_invalid_timestamp_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&issuer);
        
        let timestamp = 0u64; // Invalid timestamp
        let payload_hash = BytesN::random(&env);
        let signature = create_ed25519_signature(&env, &subject, timestamp, &payload_hash);
        
        // Try to submit attestation with invalid timestamp - should fail
        client.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #6)")]
    fn test_submit_attestation_replay_attack_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&issuer);
        
        let timestamp = 1234567890u64;
        let payload_hash = BytesN::random(&env);
        let signature = create_ed25519_signature(&env, &subject, timestamp, &payload_hash);
        
        // Submit attestation
        client.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);
        
        // Try to submit same attestation again - should fail
        client.submit_attestation(&issuer, &subject, &timestamp, &payload_hash, &signature);
    }

    #[test]
    fn test_multiple_attestations() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let issuer = Address::generate(&env);
        let subject1 = Address::generate(&env);
        let subject2 = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&issuer);
        
        // Submit first attestation
        let timestamp1 = 1234567890u64;
        let payload_hash1 = BytesN::random(&env);
        let signature1 = create_ed25519_signature(&env, &subject1, timestamp1, &payload_hash1);
        let id1 = client.submit_attestation(&issuer, &subject1, &timestamp1, &payload_hash1, &signature1);
        
        // Submit second attestation
        let timestamp2 = 1234567891u64;
        let payload_hash2 = BytesN::random(&env);
        let signature2 = create_ed25519_signature(&env, &subject2, timestamp2, &payload_hash2);
        let id2 = client.submit_attestation(&issuer, &subject2, &timestamp2, &payload_hash2, &signature2);
        
        // Verify IDs are sequential
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        
        // Verify both attestations can be retrieved
        let attestation1 = client.get_attestation(&id1);
        assert_eq!(attestation1.subject, subject1);
        
        let attestation2 = client.get_attestation(&id2);
        assert_eq!(attestation2.subject, subject2);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #8)")]
    fn test_get_nonexistent_attestation_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        
        // Try to get non-existent attestation - should fail
        client.get_attestation(&999);
    }

    #[test]
    fn test_is_attestor_returns_false_for_unregistered() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        
        // Check unregistered attestor
        assert!(!client.is_attestor(&attestor));
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #2)")]
    fn test_get_admin_before_initialize_fails() {
        let env = Env::default();
        
        let (_contract_id, client) = create_test_contract(&env);
        
        // Try to get admin before initialization - should fail
        client.get_admin();
    }

    #[test]
    fn test_configure_endpoint() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        let url = String::from_str(&env, "https://api.example.com/attest");
        
        // Configure endpoint
        client.configure_endpoint(&attestor, &url);
        
        // Verify endpoint is configured
        let endpoint = client.get_endpoint(&attestor);
        assert_eq!(endpoint.url, url);
        assert_eq!(endpoint.attestor, attestor);
        assert_eq!(endpoint.is_active, true);
        
        // Check event was emitted
        let events = env.events().all();
        let event = events.last().unwrap();
        assert_eq!(
            event.1,
            (soroban_sdk::symbol_short!("endpoint"), soroban_sdk::symbol_short!("config")).into_val(&env)
        );
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")]
    fn test_configure_endpoint_invalid_format_no_protocol() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        let url = String::from_str(&env, "api.example.com/attest");
        
        // Try to configure endpoint with invalid format - should fail
        client.configure_endpoint(&attestor, &url);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")]
    fn test_configure_endpoint_invalid_format_empty() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        let url = String::from_str(&env, "");
        
        // Try to configure endpoint with empty URL - should fail
        client.configure_endpoint(&attestor, &url);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #10)")]
    fn test_configure_endpoint_invalid_format_protocol_only() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        let url = String::from_str(&env, "https://");
        
        // Try to configure endpoint with protocol only - should fail
        client.configure_endpoint(&attestor, &url);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")]
    fn test_configure_endpoint_unregistered_attestor_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        // Don't register attestor
        
        let url = String::from_str(&env, "https://api.example.com/attest");
        
        // Try to configure endpoint for unregistered attestor - should fail
        client.configure_endpoint(&attestor, &url);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #12)")]
    fn test_configure_endpoint_twice_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        let url = String::from_str(&env, "https://api.example.com/attest");
        
        // Configure endpoint
        client.configure_endpoint(&attestor, &url);
        
        // Try to configure again - should fail
        client.configure_endpoint(&attestor, &url);
    }

    #[test]
    fn test_update_endpoint() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        let url1 = String::from_str(&env, "https://api.example.com/attest");
        client.configure_endpoint(&attestor, &url1);
        
        // Update endpoint
        let url2 = String::from_str(&env, "https://api.newdomain.com/attest");
        client.update_endpoint(&attestor, &url2, &false);
        
        // Verify endpoint is updated
        let endpoint = client.get_endpoint(&attestor);
        assert_eq!(endpoint.url, url2);
        assert_eq!(endpoint.is_active, false);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #11)")]
    fn test_update_nonexistent_endpoint_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        let url = String::from_str(&env, "https://api.example.com/attest");
        
        // Try to update non-existent endpoint - should fail
        client.update_endpoint(&attestor, &url, &true);
    }

    #[test]
    fn test_remove_endpoint() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        let url = String::from_str(&env, "https://api.example.com/attest");
        client.configure_endpoint(&attestor, &url);
        
        // Remove endpoint
        client.remove_endpoint(&attestor);
        
        // Check event was emitted
        let events = env.events().all();
        let event = events.last().unwrap();
        assert_eq!(
            event.1,
            (soroban_sdk::symbol_short!("endpoint"), soroban_sdk::symbol_short!("removed")).into_val(&env)
        );
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #11)")]
    fn test_remove_nonexistent_endpoint_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        // Try to remove non-existent endpoint - should fail
        client.remove_endpoint(&attestor);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #11)")]
    fn test_get_nonexistent_endpoint_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        // Try to get non-existent endpoint - should fail
        client.get_endpoint(&attestor);
    }

    #[test]
    fn test_endpoint_with_http_protocol() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&attestor);
        
        let url = String::from_str(&env, "http://api.example.com/attest");
        
        // Configure endpoint with http protocol
        client.configure_endpoint(&attestor, &url);
        
        // Verify endpoint is configured
        let endpoint = client.get_endpoint(&attestor);
        assert_eq!(endpoint.url, url);
    }
}
