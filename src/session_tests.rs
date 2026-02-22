#[cfg(test)]
mod session_tests {
    use crate::{AnchorKitContract, AnchorKitContractClient, OperationContext};
    use soroban_sdk::{testutils::Address as _, xdr::ToXdr, Address, Bytes, BytesN, Env};

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct ReplaySnapshot {
        session_id: u64,
        operation_count: u64,
        attestation_id_1: u64,
        attestation_id_2: u64,
        log_0_hash: [u8; 32],
        log_1_hash: [u8; 32],
        log_2_hash: [u8; 32],
    }

    fn create_test_contract(env: &Env) -> AnchorKitContractClient<'_> {
        let contract_id = env.register_contract(None, AnchorKitContract);
        AnchorKitContractClient::new(env, &contract_id)
    }

    fn hash_operation_context(env: &Env, op: &OperationContext) -> [u8; 32] {
        let xdr = op.clone().to_xdr(env);
        let hash: BytesN<32> = env.crypto().sha256(&xdr).into();
        hash.to_array()
    }

    fn run_recorded_session_replay(env: &Env) -> ReplaySnapshot {
        env.mock_all_auths();

        let admin = Address::generate(env);
        let initiator = Address::generate(env);
        let attestor = Address::generate(env);
        let subject = Address::generate(env);

        let client = create_test_contract(env);
        client.initialize(&admin);

        let session_id = client.create_session(&initiator);

        // Replay a fixed, recorded workflow offline:
        // 1) register attestor, 2) submit attestation, 3) submit second attestation.
        client.register_attestor_with_session(&session_id, &attestor);

        let payload_hash_1 = BytesN::from_array(env, &[1; 32]);
        let signature_1 = Bytes::from_slice(env, &[10, 11, 12, 13]);
        let attestation_id_1 = client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &1_700_000_001u64,
            &payload_hash_1,
            &signature_1,
        );

        let payload_hash_2 = BytesN::from_array(env, &[2; 32]);
        let signature_2 = Bytes::from_slice(env, &[20, 21, 22, 23]);
        let attestation_id_2 = client.submit_attestation_with_session(
            &session_id,
            &attestor,
            &subject,
            &1_700_000_002u64,
            &payload_hash_2,
            &signature_2,
        );

        let operation_count = client.get_session_operation_count(&session_id);
        let log_0 = client.get_audit_log(&0u64);
        let log_1 = client.get_audit_log(&1u64);
        let log_2 = client.get_audit_log(&2u64);

        ReplaySnapshot {
            session_id,
            operation_count,
            attestation_id_1,
            attestation_id_2,
            log_0_hash: hash_operation_context(env, &log_0.operation),
            log_1_hash: hash_operation_context(env, &log_1.operation),
            log_2_hash: hash_operation_context(env, &log_2.operation),
        }
    }

    #[test]
    fn test_recorded_anchor_session_replay_is_reproducible_offline() {
        let env_1 = Env::default();
        let env_2 = Env::default();

        let replay_1 = run_recorded_session_replay(&env_1);
        let replay_2 = run_recorded_session_replay(&env_2);

        assert_eq!(replay_1.operation_count, 3);
        assert_eq!(replay_1.attestation_id_1, 0);
        assert_eq!(replay_1.attestation_id_2, 1);

        assert_eq!(
            replay_1, replay_2,
            "Recorded workflow replay must be deterministic across isolated offline runs"
        );
    }
}
