#![cfg(test)]

mod extend_expiry_tests {
    use soroban_sdk::{
        symbol_short,
        testutils::{Address as _, Events, Ledger, LedgerInfo},
        xdr::ContractEventBody,
        Address, Bytes, Env, String, Symbol, TryFromVal,
    };

    use crate::contract::{AnchorKitContract, AnchorKitContractClient, Attestation};
    use crate::errors::ErrorCode;
    use crate::events::{AttestationExpiryExtended, AuditLogEvent};
    use crate::storage::StorageKey;

    const NOW: u64 = 1_000_000;
    const ID: u64 = 7;

    fn set_time(env: &Env, timestamp: u64) {
        env.ledger().set(LedgerInfo {
            timestamp,
            protocol_version: 26,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });
    }

    /// Set up a fresh contract and hand its handles to `f`.
    fn with_setup<F>(f: F)
    where
        F: FnOnce(&Env, &Address, &AnchorKitContractClient, &Address, &Address, &Address),
    {
        let env = Env::default();
        env.mock_all_auths();
        set_time(&env, NOW);

        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let attestor = Address::generate(&env);
        let other = Address::generate(&env);
        let subject = Address::generate(&env);
        // `initialize` is not needed for expiry extension; seed the admin so
        // the pause-path test can exercise the contract guard.
        env.as_contract(&contract_id, || {
            env.storage()
                .instance()
                .set(&crate::storage::key_admin(&env), &admin);
        });

        f(&env, &contract_id, &client, &attestor, &other, &subject);
    }

    /// Seed an attestation record directly. #1513 only mutates the expiry, so
    /// the tests do not need to exercise the submission/signature path.
    fn seed_attestation(
        env: &Env,
        contract_id: &Address,
        issuer: &Address,
        subject: &Address,
        expires_at: Option<u64>,
    ) {
        let payload_hash = Bytes::from_slice(env, &[0x11; 32]);
        env.as_contract(contract_id, || {
            env.storage().persistent().set(
                &StorageKey::Attest(ID),
                &Attestation {
                    id: ID,
                    issuer: issuer.clone(),
                    subject: subject.clone(),
                    timestamp: NOW,
                    payload_hash,
                    signature: Bytes::new(env),
                    issuer_revoked: false,
                    expires_at,
                },
            );
            // Subject index, so `list_attestations` reflects the expiry change.
            env.storage()
                .persistent()
                .set(&StorageKey::SubjectCount(subject.clone()), &1u64);
            env.storage()
                .persistent()
                .set(&StorageKey::SubjectAttestation(subject.clone(), 0), &ID);
        });
    }

    /// Locate event data by its first two topic symbols.
    fn event_data(env: &Env, topic0: Symbol, topic1: Symbol) -> Option<soroban_sdk::xdr::ScVal> {
        for event in env.events().all().events() {
            if let ContractEventBody::V0(body) = &event.body {
                if body.topics.len() >= 2
                    && Symbol::try_from_val(env, &body.topics[0]).ok() == Some(topic0.clone())
                    && Symbol::try_from_val(env, &body.topics[1]).ok() == Some(topic1.clone())
                {
                    return Some(body.data.clone());
                }
            }
        }
        None
    }

    #[test]
    fn issuer_can_extend_expiry_and_event_is_emitted() {
        with_setup(|env, contract_id, client, attestor, _other, subject| {
            let old_expiry = NOW + 100;
            let new_expiry = NOW + 500;
            seed_attestation(env, contract_id, attestor, subject, Some(old_expiry));

            client.extend_attestation_expiry(attestor, &ID, &new_expiry);
            // Soroban's test event log holds only the last invocation, so read
            // the events before making further client calls.
            let data = event_data(env, symbol_short!("attest"), symbol_short!("exp_ext"))
                .expect("expected an attestation expiry extension event");

            let stored = client.get_attestation(&ID).unwrap();
            assert_eq!(stored.id, ID);
            assert_eq!(stored.issuer, *attestor);
            assert_eq!(stored.expires_at, Some(new_expiry));

            let event = AttestationExpiryExtended::try_from_val(env, &data).unwrap();
            assert_eq!(event.attestation_id, ID);
            assert_eq!(event.issuer, *attestor);
            assert_eq!(event.old_expires_at, Some(old_expiry));
            assert_eq!(event.new_expires_at, new_expiry);
            assert_eq!(event.timestamp, NOW);
        });
    }

