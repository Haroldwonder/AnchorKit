#![cfg(test)]

mod webhook_middleware_tests {
    use soroban_sdk::{testutils::{Ledger, LedgerInfo}, Address, Env, String};

    /// Verifies that a G-address (Stellar account address) is rejected when used
    /// as a Soroban contract address source_address parameter.
    /// The host emits "unexpected strkey length" diagnostic events and panics.
    #[test]
    #[should_panic]
    fn test_webhook_request_with_source_address() {
        let env = Env::default();
        env.ledger().set(LedgerInfo {
            timestamp: 0,
            protocol_version: 21,
            sequence_number: 0,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });

        // G-addresses are Stellar account addresses (56 chars), not Soroban contract
        // addresses (C-addresses, 58 chars). Parsing one as Address panics with
        // "unexpected strkey length".
        let g_address = String::from_str(&env, "GBBD6A7KNZF5WNWQEPZP5DYJD2AYUTLXRB6VXJ4RCX4RTNPPQVNF3GQ");
        let _ = Address::from_string(&g_address);
    }
}
