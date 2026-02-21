#![no_std]

mod config;
mod errors;
mod events;
mod storage;
mod types;
mod validation;

#[cfg(test)]
mod config_tests;

use soroban_sdk::{contract, contractimpl, Address, Bytes, BytesN, Env, String, Vec};

pub use config::{AttestorConfig, ContractConfig, SessionConfig};
pub use errors::Error;
pub use events::{
    AttestationRecorded, AttestorAdded, AttestorRemoved, EndpointConfigured, EndpointRemoved,
    OperationLogged, QuoteSubmitted, ServicesConfigured, SessionCreated,
};
pub use storage::Storage;
pub use types::{
    AnchorServices, Attestation, AuditLog, Endpoint, InteractionSession, OperationContext,
    QuoteData, QuoteRequest, RateComparison, ServiceType, TransactionIntent,
    TransactionIntentBuilder,
};
pub use validation::{validate_init_config, validate_attestor_batch, validate_session_config};

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
    pub fn batch_register_attestors(
        env: Env,
        attestors: Vec<AttestorConfig>,
    ) -> Result<(), Error> {
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
    pub fn configure_session_settings(
        env: Env,
        config: SessionConfig,
    ) -> Result<(), Error> {
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

        if timestamp == 0 {
            return Err(Error::InvalidTimestamp);
        }

        if !Storage::is_attestor(&env, &issuer) {
            return Err(Error::UnauthorizedAttestor);
        }

        if Storage::is_hash_used(&env, &payload_hash) {
            return Err(Error::ReplayAttack);
        }

        Self::verify_signature(&env, &issuer, &subject, timestamp, &payload_hash, &signature)?;

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

    /// Configure an endpoint for an attestor. Callable by the attestor.
    pub fn configure_endpoint(env: Env, attestor: Address, url: String) -> Result<(), Error> {
        Storage::get_admin(&env)?;
        attestor.require_auth();

        Self::validate_endpoint_url(&url)?;

        if !Storage::is_attestor(&env, &attestor) {
            return Err(Error::AttestorNotRegistered);
        }

        if Storage::has_endpoint(&env, &attestor) {
            return Err(Error::EndpointAlreadyExists);
        }

        let endpoint = Endpoint {
            url: url.clone(),
            attestor: attestor.clone(),
            is_active: true,
        };

        Storage::set_endpoint(&env, &endpoint);

        EndpointConfigured { attestor, url }.publish(&env);

        Ok(())
    }

    /// Update an existing endpoint for an attestor. Callable by the attestor.
    pub fn update_endpoint(
        env: Env,
        attestor: Address,
        url: String,
        is_active: bool,
    ) -> Result<(), Error> {
        Storage::get_admin(&env)?;
        attestor.require_auth();

        Self::validate_endpoint_url(&url)?;

        if !Storage::has_endpoint(&env, &attestor) {
            return Err(Error::EndpointNotFound);
        }

        let endpoint = Endpoint {
            url: url.clone(),
            attestor: attestor.clone(),
            is_active,
        };

        Storage::set_endpoint(&env, &endpoint);

        EndpointConfigured { attestor, url }.publish(&env);

        Ok(())
    }

    /// Remove an endpoint for an attestor. Only callable by admin.
    pub fn remove_endpoint(env: Env, attestor: Address) -> Result<(), Error> {
        let admin = Storage::get_admin(&env)?;
        admin.require_auth();

        if !Storage::has_endpoint(&env, &attestor) {
            return Err(Error::EndpointNotFound);
        }

        Storage::remove_endpoint(&env, &attestor);
        EndpointRemoved { attestor }.publish(&env);

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

        Self::verify_signature(&env, &issuer, &subject, timestamp, &payload_hash, &signature)?;

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

        let mut best_quote = valid_quotes.get(0).unwrap();
        let mut best_effective_rate = Self::calculate_effective_rate(&best_quote, request.amount);

        for i in 1..valid_quotes.len() {
            let quote = valid_quotes.get(i).unwrap();
            let effective_rate = Self::calculate_effective_rate(&quote, request.amount);

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
}
