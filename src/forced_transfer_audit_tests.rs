#![cfg(test)]

// Issue #958 – Verify forced transfers append exactly one audit record.
//
// "Forced transfer" in AnchorKit is an exceptional administrative action
// modelled by `revoke_attestor_with_session`: an admin forcibly removes a
// registered holder (attestor) outside the normal flow and the action MUST
// produce exactly one immutable audit log entry.
//
// The tests assert:
//   a) One successful forced transfer increments the audit counter by exactly 1
//      and the resulting log entry is present and correct.
//   b) A rejected forced transfer (invalid target – attestor not registered)
//      does NOT write any new audit log entry; the counter is unchanged.

mod forced_transfer_audit_tests {
    use soroban_sdk::{
        testutils::{Address as _, Ledger, LedgerInfo},
        Address, Env,
    };
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    use crate::contract::{AnchorKitContract, AnchorKitContractClient};
    use crate::sep10_test_util::register_attestor_with_sep10;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn make_env() -> Env {
        let env = Env::default();
        env.mock_all_auths();
        env
    }

    fn setup_ledger(env: &Env) {
        env.ledger().set(LedgerInfo {
            timestamp: 1_700_000_000,
            protocol_version: 21,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 100,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6_312_000,
        });
    }

    fn setup(env: &Env) -> (AnchorKitContractClient<'static>, Address) {
        let contract_id = env.register_contract(None, AnchorKitContract);
        let client = AnchorKitContractClient::new(env, &contract_id);
        let admin = Address::generate(env);
        client.initialize(&admin, &1_000_u64, &None, &None);
        (client, admin)
    }

    /// Read the current audit log counter (the ID that will be assigned to the
    /// *next* entry).  We derive it from `get_audit_log_offset` plus the fact
    /// that `get_audit_log` panics for IDs that don't exist, so we probe
    /// upward from the offset until we hit a missing entry.
    ///
    /// In practice a freshly-initialised contract starts at log_id 0 and each
    /// session-based operation increments it by 1, so this helper is reliable
    /// for small test sequences.
    fn current_log_count(client: &AnchorKitContractClient) -> u64 {
        let offset = client.get_audit_log_offset();
        // Walk forward until get_audit_log returns an error (try_ variant).
        let mut id = offset;
        loop {
            if client.try_get_audit_log(&id).is_err() {
                break;
            }
            id += 1;
        }
        id - offset // number of live entries
    }

    // -----------------------------------------------------------------------
    // Test 1: one successful forced transfer → exactly one new audit record
    // -----------------------------------------------------------------------

    #[test]
    fn test_successful_forced_transfer_appends_exactly_one_audit_record() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _admin) = setup(&env);

        // Register a holder (attestor) that will be forcibly removed.
        let holder = Address::generate(&env);
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &holder, &holder, &sk);

        // Open a session so we can use the session-aware revoke method.
        let session_id = client.create_session(&holder);

        // Snapshot the log count before the forced transfer.
        let before = current_log_count(&client);

        // Perform the forced transfer (admin removes the holder).
        client.revoke_attestor_with_session(&session_id, &holder);

        // Exactly one new entry must have been appended.
        let after = current_log_count(&client);
        assert_eq!(
            after - before,
            1,
            "forced transfer must append exactly 1 audit record"
        );

        // Retrieve the new entry and verify its contents.
        let log_id = before + client.get_audit_log_offset();
        let entry = client.get_audit_log(&log_id);
        assert_eq!(entry.session_id, session_id);
        assert_eq!(entry.actor, _admin);

        // operation_type must identify this as a revocation.
        let op_type = entry.operation.operation_type;
        assert_eq!(
            op_type,
            soroban_sdk::String::from_str(&env, "revoke"),
            "audit entry operation_type must be 'revoke'"
        );
        assert_eq!(
            entry.operation.status,
            soroban_sdk::String::from_str(&env, "success")
        );

        // The holder must no longer be registered after the forced transfer.
        assert!(
            !client.is_attestor(&holder),
            "holder must not be registered after forced transfer"
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: rejected forced transfer (unregistered target) → zero new audit
    // -----------------------------------------------------------------------

    #[test]
    fn test_rejected_forced_transfer_does_not_append_audit_record() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _admin) = setup(&env);

        // We need a session initiator who IS a registered attestor so
        // create_session + revoke_attestor_with_session can be called.
        let real_holder = Address::generate(&env);
        let sk = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &real_holder, &real_holder, &sk);

        let session_id = client.create_session(&real_holder);

        // Snapshot before attempting the bad transfer.
        let before = current_log_count(&client);

        // Attempt to forcibly transfer an address that was never registered.
        let ghost = Address::generate(&env);
        let result = client.try_revoke_attestor_with_session(&session_id, &ghost);

        // The call must fail (AttestorNotRegistered).
        assert!(
            result.is_err(),
            "revoking an unregistered holder must fail"
        );

        // The audit log count must be unchanged.
        let after = current_log_count(&client);
        assert_eq!(
            after,
            before,
            "a rejected forced transfer must not append any audit record"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: two consecutive forced transfers → exactly two audit records
    // -----------------------------------------------------------------------

    #[test]
    fn test_two_forced_transfers_append_two_independent_audit_records() {
        let env = make_env();
        setup_ledger(&env);
        let (client, _admin) = setup(&env);

        // Register two holders.
        let h1 = Address::generate(&env);
        let h2 = Address::generate(&env);
        let sk1 = SigningKey::generate(&mut OsRng);
        let sk2 = SigningKey::generate(&mut OsRng);
        register_attestor_with_sep10(&env, &client, &h1, &h1, &sk1);
        register_attestor_with_sep10(&env, &client, &h2, &h2, &sk2);

        let session_id = client.create_session(&h1);
        let before = current_log_count(&client);

        client.revoke_attestor_with_session(&session_id, &h1);
        client.revoke_attestor_with_session(&session_id, &h2);

        let after = current_log_count(&client);
        assert_eq!(
            after - before,
            2,
            "two forced transfers must produce exactly two audit records"
        );

        // Both entries must be individually retrievable and distinct.
        let offset = client.get_audit_log_offset();
        let entry_a = client.get_audit_log(&(before + offset));
        let entry_b = client.get_audit_log(&(before + offset + 1));

        assert_ne!(
            entry_a.log_id, entry_b.log_id,
            "the two audit records must have distinct log IDs"
        );
        assert_eq!(
            entry_a.operation.operation_type,
            soroban_sdk::String::from_str(&env, "revoke")
        );
        assert_eq!(
            entry_b.operation.operation_type,
            soroban_sdk::String::from_str(&env, "revoke")
        );
    }
}
