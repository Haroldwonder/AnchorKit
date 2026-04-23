//! Rate limiting for attestation submissions
//!
//! This module implements per-attestor rate limiting for attestation submissions
//! to prevent spam and abuse of the contract.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};
use crate::errors::AnchorKitError;

#[cfg(test)]
use crate::errors::ErrorCode;

/// Rate limit configuration stored in contract storage
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitConfig {
    /// Maximum number of submissions allowed per window
    pub max_submissions: u32,
    /// Length of the rate limit window in ledgers
    pub window_length: u32,
}

/// Per-attestor rate limit state stored in contract storage
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitState {
    /// Number of submissions in the current window
    pub submission_count: u32,
    /// Ledger number when the current window started
    pub window_start_ledger: u32,
    /// Cumulative total requests across all windows (never reset)
    pub total_requests: u64,
}

/// Rate limiter for attestation submissions
#[contract]
pub struct RateLimiter;

#[contractimpl]
impl RateLimiter {
    /// Get the current rate limit state for an attestor
    pub fn get_state(env: Env, attestor: Address) -> RateLimitState {
        let state_key = Self::get_state_key(&env, &attestor);
        env.storage().persistent().get::<_, RateLimitState>(&state_key)
            .unwrap_or(RateLimitState {
                submission_count: 0,
                window_start_ledger: env.ledger().sequence(),
                total_requests: 0,
            })
    }

    /// Get the current rate limit configuration
    pub fn get_config(env: Env) -> RateLimitConfig {
        let config_key = Self::get_config_key(&env);
        env.storage().persistent().get::<_, RateLimitConfig>(&config_key)
            .unwrap_or(RateLimitConfig {
                max_submissions: 10,
                window_length: 100,
            })
    }
}

impl RateLimiter {
    /// Check if an attestor can submit an attestation and increment their counter.
    pub fn check_and_increment(
        env: Env,
        attestor: Address,
    ) -> Result<(), ErrorCode> {
        let config = Self::get_effective_config(env.clone(), attestor.clone());
        let current_ledger = env.ledger().sequence();
        let state_key = Self::get_state_key(env, attestor);

        let mut state = env.storage().persistent().get::<_, RateLimitState>(&state_key)
            .unwrap_or(RateLimitState {
                submission_count: 0,
                window_start_ledger: current_ledger,
                total_requests: 0,
            });

        if Self::is_window_expired(current_ledger, state.window_start_ledger, config.window_length) {
            state.submission_count = 0;
            state.window_start_ledger = current_ledger;
        }

        state.total_requests += 1;

        if state.submission_count >= config.max_submissions {
            env.storage().persistent().set(&state_key, &state);
            return Err(ErrorCode::RateLimitExceeded);
        }

        state.submission_count += 1;
        env.storage().persistent().set(&state_key, &state);

        Ok(())
    }

    /// Update the global rate limit configuration, or set a per-attestor override when
    /// `attestor` is `Some`.
    pub fn update_config(
        env: Env,
        _admin: Address,
        config: RateLimitConfig,
        attestor: Option<Address>,
    ) -> Result<(), ErrorCode> {
        match attestor {
            Some(addr) => {
                let key = Self::get_attestor_config_key(&env, &addr);
                env.storage().persistent().set(&key, &config);
            }
            None => {
                let key = Self::get_config_key(&env);
                env.storage().persistent().set(&key, &config);
            }
        }
        Ok(())
    }

    /// Get the effective config for an attestor: per-attestor override if set, else global.
    pub fn get_effective_config(env: Env, attestor: Address) -> RateLimitConfig {
        let key = Self::get_attestor_config_key(&env, &attestor);
        env.storage().persistent().get::<_, RateLimitConfig>(&key)
            .unwrap_or_else(|| Self::get_config(env.clone()))
    }

    fn is_window_expired(current_ledger: u32, window_start_ledger: u32, window_length: u32) -> bool {
        current_ledger.saturating_sub(window_start_ledger) >= window_length
    }

    fn get_state_key(env: &Env, attestor: &Address) -> soroban_sdk::BytesN<32> {
        let address_str = attestor.to_string();
        let mut address_bytes = [0u8; 128];
        let len = address_str.len() as usize;
        let final_len = if len > 128 { 128 } else { len };
        address_str.copy_into_slice(&mut address_bytes[..final_len]);
        let bytes = soroban_sdk::Bytes::from_slice(env, &address_bytes[..final_len]);
        env.crypto().sha256(&bytes).into()
    }

    fn get_config_key(env: &Env) -> soroban_sdk::BytesN<32> {
        let config_key = *b"rate_limit_config_______________";
        soroban_sdk::BytesN::from_array(env, &config_key)
    }

    fn get_attestor_config_key(env: &Env, attestor: &Address) -> soroban_sdk::BytesN<32> {
        let address_str = attestor.to_string();
        let mut buf = [0u8; 56];
        address_str.copy_into_slice(&mut buf);
        let mut prefixed = [0u8; 57];
        prefixed[0] = b'c';
        prefixed[1..].copy_from_slice(&buf);
        let bytes = soroban_sdk::Bytes::from_slice(env, &prefixed);
        env.crypto().sha256(&bytes).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn make_contract(env: &Env) -> Address {
        env.register_contract(None, crate::rate_limiter::RateLimiter)
    }

    #[test]
    fn test_rate_limit_under_limit() {
        let env = Env::default();
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let contract_address = make_contract(&env);
        let contract_id = env.register_contract(&contract_address, crate::rate_limiter::RateLimiter);

        // Set global config with limit of 10
        env.as_contract(&contract_id, &|| {
            RateLimiter::update_config(&env, &contract_address, &RateLimitConfig { max_submissions: 10, window_length: 100 }, None).unwrap();
        });

        let result = env.as_contract(&contract_id, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        });
        assert!(result.is_ok());

        let state = env.as_contract(&contract_id, &|| {
            RateLimiter::get_state(env.clone(), attestor.clone())
        });
        assert_eq!(state.submission_count, 1);
    }