    #[test]
    fn extension_writes_audit_entry_and_event() {
        with_setup(|env, contract_id, client, attestor, _other, subject| {
            let new_expiry = NOW + 500;
            seed_attestation(env, contract_id, attestor, subject, Some(NOW + 100));

            client.extend_attestation_expiry(attestor, &ID, &new_expiry);
            let data = event_data(env, symbol_short!("audit"), symbol_short!("logged"))
                .expect("expected an audit log event");

            // Direct (session-less) calls use 0 for session/operation index.
            let audit = client.get_audit_log(&0u64);
            assert_eq!(audit.log_id, 0);
            assert_eq!(audit.session_id, 0);
            assert_eq!(audit.actor, *attestor);
            assert_eq!(
                audit.operation.operation_type,
                String::from_str(env, "extend_expiry")
            );
            assert_eq!(audit.operation.status, String::from_str(env, "success"));
            assert_eq!(
                audit.operation.result_summary,
                String::from_str(
                    env,
                    &alloc::format!("attestation_id={} new_expires_at={}", ID, new_expiry)
                )
            );

            let event = AuditLogEvent::try_from_val(env, &data).unwrap();
            assert_eq!(event.log_id, 0);
            assert_eq!(event.session_id, 0);
            assert_eq!(event.operation_type, String::from_str(env, "extend_expiry"));
        });
    }

    #[test]
    fn extended_expiry_keeps_attestation_listed_until_new_expiry() {
        with_setup(|env, contract_id, client, attestor, _other, subject| {
            let old_expiry = NOW + 100;
            let new_expiry = NOW + 500;
            seed_attestation(env, contract_id, attestor, subject, Some(old_expiry));

            assert_eq!(client.list_attestations(subject, &0, &10).len(), 1);
            client.extend_attestation_expiry(attestor, &ID, &new_expiry);

            set_time(env, old_expiry + 1);
            assert_eq!(client.list_attestations(subject, &0, &10).len(), 1);
            set_time(env, new_expiry + 1);
            assert_eq!(client.list_attestations(subject, &0, &10).len(), 0);
        });
    }

    #[test]
    fn non_issuer_cannot_extend_expiry() {
        with_setup(|env, contract_id, client, attestor, other, subject| {
            let old_expiry = NOW + 100;
            seed_attestation(env, contract_id, attestor, subject, Some(old_expiry));

            assert_eq!(
                client
                    .try_extend_attestation_expiry(other, &ID, &(NOW + 500))
                    .unwrap_err()
                    .unwrap(),
                ErrorCode::UnauthorizedAttestor.into()
            );
            assert_eq!(
                client.get_attestation(&ID).unwrap().expires_at,
                Some(old_expiry)
            );
        });
    }

    #[test]
    fn new_expiry_must_be_later_than_current() {
        with_setup(|env, contract_id, client, attestor, _other, subject| {
            let old_expiry = NOW + 100;
            seed_attestation(env, contract_id, attestor, subject, Some(old_expiry));

            for candidate in [old_expiry, old_expiry - 1] {
                assert_eq!(
                    client
                        .try_extend_attestation_expiry(attestor, &ID, &candidate)
                        .unwrap_err()
                        .unwrap(),
                    ErrorCode::InvalidTimestamp.into()
                );
            }
            assert_eq!(
                client.get_attestation(&ID).unwrap().expires_at,
                Some(old_expiry)
            );
        });
    }

    #[test]
    fn attestation_without_expiry_cannot_be_extended() {
        with_setup(|env, contract_id, client, attestor, _other, subject| {
            seed_attestation(env, contract_id, attestor, subject, None);

            assert_eq!(
                client
                    .try_extend_attestation_expiry(attestor, &ID, &(NOW + 500))
                    .unwrap_err()
                    .unwrap(),
                ErrorCode::InvalidTimestamp.into()
            );
            assert_eq!(client.get_attestation(&ID).unwrap().expires_at, None);
        });
    }

    #[test]
    fn unknown_attestation_cannot_be_extended() {
        with_setup(|_env, _contract_id, client, attestor, _other, _subject| {
            assert_eq!(
                client
                    .try_extend_attestation_expiry(attestor, &99u64, &(NOW + 500))
                    .unwrap_err()
                    .unwrap(),
                ErrorCode::AttestationNotFound.into()
            );
        });
    }

    #[test]
    fn extension_is_rejected_while_contract_is_paused() {
        with_setup(|env, contract_id, client, attestor, _other, subject| {
            let old_expiry = NOW + 100;
            seed_attestation(env, contract_id, attestor, subject, Some(old_expiry));
            client.pause_contract();

            assert_eq!(
                client
                    .try_extend_attestation_expiry(attestor, &ID, &(NOW + 500))
                    .unwrap_err()
                    .unwrap(),
                ErrorCode::ContractPaused.into()
            );
            assert_eq!(
                client.get_attestation(&ID).unwrap().expires_at,
                Some(old_expiry)
            );
        });
    }
}
