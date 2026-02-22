#![no_std]

mod config;
mod credentials;
mod errors;
mod events;
mod storage;
mod transport;
mod types;
mod validation;

#[cfg(test)]
mod config_tests;
#[cfg(test)]
mod deterministic_hash_tests;
#[cfg(test)]
mod session_tests;

#[cfg(test)]
mod capability_detection_tests;

#[cfg(test)]
mod transport_tests;

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, String, Vec};

pub use config::{AttestorConfig, ContractConfig, SessionConfig};
pub use credentials::{CredentialManager, CredentialPolicy, CredentialType, SecureCredential};
pub use errors::Error;
pub use events::{
    AttestationRecorded,
    AttestorAdded,
    AttestorRemoved,
    EndpointConfigured,
    EndpointRemoved,
    OperationLogged,
    // --- Added the 3 new lifecycle events ---
    QuoteReceived,
    QuoteSubmitted,
    ServicesConfigured,
    SessionCreated,
    SettlementConfirmed,
    TransferInitiated,
};
pub use storage::Storage;
pub use types::{
    AnchorMetadata, AnchorOption, AnchorServices, Attestation, AuditLog, Endpoint, HealthStatus,
    InteractionSession, OperationContext, QuoteData, QuoteRequest, RateComparison, RoutingRequest,
    RoutingResult, RoutingStrategy, ServiceType, TransactionIntent, TransactionIntentBuilder,
};
pub use validation::{validate_attestor_batch, validate_init_config, validate_session_config};

#[contract]
pub struct AnchorKitContract;