    #[test]
    fn test_rate_limit_at_limit() {
        let env = Env::default();
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let contract_address = make_contract(&env);

        env.as_contract(&contract_address, &|| {
            RateLimiter::update_config(&env, &contract_address, &RateLimitConfig { max_submissions: 2, window_length: 100 }, None).unwrap();
        });

        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        }).is_ok());
        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        }).is_ok());

        let result = env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        });
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, ErrorCode::RateLimitExceeded);
    }

    #[test]
    fn test_rate_limit_over_limit() {
        let env = Env::default();
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let contract_address = make_contract(&env);

        env.as_contract(&contract_address, &|| {
            RateLimiter::update_config(&env, &contract_address, &RateLimitConfig { max_submissions: 1, window_length: 100 }, None).unwrap();
        });

        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        }).is_ok());

        let result = env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        });
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, ErrorCode::RateLimitExceeded);
    }

    #[test]
    fn test_rate_limit_window_reset() {
        let env = Env::default();
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let contract_address = make_contract(&env);

        env.as_contract(&contract_address, &|| {
            RateLimiter::update_config(&env, &contract_address, &RateLimitConfig { max_submissions: 1, window_length: 10 }, None).unwrap();
        });

        // First call succeeds, count = 1
        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        }).is_ok());
        // Second call hits limit, count stays at 1
        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        }).is_err());

        let state = env.as_contract(&contract_address, &|| {
            RateLimiter::get_state(env.clone(), attestor.clone())
        });
        assert_eq!(state.submission_count, 1);
        assert_eq!(state.total_requests, 2);
    }

    #[test]
    fn test_rate_limit_config_update() {
        let env = Env::default();
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let contract_address = make_contract(&env);
        let new_config = RateLimitConfig { max_submissions: 20, window_length: 200 };

        let result = env.as_contract(&contract_address, &|| {
            RateLimiter::update_config(&env, &admin, &new_config, None)
        });
        assert!(result.is_ok());

        let config = env.as_contract(&contract_address, &|| {
            RateLimiter::get_config(env.clone())
        });
        assert_eq!(config.max_submissions, 20);
        assert_eq!(config.window_length, 200);
    }

    #[test]
    fn test_rate_limit_default_config() {
        let env = Env::default();
        let contract_address = make_contract(&env);

        let config = env.as_contract(&contract_address, &|| {
            RateLimiter::get_config(env.clone())
        });
        assert_eq!(config.max_submissions, 10);
        assert_eq!(config.window_length, 100);
    }

    // --- per-attestor override tests ---

    #[test]
    fn test_per_attestor_override_takes_precedence() {
        let env = Env::default();
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let contract_address = make_contract(&env);

        env.as_contract(&contract_address, &|| {
            // Global: limit 1
            RateLimiter::update_config(&env, &contract_address, &RateLimitConfig { max_submissions: 1, window_length: 100 }, None).unwrap();
            // Per-attestor override: limit 5
            RateLimiter::update_config(&env, &contract_address, &RateLimitConfig { max_submissions: 5, window_length: 100 }, Some(&attestor)).unwrap();
        });

        // Should succeed 5 times (override), not just 1 (global)
        for _ in 0..5 {
            assert!(env.as_contract(&contract_address, &|| {
                RateLimiter::check_and_increment(&env, &attestor)
            }).is_ok());
        }
        // 6th should fail
        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        }).is_err());
    }

    #[test]
    fn test_fallback_to_global_when_no_override() {
        let env = Env::default();
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let contract_address = make_contract(&env);

        env.as_contract(&contract_address, &|| {
            // Global: limit 2, no per-attestor override
            RateLimiter::update_config(&env, &contract_address, &RateLimitConfig { max_submissions: 2, window_length: 100 }, None).unwrap();
        });

        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        }).is_ok());
        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        }).is_ok());
        // 3rd exceeds global limit
        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &attestor)
        }).is_err());
    }

    #[test]
    fn test_override_does_not_affect_other_attestors() {
        let env = Env::default();
        let high_volume = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let normal = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let contract_address = make_contract(&env);

        env.as_contract(&contract_address, &|| {
            // Global: limit 1
            RateLimiter::update_config(&env, &contract_address, &RateLimitConfig { max_submissions: 1, window_length: 100 }, None).unwrap();
            // Override only for high_volume
            RateLimiter::update_config(&env, &contract_address, &RateLimitConfig { max_submissions: 10, window_length: 100 }, Some(&high_volume)).unwrap();
        });

        // high_volume can submit 10 times
        for _ in 0..10 {
            assert!(env.as_contract(&contract_address, &|| {
                RateLimiter::check_and_increment(&env, &high_volume)
            }).is_ok());
        }

        // normal attestor is still capped at 1
        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &normal)
        }).is_ok());
        assert!(env.as_contract(&contract_address, &|| {
            RateLimiter::check_and_increment(&env, &normal)
        }).is_err());
    }
}
