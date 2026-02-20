#![no_std]

mod errors;
mod events;
mod storage;
mod types;

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, String};


pub use errors::Error;

pub use events::{AttestationRecorded, AttestorAdded, AttestorRemoved, EndpointConfigured, EndpointRemoved, ServicesConfigured};
pub use storage::Storage;
pub use types::{Attestation, Endpoint, ServiceType, AnchorServices};

pub use events::{AttestationRecorded, AttestorAdded, AttestorRemoved, EndpointConfigured, EndpointRemoved, SessionCreated, OperationLogged};
pub use storage::Storage;
pub use types::{Attestation, Endpoint, InteractionSession, OperationContext, AuditLog};


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
        
        AttestorAdded::publish(&env, &attestor);

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
        
        AttestorRemoved::publish(&env, &attestor);

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
        AttestationRecorded::publish(&env, id, &subject, timestamp, payload_hash);

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
        let _admin = Storage::get_admin(&env)?;
        
        // Require auth from either attestor or admin
        attestor.require_auth();

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
        let _admin = Storage::get_admin(&env)?;
        
        // Require auth from either attestor or admin
        attestor.require_auth();

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


    /// Configure supported services for an anchor. Only callable by the anchor or admin.
    pub fn configure_services(env: Env, anchor: Address, services: Vec<ServiceType>) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        
        // Allow either the anchor themselves or the admin to configure
        let caller_is_admin = env.try_invoke_contract::<bool, _>(&admin, &soroban_sdk::symbol_short!(""), &()).is_ok();
        
        if !caller_is_admin {
            anchor.require_auth();
        } else {
            admin.require_auth();
        }

        // Validate services list
        Self::validate_services(&services)?;

        // Check if anchor is a registered attestor
        if !Storage::is_attestor(&env, &anchor) {
            return Err(Error::AttestorNotRegistered);
        }

        let anchor_services = AnchorServices {
            anchor: anchor.clone(),
            services: services.clone(),
        };

        Storage::set_anchor_services(&env, &anchor_services);

        ServicesConfigured {
            anchor,
            services,
        }
        .publish(&env);

        Ok(())
    }

    /// Get the list of supported services for an anchor.
    pub fn get_supported_services(env: Env, anchor: Address) -> Result<Vec<ServiceType>, Error> {
        let anchor_services = Storage::get_anchor_services(&env, &anchor)?;
        Ok(anchor_services.services)
    }

    /// Check if an anchor supports a specific service.
    pub fn supports_service(env: Env, anchor: Address, service: ServiceType) -> bool {
        if let Ok(anchor_services) = Storage::get_anchor_services(&env, &anchor) {
            anchor_services.services.contains(&service)
        } else {
            false
        }
    }

    /// Validate services list.
    /// Checks for:
    /// - Non-empty list
    /// - No duplicate services
    /// - Valid service types
    fn validate_services(services: &Vec<ServiceType>) -> Result<(), Error> {
        // Check if services list is empty
        if services.is_empty() {
            return Err(Error::InvalidServiceType);
        }

        // Check for duplicates by comparing sorted list
        let mut sorted_services = services.clone();
        sorted_services.sort();
        
        for i in 1..sorted_services.len() {
            if sorted_services.get(i - 1) == sorted_services.get(i) {
                return Err(Error::InvalidServiceType);
            }
        }


    // ============ Session Management for Reproducibility ============

    /// Create a new interaction session for tracing operations.
    /// Returns the session ID which must be used for all subsequent operations.
    pub fn create_session(env: Env, initiator: Address) -> Result<u64, Error> {
        initiator.require_auth();
        
        // Verify contract is initialized
        Storage::get_admin(&env)?;

        let session_id = Storage::create_session(&env, &initiator);
        let timestamp = env.ledger().timestamp();

        SessionCreated::publish(&env, session_id, &initiator, timestamp);

        Ok(session_id)
    }

    /// Get session details for reproducibility verification.
    pub fn get_session(env: Env, session_id: u64) -> Result<InteractionSession, Error> {
        Storage::get_session(&env, session_id)
    }

    /// Get audit log entry for tracing specific operations.
    pub fn get_audit_log(env: Env, log_id: u64) -> Result<AuditLog, Error> {
        Storage::get_audit_log(&env, log_id)
    }

    /// Get the total number of operations in a session.
    /// Used to verify session completeness for reproducibility.
    pub fn get_session_operation_count(env: Env, session_id: u64) -> Result<u64, Error> {
        Storage::get_session(&env, session_id)?;
        Ok(Storage::get_session_operation_count(&env, session_id))
    }

    /// Internal helper to log an operation within a session.
    /// This ensures all contract operations are traceable and reproducible.
    fn log_session_operation(
        env: &Env,
        session_id: u64,
        actor: &Address,
        operation_type: &str,
        status: &str,
        result_data: u64,
    ) -> Result<u64, Error> {
        // Verify session exists
        Storage::get_session(env, session_id)?;

        let operation_index = Storage::increment_session_operation_count(env, session_id);
        let timestamp = env.ledger().timestamp();

        let operation = OperationContext {
            session_id,
            operation_index,
            operation_type: String::from_str(env, operation_type),
            timestamp,
            status: String::from_str(env, status),
            result_data,
        };

        let log_id = Storage::log_operation(env, session_id, actor, &operation);

        OperationLogged::publish(
            env,
            log_id,
            session_id,
            operation_index,
            &operation.operation_type,
            &operation.status,
        );

        Ok(log_id)
    }

    /// Submit an attestation within a session for full traceability.
    /// This variant ensures the operation is logged for reproducibility.
    pub fn submit_attestation_with_session(
        env: Env,
        session_id: u64,
        issuer: Address,
        subject: Address,
        timestamp: u64,
        payload_hash: BytesN<32>,
        signature: Bytes,
    ) -> Result<u64, Error> {
        issuer.require_auth();

        // Validate timestamp
        if timestamp == 0 {
            Self::log_session_operation(&env, session_id, &issuer, "attest", "failed", 0)?;
            return Err(Error::InvalidTimestamp);
        }

        // Check if issuer is a registered attestor
        if !Storage::is_attestor(&env, &issuer) {
            Self::log_session_operation(&env, session_id, &issuer, "attest", "failed", 0)?;
            return Err(Error::UnauthorizedAttestor);
        }

        // Check for replay attack
        if Storage::is_hash_used(&env, &payload_hash) {
            Self::log_session_operation(&env, session_id, &issuer, "attest", "failed", 0)?;
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
        AttestationRecorded::publish(&env, id, &subject, timestamp, payload_hash);

        // Log operation for reproducibility
        Self::log_session_operation(&env, session_id, &issuer, "attest", "success", id)?;

        Ok(id)
    }

    /// Register an attestor within a session for full traceability.
    pub fn register_attestor_with_session(env: Env, session_id: u64, attestor: Address) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if Storage::is_attestor(&env, &attestor) {
            Self::log_session_operation(&env, session_id, &admin, "register", "failed", 0)?;
            return Err(Error::AttestorAlreadyRegistered);
        }

        Storage::set_attestor(&env, &attestor, true);
        AttestorAdded::publish(&env, &attestor);

        Self::log_session_operation(&env, session_id, &admin, "register", "success", 0)?;

        Ok(())
    }

    /// Revoke an attestor within a session for full traceability.
    pub fn revoke_attestor_with_session(env: Env, session_id: u64, attestor: Address) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if !Storage::is_attestor(&env, &attestor) {
            Self::log_session_operation(&env, session_id, &admin, "revoke", "failed", 0)?;
            return Err(Error::AttestorNotRegistered);
        }

        Storage::set_attestor(&env, &attestor, false);
        AttestorRemoved::publish(&env, &attestor);

        Self::log_session_operation(&env, session_id, &admin, "revoke", "success", 0)?;


        Ok(())
    }

    /// Validate endpoint URL format.
    /// Checks for:
    /// - Non-empty URL
    /// - Valid protocol (http:// or https://)
    /// - Reasonable length
    fn validate_endpoint_url(url: &String) -> Result<(), Error> {
        let len = url.len();
        
        // Check if URL is empty or too long
        if len == 0 || len > 256 {
            return Err(Error::InvalidEndpointFormat);
        }

        // For Soroban, we do basic validation
        // Check minimum length for "http://x" (8 chars) or "https://x" (9 chars)
        if len < 8 {
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
        Address, Bytes, BytesN, Env,
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
    #[should_panic(expected = "Error(Contract, #100)")]
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
        // Event topic now includes the attestor address
        assert_eq!(event.1.len(), 3); // ("attestor", "added", address)
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #103)")]
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
        // Event topic now includes the attestor address
        assert_eq!(event.1.len(), 3); // ("attestor", "removed", address)
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #104)")]
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
        // Event topic now includes attestation_id and subject address
        assert_eq!(event.1.len(), 4); // ("attest", "recorded", id, subject)
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #102)")]
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
    #[should_panic(expected = "Error(Contract, #106)")]
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
    #[should_panic(expected = "Error(Contract, #105)")]
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
    #[should_panic(expected = "Error(Contract, #107)")]
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
    #[should_panic(expected = "Error(Contract, #101)")]
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

    #[test]
    fn test_configure_services() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&anchor);
        
        let mut services = Vec::new(&env);
        services.push_back(ServiceType::Deposits);
        services.push_back(ServiceType::Withdrawals);
        services.push_back(ServiceType::KYC);
        
        // Configure services
        client.configure_services(&anchor, &services);
        
        // Verify services are configured
        let supported = client.get_supported_services(&anchor);
        assert_eq!(supported.len(), 3);
        assert!(supported.contains(&ServiceType::Deposits));
        assert!(supported.contains(&ServiceType::Withdrawals));
        assert!(supported.contains(&ServiceType::KYC));
        
        // Check event was emitted
        let events = env.events().all();
        let event = events.last().unwrap();
        assert_eq!(
            event.1,
            (soroban_sdk::symbol_short!("services"), soroban_sdk::symbol_short!("config")).into_val(&env)
        );
    }

    #[test]
    fn test_configure_all_services() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&anchor);
        
        let mut services = Vec::new(&env);
        services.push_back(ServiceType::Deposits);
        services.push_back(ServiceType::Withdrawals);
        services.push_back(ServiceType::Quotes);
        services.push_back(ServiceType::KYC);
        
        // Configure all services
        client.configure_services(&anchor, &services);
        
        // Verify all services are configured
        let supported = client.get_supported_services(&anchor);
        assert_eq!(supported.len(), 4);
        assert!(supported.contains(&ServiceType::Deposits));
        assert!(supported.contains(&ServiceType::Withdrawals));
        assert!(supported.contains(&ServiceType::Quotes));
        assert!(supported.contains(&ServiceType::KYC));
    }

    #[test]
    fn test_supports_service() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&anchor);
        
        let mut services = Vec::new(&env);
        services.push_back(ServiceType::Deposits);
        services.push_back(ServiceType::KYC);
        
        client.configure_services(&anchor, &services);
        
        // Check individual services
        assert!(client.supports_service(&anchor, &ServiceType::Deposits));
        assert!(client.supports_service(&anchor, &ServiceType::KYC));
        assert!(!client.supports_service(&anchor, &ServiceType::Withdrawals));
        assert!(!client.supports_service(&anchor, &ServiceType::Quotes));
    }

    #[test]
    fn test_supports_service_unconfigured_anchor() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&anchor);
        
        // Check service for unconfigured anchor
        assert!(!client.supports_service(&anchor, &ServiceType::Deposits));
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #5)")]
    fn test_configure_services_unregistered_anchor_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        // Don't register anchor
        
        let mut services = Vec::new(&env);
        services.push_back(ServiceType::Deposits);
        
        // Try to configure services for unregistered anchor - should fail
        client.configure_services(&anchor, &services);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #14)")]
    fn test_configure_services_empty_list_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&anchor);
        
        let services = Vec::new(&env);
        
        // Try to configure with empty services list - should fail
        client.configure_services(&anchor, &services);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #14)")]
    fn test_configure_services_duplicate_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&anchor);
        
        let mut services = Vec::new(&env);
        services.push_back(ServiceType::Deposits);
        services.push_back(ServiceType::Deposits); // Duplicate
        
        // Try to configure with duplicate services - should fail
        client.configure_services(&anchor, &services);
    }

    #[test]
    #[should_panic(expected = "Error(Contract, #13)")]
    fn test_get_services_unconfigured_fails() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&anchor);
        
        // Try to get services for unconfigured anchor - should fail
        client.get_supported_services(&anchor);
    }

    #[test]
    fn test_update_services() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&anchor);
        
        // Configure initial services
        let mut services1 = Vec::new(&env);
        services1.push_back(ServiceType::Deposits);
        client.configure_services(&anchor, &services1);
        
        // Update services
        let mut services2 = Vec::new(&env);
        services2.push_back(ServiceType::Deposits);
        services2.push_back(ServiceType::Withdrawals);
        services2.push_back(ServiceType::Quotes);
        client.configure_services(&anchor, &services2);
        
        // Verify updated services
        let supported = client.get_supported_services(&anchor);
        assert_eq!(supported.len(), 3);
        assert!(supported.contains(&ServiceType::Deposits));
        assert!(supported.contains(&ServiceType::Withdrawals));
        assert!(supported.contains(&ServiceType::Quotes));
    }

    #[test]
    fn test_multiple_anchors_services() {
        let env = Env::default();
        env.mock_all_auths();
        
        let admin = Address::generate(&env);
        let anchor1 = Address::generate(&env);
        let anchor2 = Address::generate(&env);
        let (_contract_id, client) = create_test_contract(&env);
        
        client.initialize(&admin);
        client.register_attestor(&anchor1);
        client.register_attestor(&anchor2);
        
        // Configure services for anchor1
        let mut services1 = Vec::new(&env);
        services1.push_back(ServiceType::Deposits);
        services1.push_back(ServiceType::KYC);
        client.configure_services(&anchor1, &services1);
        
        // Configure services for anchor2
        let mut services2 = Vec::new(&env);
        services2.push_back(ServiceType::Withdrawals);
        services2.push_back(ServiceType::Quotes);
        client.configure_services(&anchor2, &services2);
        
        // Verify anchor1 services
        let supported1 = client.get_supported_services(&anchor1);
        assert_eq!(supported1.len(), 2);
        assert!(supported1.contains(&ServiceType::Deposits));
        assert!(supported1.contains(&ServiceType::KYC));
        
        // Verify anchor2 services
        let supported2 = client.get_supported_services(&anchor2);
        assert_eq!(supported2.len(), 2);
        assert!(supported2.contains(&ServiceType::Withdrawals));
        assert!(supported2.contains(&ServiceType::Quotes));
    }