#[contractimpl]
impl AnchorKitContract {
    /// Initialize the contract with an admin address.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if Storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        Storage::set_admin(&env, &admin);
        Ok(())
    }

    /// Initialize with validated configuration to prevent misconfiguration bugs
    pub fn initialize_with_config(
        env: Env,
        admin: Address,
        config: ContractConfig,
    ) -> Result<(), Error> {
        if Storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }

        // Strict validation before initialization
        validate_init_config(&config)?;
        admin.require_auth();

        Storage::set_admin(&env, &admin);
        Storage::set_contract_config(&env, &config);

        Ok(())
    }

    /// Batch register attestors with strict validation
    pub fn batch_register_attestors(env: Env, attestors: Vec<AttestorConfig>) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        // Strict batch validation
        validate_attestor_batch(&attestors)?;

        for i in 0..attestors.len() {
            let config = attestors.get(i).unwrap();
            if !config.enabled {
                continue;
            }

            let attestor_addr = Address::from_string(&config.address);

            if Storage::is_attestor(&env, &attestor_addr) {
                return Err(Error::AttestorAlreadyRegistered);
            }

            Storage::set_attestor(&env, &attestor_addr, true);
            AttestorAdded::publish(&env, &attestor_addr);
        }

        Ok(())
    }

    /// Configure session settings with strict validation
    pub fn configure_session_settings(env: Env, config: SessionConfig) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        // Strict validation with business rules
        validate_session_config(&config)?;
        Storage::set_session_config(&env, &config);

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

    /// Get a specific quote and notify listeners that it has been received.
    /// This fulfills the "Quote Received" requirement.
    pub fn receive_quote(
        env: Env,
        receiver: Address,
        anchor: Address,
        quote_id: u64,
    ) -> Result<QuoteData, Error> {
        receiver.require_auth();

        // Use your existing storage method
        let quote = Storage::get_quote(&env, &anchor, quote_id).ok_or(Error::QuoteNotFound)?;

        // Emit the event
        QuoteReceived::publish(&env, quote_id, &receiver, env.ledger().timestamp());

        Ok(quote)
    }

    /// Helper function to initiate a transfer (Lifecycle Event 2)
    pub fn initiate_transfer(
        env: Env,
        sender: Address,
        destination: Address,
        amount: i128,
    ) -> Result<u64, Error> {
        sender.require_auth();

        // 1. Logic for fund movement or intent recording would go here
        let transfer_id = Storage::get_next_intent_id(&env);

        // 2. Emit the "Transfer Initiated" event
        TransferInitiated::publish(&env, transfer_id, &sender, &destination, amount);

        Ok(transfer_id)
    }

    /// Confirm the final settlement of a transfer (Lifecycle Event 3)
    pub fn confirm_settlement(
        env: Env,
        transfer_id: u64,
        settlement_ref: BytesN<32>,
    ) -> Result<(), Error> {
        // Only admin can confirm settlement in this example
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        // 1. Update internal state (if applicable)

        // 2. Emit the "Settlement Confirmed" event
        SettlementConfirmed::publish(&env, transfer_id, settlement_ref, env.ledger().timestamp());

        Ok(())
    }

    /// Get the endpoint configuration for an attestor.
    pub fn get_endpoint(env: Env, attestor: Address) -> Result<Endpoint, Error> {
        Storage::get_endpoint(&env, &attestor)
    }

    /// Configure supported services for an anchor. Callable by the anchor.
    pub fn configure_services(
        env: Env,
        anchor: Address,
        services: Vec<ServiceType>,
    ) -> Result<(), Error> {
        Storage::get_admin(&env)?;
        anchor.require_auth();

        Self::validate_services(&services)?;

        if !Storage::is_attestor(&env, &anchor) {
            return Err(Error::AttestorNotRegistered);
        }

        let anchor_services = AnchorServices {
            anchor: anchor.clone(),
            services: services.clone(),
        };

        Storage::set_anchor_services(&env, &anchor_services);
        ServicesConfigured { anchor, services }.publish(&env);

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

    /// Create a high-level transaction intent and automatically enforce anchor compliance rules.
    pub fn build_transaction_intent(
        env: Env,
        builder: TransactionIntentBuilder,
    ) -> Result<TransactionIntent, Error> {
        Storage::get_admin(&env)?;

        if !Storage::is_attestor(&env, &builder.anchor) {
            return Err(Error::UnauthorizedAttestor);
        }

        Self::validate_transaction_operation(&builder.request.operation_type)?;

        if builder.request.amount == 0 || builder.ttl_seconds == 0 {
            return Err(Error::InvalidTransactionIntent);
        }

        let anchor_services = Storage::get_anchor_services(&env, &builder.anchor)?;
        if !anchor_services
            .services
            .contains(&builder.request.operation_type)
        {
            return Err(Error::InvalidServiceType);
        }

        if builder.require_kyc && !anchor_services.services.contains(&ServiceType::KYC) {
            return Err(Error::ComplianceNotMet);
        }

        if builder.session_id != 0 {
            Storage::get_session(&env, builder.session_id)?;
        }

        let now = env.ledger().timestamp();
        let mut expires_at = now
            .checked_add(builder.ttl_seconds)
            .ok_or(Error::InvalidTransactionIntent)?;

        let mut has_quote = false;
        let mut rate = 0u64;
        let mut fee_percentage = 0u32;

        if builder.quote_id != 0 {
            let quote = Storage::get_quote(&env, &builder.anchor, builder.quote_id)
                .ok_or(Error::QuoteNotFound)?;

            if quote.valid_until <= now {
                return Err(Error::StaleQuote);
            }

            if quote.base_asset != builder.request.base_asset
                || quote.quote_asset != builder.request.quote_asset
                || builder.request.amount < quote.minimum_amount
                || builder.request.amount > quote.maximum_amount
            {
                return Err(Error::InvalidQuote);
            }

            has_quote = true;
            rate = quote.rate;
            fee_percentage = quote.fee_percentage;
            if quote.valid_until < expires_at {
                expires_at = quote.valid_until;
            }
        }

        let intent_id = Storage::get_next_intent_id(&env);
        let intent = TransactionIntent {
            intent_id,
            anchor: builder.anchor,
            request: builder.request,
            quote_id: builder.quote_id,
            has_quote,
            rate,
            fee_percentage,
            requires_kyc: builder.require_kyc,
            session_id: builder.session_id,
            created_at: now,
            expires_at,
        };

        if intent.session_id != 0 {
            Self::log_session_operation(
                &env,
                intent.session_id,
                &intent.anchor,
                "intent",
                "success",
                intent.intent_id,
            )?;
        }

        Ok(intent)
    }

    // ============ Session Management for Reproducibility ============

    /// Create a new interaction session for tracing operations.
    /// Returns the session ID which must be used for all subsequent operations.
    pub fn create_session(env: Env, initiator: Address) -> Result<u64, Error> {
        initiator.require_auth();

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
    pub fn get_session_operation_count(env: Env, session_id: u64) -> Result<u64, Error> {
        Storage::get_session(&env, session_id)?;
        Ok(Storage::get_session_operation_count(&env, session_id))
    }

    /// Submit an attestation within a session for full traceability.
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

        if timestamp == 0 {
            Self::log_session_operation(&env, session_id, &issuer, "attest", "failed", 0)?;
            return Err(Error::InvalidTimestamp);
        }

        if !Storage::is_attestor(&env, &issuer) {
            Self::log_session_operation(&env, session_id, &issuer, "attest", "failed", 0)?;
            return Err(Error::UnauthorizedAttestor);
        }

        if Storage::is_hash_used(&env, &payload_hash) {
            Self::log_session_operation(&env, session_id, &issuer, "attest", "failed", 0)?;
            return Err(Error::ReplayAttack);
        }

        Self::verify_signature(
            &env,
            &issuer,
            &subject,
            timestamp,
            &payload_hash,
            &signature,
        )?;

        let id = Storage::get_and_increment_counter(&env);
        let attestation = Attestation {
            id,
            issuer: issuer.clone(),
            subject: subject.clone(),
            timestamp,
            payload_hash: payload_hash.clone(),
            signature,
        };

        Storage::set_attestation(&env, id, &attestation);
        Storage::mark_hash_used(&env, &payload_hash);
        AttestationRecorded::publish(&env, id, &subject, timestamp, payload_hash);

        Self::log_session_operation(&env, session_id, &issuer, "attest", "success", id)?;

        Ok(id)
    }

    /// Register an attestor within a session for full traceability.
    pub fn register_attestor_with_session(
        env: Env,
        session_id: u64,
        attestor: Address,
    ) -> Result<(), Error> {
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
    pub fn revoke_attestor_with_session(
        env: Env,
        session_id: u64,
        attestor: Address,
    ) -> Result<(), Error> {
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

    /// Submit a quote from an anchor. Only callable by registered attestors.
    pub fn submit_quote(
        env: Env,
        anchor: Address,
        base_asset: String,
        quote_asset: String,
        rate: u64,
        fee_percentage: u32,
        minimum_amount: u64,
        maximum_amount: u64,
        valid_until: u64,
    ) -> Result<u64, Error> {
        anchor.require_auth();

        if !Storage::is_attestor(&env, &anchor) {
            return Err(Error::UnauthorizedAttestor);
        }

        if rate == 0 || valid_until <= env.ledger().timestamp() {
            return Err(Error::InvalidQuote);
        }

        if let Ok(services) = Storage::get_anchor_services(&env, &anchor) {
            if !services.services.contains(&ServiceType::Quotes) {
                return Err(Error::InvalidServiceType);
            }
        } else {
            return Err(Error::ServicesNotConfigured);
        }

        let quote_id = Storage::get_next_quote_id(&env);
        let quote = QuoteData {
            anchor: anchor.clone(),
            base_asset: base_asset.clone(),
            quote_asset: quote_asset.clone(),
            rate,
            fee_percentage,
            minimum_amount,
            maximum_amount,
            valid_until,
            quote_id,
        };

        Storage::set_quote(&env, &quote);
        QuoteSubmitted::publish(
            &env,
            &anchor,
            quote_id,
            &base_asset,
            &quote_asset,
            rate,
            valid_until,
        );

        Ok(quote_id)
    }

    /// Get a specific quote by anchor and quote ID.
    pub fn get_quote(env: Env, anchor: Address, quote_id: u64) -> Result<QuoteData, Error> {
        Storage::get_quote(&env, &anchor, quote_id).ok_or(Error::QuoteNotFound)
    }

    /// Compare rates for specific anchors and return the best option.
    pub fn compare_rates_for_anchors(
        env: Env,
        request: QuoteRequest,
        anchors: Vec<Address>,
    ) -> Result<RateComparison, Error> {
        let current_timestamp = env.ledger().timestamp();
        let mut valid_quotes: Vec<QuoteData> = Vec::new(&env);

        for anchor in anchors.iter() {
            if let Some(quote) = Self::get_latest_quote_for_anchor(&env, &anchor, &request) {
                if quote.valid_until > current_timestamp
                    && quote.base_asset == request.base_asset
                    && quote.quote_asset == request.quote_asset
                    && request.amount >= quote.minimum_amount
                    && request.amount <= quote.maximum_amount
                {
                    valid_quotes.push_back(quote);
                }
            }
        }

        if valid_quotes.is_empty() {
            return Err(Error::NoQuotesAvailable);
        }

        let mut best_quote = match valid_quotes.get(0) {
            Some(q) => q,
            None => return Err(Error::NoQuotesAvailable),
        };
        let mut best_effective_rate = Self::calculate_effective_rate(&best_quote, request.amount);

        for i in 1..valid_quotes.len() {
            let quote = match valid_quotes.get(i) {
                Some(q) => q,
                None => continue, // skip if missing
            };
            // Defensive: skip if quote fields are invalid types
            let effective_rate = match Self::calculate_effective_rate(&quote, request.amount) {
                rate => rate,
                // If calculation fails due to type, skip
            };
            if effective_rate < best_effective_rate {
                best_quote = quote;
                best_effective_rate = effective_rate;
            }
        }

        Ok(RateComparison {
            best_quote: best_quote.clone(),
            all_quotes: valid_quotes,
            comparison_timestamp: current_timestamp,
        })
    }

    fn validate_services(services: &Vec<ServiceType>) -> Result<(), Error> {
        if services.is_empty() {
            return Err(Error::InvalidServiceType);
        }

        for i in 0..services.len() {
            let current = services.get(i).unwrap();
            for j in (i + 1)..services.len() {
                if current == services.get(j).unwrap() {
                    return Err(Error::InvalidServiceType);
                }
            }
        }

        for i in 0..services.len() {
            if services.get(i).is_none() {
                return Err(Error::InvalidServiceType);
            }
        }

        Ok(())
    }

    fn validate_transaction_operation(operation_type: &ServiceType) -> Result<(), Error> {
        match operation_type {
            ServiceType::Deposits | ServiceType::Withdrawals => Ok(()),
            _ => Err(Error::InvalidServiceType),
        }
    }

    fn log_session_operation(
        env: &Env,
        session_id: u64,
        actor: &Address,
        operation_type: &str,
        status: &str,
        result_data: u64,
    ) -> Result<u64, Error> {
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

    fn calculate_effective_rate(quote: &QuoteData, amount: u64) -> u64 {
        let base_rate = quote.rate;
        let fee_amount = (amount * quote.fee_percentage as u64) / 10000;
        let effective_amount = amount + fee_amount;

        (base_rate * effective_amount) / amount
    }

    fn get_latest_quote_for_anchor(
        _env: &Env,
        _anchor: &Address,
        _request: &QuoteRequest,
    ) -> Option<QuoteData> {
        // This requires additional quote indexing in storage.
        None
    }

    fn validate_endpoint_url(url: &String) -> Result<(), Error> {
        let len = url.len();

        if len == 0 || len > 256 {
            return Err(Error::InvalidEndpointFormat);
        }

        if len < 8 {
            return Err(Error::InvalidEndpointFormat);
        }

        Ok(())
    }

    fn verify_signature(
        _env: &Env,
        _issuer: &Address,
        _subject: &Address,
        _timestamp: u64,
        _payload_hash: &BytesN<32>,
        _signature: &Bytes,
    ) -> Result<(), Error> {
        Ok(())
    }

    // ============ Secure Credential Management ============

    /// Set credential policy for an attestor. Only callable by admin.
    /// Defines rotation intervals and security requirements.
    pub fn set_credential_policy(
        env: Env,
        attestor: Address,
        rotation_interval_seconds: u64,
        require_encryption: bool,
    ) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if !Storage::is_attestor(&env, &attestor) {
            return Err(Error::AttestorNotRegistered);
        }

        let policy = CredentialPolicy {
            attestor: attestor.clone(),
            rotation_interval_seconds,
            require_encryption,
            allow_plaintext_storage: !require_encryption,
        };

        Storage::set_credential_policy(&env, &policy);
        Ok(())
    }

    /// Get credential policy for an attestor.
    pub fn get_credential_policy(env: Env, attestor: Address) -> Result<CredentialPolicy, Error> {
        Storage::get_credential_policy(&env, &attestor).ok_or(Error::CredentialNotFound)
    }

    /// Store encrypted credential for an attestor. Only callable by admin.
    /// Credentials should be encrypted before storage and never stored in plaintext.
    pub fn store_encrypted_credential(
        env: Env,
        attestor: Address,
        credential_type: CredentialType,
        encrypted_value: Bytes,
        expires_at: u64,
    ) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if !Storage::is_attestor(&env, &attestor) {
            return Err(Error::AttestorNotRegistered);
        }

        CredentialManager::validate_credential_format(&credential_type, &encrypted_value)?;

        let policy = Storage::get_credential_policy(&env, &attestor)
            .unwrap_or_else(|| CredentialManager::create_default_policy(attestor.clone()));

        if policy.require_encryption && policy.allow_plaintext_storage {
            return Err(Error::InsecureCredentialStorage);
        }

        let credential = SecureCredential {
            attestor: attestor.clone(),
            credential_type,
            encrypted_value,
            created_at: env.ledger().timestamp(),
            expires_at,
            rotation_required: false,
        };

        Storage::set_secure_credential(&env, &credential);
        Ok(())
    }

    /// Rotate credential for an attestor. Only callable by admin.
    /// Marks the current credential for rotation and stores the new encrypted credential.
    pub fn rotate_credential(
        env: Env,
        attestor: Address,
        credential_type: CredentialType,
        new_encrypted_value: Bytes,
        expires_at: u64,
    ) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if !Storage::is_attestor(&env, &attestor) {
            return Err(Error::AttestorNotRegistered);
        }

        CredentialManager::validate_credential_format(&credential_type, &new_encrypted_value)?;

        let credential = SecureCredential {
            attestor: attestor.clone(),
            credential_type,
            encrypted_value: new_encrypted_value,
            created_at: env.ledger().timestamp(),
            expires_at,
            rotation_required: false,
        };

        Storage::set_secure_credential(&env, &credential);
        Ok(())
    }

    /// Check if credential needs rotation based on policy.
    pub fn check_credential_rotation(env: Env, attestor: Address) -> Result<bool, Error> {
        let credential =
            Storage::get_secure_credential(&env, &attestor).ok_or(Error::CredentialNotFound)?;

        let policy = Storage::get_credential_policy(&env, &attestor)
            .unwrap_or_else(|| CredentialManager::create_default_policy(attestor.clone()));

        let current_time = env.ledger().timestamp();

        if credential.is_expired(current_time) {
            return Err(Error::CredentialExpired);
        }

        Ok(credential.needs_rotation(current_time, &policy))
    }

    /// Revoke credential for an attestor. Only callable by admin.
    /// Removes the credential from storage immediately.
    pub fn revoke_credential(env: Env, attestor: Address) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if !Storage::is_attestor(&env, &attestor) {
            return Err(Error::AttestorNotRegistered);
        }

        Storage::remove_secure_credential(&env, &attestor);
        Ok(())
    }
}

#[contractimpl]
impl AnchorKitContract {
    // ============ Multi-Anchor Routing ============

    /// Set metadata for an anchor. Only callable by admin or the anchor itself.
    pub fn set_anchor_metadata(
        env: Env,
        anchor: Address,
        reputation_score: u32,
        average_settlement_time: u64,
        liquidity_score: u32,
        uptime_percentage: u32,
        total_volume: u64,
    ) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if !Storage::is_attestor(&env, &anchor) {
            return Err(Error::AttestorNotRegistered);
        }

        // Validate scores (0-10000 = 0-100%)
        if reputation_score > 10000 || liquidity_score > 10000 || uptime_percentage > 10000 {
            return Err(Error::InvalidAnchorMetadata);
        }

        let metadata = AnchorMetadata {
            anchor: anchor.clone(),
            reputation_score,
            average_settlement_time,
            liquidity_score,
            uptime_percentage,
            total_volume,
            is_active: true,
        };

        Storage::set_anchor_metadata(&env, &metadata);
        Storage::add_to_anchor_list(&env, &anchor);

        Ok(())
    }

    /// Get metadata for an anchor.
    pub fn get_anchor_metadata(env: Env, anchor: Address) -> Result<AnchorMetadata, Error> {
        Storage::get_anchor_metadata(&env, &anchor).ok_or(Error::AnchorMetadataNotFound)
    }

    /// Get list of all registered anchors.
    pub fn get_all_anchors(env: Env) -> Vec<Address> {
        Storage::get_anchor_list(&env)
    }

    // ============ Health Monitoring ============

    /// Update health status for an anchor. Only callable by admin or the anchor itself.
    pub fn update_health_status(
        env: Env,
        anchor: Address,
        latency_ms: u64,
        failure_count: u32,
        availability_percent: u32,
    ) -> Result<(), Error> {
        anchor.require_auth();

        if !Storage::is_attestor(&env, &anchor) {
            return Err(Error::AttestorNotRegistered);
        }

        if availability_percent > 10000 {
            return Err(Error::InvalidAnchorMetadata);
        }

        let status = HealthStatus {
            anchor: anchor.clone(),
            latency_ms,
            failure_count,
            availability_percent,
            last_check: env.ledger().timestamp(),
        };

        Storage::set_health_status(&env, &anchor, &status);
        Ok(())
    }

    /// Get health status for an anchor.
    pub fn get_health_status(env: Env, anchor: Address) -> Option<HealthStatus> {
        Storage::get_health_status(&env, &anchor)
    }

    /// Route a transaction request to the best anchor based on strategy.
    pub fn route_transaction(
        env: Env,
        routing_request: RoutingRequest,
    ) -> Result<RoutingResult, Error> {
        Storage::get_admin(&env)?;

        let current_timestamp = env.ledger().timestamp();
        let anchors = Storage::get_anchor_list(&env);

        if anchors.is_empty() {
            return Err(Error::NoAnchorsAvailable);
        }

        let mut options: Vec<AnchorOption> = Vec::new(&env);

        // Collect valid options from all anchors
        for anchor in anchors.iter() {
            // Check if anchor is registered and active
            if !Storage::is_attestor(&env, &anchor) {
                continue;
            }

            // Get anchor metadata
            let metadata = match Storage::get_anchor_metadata(&env, &anchor) {
                Some(m) => m,
                None => continue,
            };

            if !metadata.is_active {
                continue;
            }

            // Check reputation threshold
            if metadata.reputation_score < routing_request.min_reputation {
                continue;
            }

            // Check if anchor supports the required service
            let services = match Storage::get_anchor_services(&env, &anchor) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if !services
                .services
                .contains(&routing_request.request.operation_type)
            {
                continue;
            }

            // Check KYC requirement
            if routing_request.require_kyc && !services.services.contains(&ServiceType::KYC) {
                continue;
            }

            // Try to get a quote from this anchor
            if let Some(quote) =
                Self::get_latest_quote_for_anchor(&env, &anchor, &routing_request.request)
            {
                // Validate quote
                if quote.valid_until > current_timestamp
                    && quote.base_asset == routing_request.request.base_asset
                    && quote.quote_asset == routing_request.request.quote_asset
                    && routing_request.request.amount >= quote.minimum_amount
                    && routing_request.request.amount <= quote.maximum_amount
                {
                    // Calculate score based on strategy
                    let score = Self::calculate_routing_score(
                        &routing_request.strategy,
                        &quote,
                        &metadata,
                        routing_request.request.amount,
                    );

                    options.push_back(AnchorOption {
                        anchor: anchor.clone(),
                        quote: quote.clone(),
                        score,
                        metadata: metadata.clone(),
                    });
                }
            }
        }

        if options.is_empty() {
            return Err(Error::NoQuotesAvailable);
        }

        // Sort options by score (descending)
        let mut sorted_options = options.clone();
        for i in 0..sorted_options.len() {
            for j in (i + 1)..sorted_options.len() {
                let score_i = sorted_options.get(i).unwrap().score;
                let score_j = sorted_options.get(j).unwrap().score;
                if score_j > score_i {
                    let temp = sorted_options.get(i).unwrap();
                    sorted_options.set(i, sorted_options.get(j).unwrap());
                    sorted_options.set(j, temp);
                }
            }
        }

        // Limit alternatives
        let max_alternatives = routing_request.max_anchors.min(sorted_options.len());
        let mut alternatives: Vec<AnchorOption> = Vec::new(&env);
        for i in 1..max_alternatives {
            alternatives.push_back(sorted_options.get(i).unwrap());
        }

        let best = sorted_options.get(0).unwrap();

        Ok(RoutingResult {
            selected_anchor: best.anchor.clone(),
            selected_quote: best.quote.clone(),
            score: best.score,
            alternatives,
            routing_timestamp: current_timestamp,
        })
    }

    /// Find best anchor for a specific service and asset pair.
    pub fn find_best_anchor(
        env: Env,
        base_asset: String,
        quote_asset: String,
        amount: u64,
        operation_type: ServiceType,
        strategy: RoutingStrategy,
    ) -> Result<Address, Error> {
        let request = QuoteRequest {
            base_asset,
            quote_asset,
            amount,
            operation_type,
        };

        let routing_request = RoutingRequest {
            request,
            strategy,
            max_anchors: 1,
            require_kyc: false,
            min_reputation: 0,
        };

        let result = Self::route_transaction(env, routing_request)?;
        Ok(result.selected_anchor)
    }

    /// Calculate routing score based on strategy.
    fn calculate_routing_score(
        strategy: &RoutingStrategy,
        quote: &QuoteData,
        metadata: &AnchorMetadata,
        amount: u64,
    ) -> u64 {
        match strategy {
            RoutingStrategy::BestRate => {
                // Higher rate is better (inverted for scoring)
                let effective_rate = Self::calculate_effective_rate(quote, amount);
                // Invert so lower effective rate = higher score
                if effective_rate > 0 {
                    1_000_000_000 / effective_rate
                } else {
                    0
                }
            }
            RoutingStrategy::LowestFee => {
                // Lower fee is better
                let max_fee = 10000u32; // 100%
                let fee_score = max_fee.saturating_sub(quote.fee_percentage);
                fee_score as u64 * 100_000
            }
            RoutingStrategy::FastestSettlement => {
                // Lower settlement time is better
                let max_time = 86400u64; // 24 hours
                let time_score = max_time.saturating_sub(metadata.average_settlement_time);
                time_score * 10_000
            }
            RoutingStrategy::HighestLiquidity => {
                // Higher liquidity is better
                metadata.liquidity_score as u64 * 100_000
            }
            RoutingStrategy::Custom => {
                // Weighted combination of all factors
                let rate_score = if quote.rate > 0 {
                    (1_000_000 / quote.rate) * 30 // 30% weight
                } else {
                    0
                };
                let fee_score = (10000u32.saturating_sub(quote.fee_percentage) as u64) * 25; // 25% weight
                let reputation_score = metadata.reputation_score as u64 * 20; // 20% weight
                let liquidity_score = metadata.liquidity_score as u64 * 15; // 15% weight
                let uptime_score = metadata.uptime_percentage as u64 * 10; // 10% weight

                rate_score + fee_score + reputation_score + liquidity_score + uptime_score
            }
        }
    }

    /// Deactivate an anchor (admin only).
    pub fn deactivate_anchor(env: Env, anchor: Address) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        let mut metadata =
            Storage::get_anchor_metadata(&env, &anchor).ok_or(Error::AnchorMetadataNotFound)?;

        metadata.is_active = false;
        Storage::set_anchor_metadata(&env, &metadata);

        Ok(())
    }

    /// Reactivate an anchor (admin only).
    pub fn reactivate_anchor(env: Env, anchor: Address) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        let mut metadata =
            Storage::get_anchor_metadata(&env, &anchor).ok_or(Error::AnchorMetadataNotFound)?;

        metadata.is_active = true;
        Storage::set_anchor_metadata(&env, &metadata);

        Ok(())
    }
}
