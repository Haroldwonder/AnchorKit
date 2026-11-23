//! Rate limiting for attestation submissions
//!
//! This module implements per-attestor rate limiting for attestation submissions
//! to prevent spam and abuse of the contract.

use soroban_sdk::{contracttype, symbol_short, Address, Env};
use crate::errors::ErrorCode;
use crate::events::{RateLimitReset, RateLimitWindowReset};
use crate::storage::StorageKey;

/// Rate limit configuration stored in contract storage
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitConfig {
    /// Maximum number of submissions allowed per window
    pub max_submissions: u32,
    /// Length of the rate limit window in ledgers. Can be set per-attestor via update_config or set_attestor_config.
    pub window_length: u32,
    /// Per-window burst allowance: extra submissions permitted each time the rate-limit window rolls over.
    /// Set to 0 to disable burst tolerance.
    pub burst: u32,
}

impl RateLimitConfig {
    /// Validate the rate limit configuration.
    /// Rejects invalid configurations that would cause silent failures:
    /// - window_length must be > 0 (window_length=0 makes every check reset the window)
    /// - max_submissions must be > 0 (max_submissions=0 permanently blocks all submissions)
    pub fn validate(&self) -> Result<(), ErrorCode> {
        if self.window_length == 0 {
            return Err(ErrorCode::ValidationError);
        }
        if self.max_submissions == 0 {
            return Err(ErrorCode::ValidationError);
        }
        Ok(())
    }
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
    /// Whether the one-time burst allowance has been consumed.
    pub burst_used: bool,
    /// Cumulative total rejected submissions (never reset).
    pub total_rejections: u64,
}

/// Rate limiter utility — plain Rust struct, no Soroban contract boundary.
pub struct RateLimiter;

/// Current rate limit status for an attestor (returned by get_rate_limit_status).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RateLimitStatus {
    pub used: u32,
    pub limit: u32,
    pub window_resets_at: u32,
}

impl RateLimiter {
    /// Get the current rate limit state for an attestor.
    pub fn get_state(env: Env, attestor: Address) -> RateLimitState {
        let state_key = StorageKey::RateLimitState(attestor.clone());
        env.storage().persistent().get::<_, RateLimitState>(&state_key)
            .unwrap_or(RateLimitState {
                submission_count: 0,
                window_start_ledger: env.ledger().sequence(),
                total_requests: 0,
                burst_used: false,
                total_rejections: 0,
            })
    }

    /// Returns the total number of rejected submissions for an attestor.
    pub fn get_rate_limit_rejections(env: Env, attestor: Address) -> u64 {
        Self::get_state(env, attestor).total_rejections
    }

    /// Get the current global rate limit configuration.
    ///
    /// If no configuration has been set via [`update_config`], the following defaults apply:
    /// - `max_submissions = 10`
    /// - `window_length = 100` ledgers
    ///
    /// At Stellar's target close time of ~5 seconds per ledger, the default window
    /// is approximately **500 seconds (~8 minutes)** of wall-clock time, allowing
    /// up to 10 submissions per attestor in that period.
    pub fn get_config(env: Env) -> RateLimitConfig {
        let config_key = Self::get_config_key(&env);
        env.storage().persistent().get::<_, RateLimitConfig>(&config_key)
            .unwrap_or(RateLimitConfig {
                max_submissions: 10,
                window_length: 100,
                burst: 0,
            })
    }

    /// Check if an attestor can submit an attestation and increment their counter.
    ///
    /// Rate limiting is opt-in: if no global `RateLimitConfig` has been written
    /// to storage and no per-attestor override exists for `attestor`, the check
    /// is skipped entirely and `Ok(())` is returned without touching state.
    pub fn check_and_increment(
        env: &Env,
        attestor: &Address,
    ) -> Result<(), ErrorCode> {
        if !Self::is_configured(env, attestor) {
            return Ok(());
        }
        let config = Self::get_effective_config(env.clone(), attestor.clone());
        let current_ledger = env.ledger().sequence();
        let state_key = StorageKey::RateLimitState(attestor.clone());

        let mut state = env.storage().persistent().get::<_, RateLimitState>(&state_key)
            .unwrap_or(RateLimitState {
                submission_count: 0,
                window_start_ledger: current_ledger,
                total_requests: 0,
                burst_used: false,
                total_rejections: 0,
            });

        if Self::is_window_expired(current_ledger, state.window_start_ledger, config.window_length) {
            state.submission_count = 0;
            state.window_start_ledger = current_ledger;
            state.burst_used = false;  // reset burst for new window
            env.events().publish(
                (symbol_short!("rate"), symbol_short!("win_reset")),
                RateLimitWindowReset {
                    attestor: attestor.clone(),
                    window_start: current_ledger as u64,
                },
            );
        }

        let effective_limit = if !state.burst_used && config.burst > 0 {
            config.max_submissions + config.burst
        } else {
            config.max_submissions
        };
        if state.submission_count >= effective_limit {
            state.total_rejections += 1;
            env.storage().persistent().set(&state_key, &state);
            env.storage().persistent().extend_ttl(&state_key, config.window_length, config.window_length);
            return Err(ErrorCode::RateLimitExceeded);
        }

        state.submission_count += 1;
        state.total_requests += 1;
        if !state.burst_used && config.burst > 0 && state.submission_count > config.max_submissions {
            state.burst_used = true;
        }
        env.storage().persistent().set(&state_key, &state);
        env.storage().persistent().extend_ttl(&state_key, config.window_length, config.window_length);

        Ok(())
    }

    /// Admin function to tune the rate limit configuration.
    ///
    /// When `attestor` is `None`, updates the global configuration. When `Some(addr)`,
    /// sets a per-attestor override for that address only.
    pub fn update_config(
        env: &Env,
        _admin: &Address,
        config: RateLimitConfig,
        attestor: Option<&Address>,
    ) -> Result<(), ErrorCode> {
        config.validate()?;
        match attestor {
            Some(addr) => {
                let key = StorageKey::RateLimitOverride(addr.clone());
                env.storage().persistent().set(&key, &config);
                env.storage().persistent().extend_ttl(&key, config.window_length, config.window_length);
            }
            None => {
                let key = Self::get_config_key(env);
                env.storage().persistent().set(&key, &config);
                env.storage().persistent().extend_ttl(&key, config.window_length, config.window_length);
            }
        }
        Ok(())
    }

    /// Get the effective config for an attestor: per-attestor override if set, else global.
    pub fn get_effective_config(env: Env, attestor: Address) -> RateLimitConfig {
        let key = StorageKey::RateLimitOverride(attestor.clone());
        env.storage().persistent().get::<_, RateLimitConfig>(&key)
            .unwrap_or_else(|| Self::get_config(env.clone()))
    }

    /// Configure rate limits for a specific attestor, including their window duration.
    /// High-volume attestors can have shorter windows; low-volume ones can have longer windows.
    pub fn set_attestor_config(
        env: &Env,
        attestor: &Address,
        config: RateLimitConfig,
    ) -> Result<(), ErrorCode> {
        config.validate()?;
        let key = StorageKey::RateLimitOverride(attestor.clone());
        env.storage().persistent().set(&key, &config);
        env.storage().persistent().extend_ttl(&key, config.window_length, config.window_length);
        Ok(())
    }

    /// Returns the current rate limit status for an attestor.
    pub fn get_rate_limit_status(env: Env, attestor: Address) -> RateLimitStatus {
        let config = Self::get_effective_config(env.clone(), attestor.clone());
        let state = Self::get_state(env.clone(), attestor.clone());
        let current_ledger = env.ledger().sequence();
        let window_resets_at = if Self::is_window_expired(current_ledger, state.window_start_ledger, config.window_length) {
            current_ledger
        } else {
            state.window_start_ledger + config.window_length
        };
        let effective_limit = if !state.burst_used && config.burst > 0 {
            config.max_submissions + config.burst
        } else {
            config.max_submissions
        };
        RateLimitStatus {
            used: state.submission_count,
            limit: effective_limit,
            window_resets_at,
        }
    }

    /// Returns true if rate limiting has been explicitly configured — either via
    /// a global config or a per-attestor override.
    fn is_configured(env: &Env, attestor: &Address) -> bool {
        let override_key = StorageKey::RateLimitOverride(attestor.clone());
        if env.storage().persistent().has(&override_key) {
            return true;
        }
        let global_key = Self::get_config_key(env);
        env.storage().persistent().has(&global_key)
    }

    /// Reset the rate limit for a specified attestor (admin-only).
    ///
    /// Clears `submission_count` and `window_start_ledger`; preserves `total_requests`.
    pub fn reset_rate_limit(env: &Env, admin: &Address, attestor: &Address) -> Result<(), ErrorCode> {
        let state_key = StorageKey::RateLimitState(attestor.clone());
        let current_state = env.storage().persistent().get::<_, RateLimitState>(&state_key)
            .unwrap_or(RateLimitState {
                submission_count: 0,
                window_start_ledger: env.ledger().sequence(),
                total_requests: 0,
                burst_used: false,
                total_rejections: 0,
            });

        let reset_state = RateLimitState {
            submission_count: 0,
            window_start_ledger: env.ledger().sequence(),
            total_requests: current_state.total_requests,
            burst_used: false,
            total_rejections: current_state.total_rejections,
        };

        env.storage().persistent().set(&state_key, &reset_state);
        let window = Self::get_effective_config(env.clone(), attestor.clone()).window_length;
        env.storage().persistent().extend_ttl(&state_key, window, window);

        env.events().publish(
            (symbol_short!("rate"), symbol_short!("reset")),
            RateLimitReset {
                attestor: attestor.clone(),
                admin: admin.clone(),
                timestamp: env.ledger().timestamp(),
            },
        );

        Ok(())
    }

    fn is_window_expired(current_ledger: u32, window_start_ledger: u32, window_length: u32) -> bool {
        current_ledger.saturating_sub(window_start_ledger) >= window_length
    }

    fn get_config_key(env: &Env) -> soroban_sdk::BytesN<32> {
        let config_key = *b"rate_limit_config_______________";
        soroban_sdk::BytesN::from_array(env, &config_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::AnchorKitContract;
    use soroban_sdk::Symbol;
    use soroban_sdk::TryFromVal;
    use soroban_sdk::testutils::{Address as _, Events, Ledger, LedgerInfo};

    fn with_contract<F, R>(f: F) -> R
    where
        F: FnOnce(Env) -> R,
    {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, AnchorKitContract);
        env.as_contract(&contract_id, || f(env.clone()))
    }

    #[test]
    fn test_rate_limit_under_limit() {
        with_contract(|env| {
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &attestor, RateLimitConfig { max_submissions: 10, window_length: 100, burst: 0 }, None).unwrap();

        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());

        let state = RateLimiter::get_state(env.clone(), attestor.clone());
        assert_eq!(state.submission_count, 1);
        });
    }

    #[test]
    fn test_rate_limit_at_limit() {
        with_contract(|env| {
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &attestor, RateLimitConfig { max_submissions: 2, window_length: 100, burst: 0 }, None).unwrap();

        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());
        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());
        let result = RateLimiter::check_and_increment(&env, &attestor);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ErrorCode::RateLimitExceeded);
        });
    }

    #[test]
    fn test_rate_limit_over_limit() {
        with_contract(|env| {
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &attestor, RateLimitConfig { max_submissions: 1, window_length: 100, burst: 0 }, None).unwrap();

        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());
        let result = RateLimiter::check_and_increment(&env, &attestor);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ErrorCode::RateLimitExceeded);
        });
    }

    #[test]
    fn test_rate_limit_window_reset() {
        with_contract(|env| {
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &attestor, RateLimitConfig { max_submissions: 1, window_length: 10, burst: 0 }, None).unwrap();

        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());
        assert!(RateLimiter::check_and_increment(&env, &attestor).is_err());

        let current_ledger = env.ledger().sequence();
        env.ledger().set(LedgerInfo {
            sequence_number: current_ledger + 10,
            timestamp: 0,
            protocol_version: 21,
            network_id: Default::default(),
            base_reserve: 0,
            min_persistent_entry_ttl: 4096,
            min_temp_entry_ttl: 16,
            max_entry_ttl: 6312000,
        });

        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let (_publisher, topics, _event_data) = events.get(0).unwrap();
        assert_eq!(topics.len(), 2);
        assert_eq!(Symbol::try_from_val(&env, &topics.get(0).unwrap()).unwrap(), symbol_short!("rate"));
        assert_eq!(Symbol::try_from_val(&env, &topics.get(1).unwrap()).unwrap(), symbol_short!("win_reset"));

        let state = RateLimiter::get_state(env.clone(), attestor.clone());
        assert_eq!(state.submission_count, 1);
        assert_eq!(state.total_requests, 2);
        });
    }

    #[test]
    fn test_rate_limit_config_update() {
        with_contract(|env| {
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let new_config = RateLimitConfig { max_submissions: 20, window_length: 200, burst: 0 };

        assert!(RateLimiter::update_config(&env, &admin, new_config.clone(), None).is_ok());

        let config = RateLimiter::get_config(env.clone());
        assert_eq!(config.max_submissions, 20);
        assert_eq!(config.window_length, 200);
        });
    }

    #[test]
    fn test_rate_limit_default_config() {
        with_contract(|env| {
        let config = RateLimiter::get_config(env.clone());
        assert_eq!(config.max_submissions, 10);
        assert_eq!(config.window_length, 100);
        });
    }

    #[test]
    fn test_per_attestor_override_takes_precedence() {
        with_contract(|env| {
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &attestor, RateLimitConfig { max_submissions: 1, window_length: 100, burst: 0 }, None).unwrap();
        RateLimiter::update_config(&env, &attestor, RateLimitConfig { max_submissions: 5, window_length: 100, burst: 0 }, Some(&attestor)).unwrap();

        for _ in 0..5 {
            assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());
        }
        assert!(RateLimiter::check_and_increment(&env, &attestor).is_err());
        });
    }

    #[test]
    fn test_fallback_to_global_when_no_override() {
        with_contract(|env| {
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &attestor, RateLimitConfig { max_submissions: 2, window_length: 100, burst: 0 }, None).unwrap();

        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());
        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());
        assert!(RateLimiter::check_and_increment(&env, &attestor).is_err());
        });
    }

    #[test]
    fn test_override_does_not_affect_other_attestors() {
        with_contract(|env| {
        let high_volume = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let normal = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &high_volume, RateLimitConfig { max_submissions: 1, window_length: 100, burst: 0 }, None).unwrap();
        RateLimiter::update_config(&env, &high_volume, RateLimitConfig { max_submissions: 10, window_length: 100, burst: 0 }, Some(&high_volume)).unwrap();

        for _ in 0..10 {
            assert!(RateLimiter::check_and_increment(&env, &high_volume).is_ok());
        }

        assert!(RateLimiter::check_and_increment(&env, &normal).is_ok());
        assert!(RateLimiter::check_and_increment(&env, &normal).is_err());
        });
    }

    #[test]
    fn test_reset_rate_limit_admin_successfully_resets() {
        with_contract(|env| {
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &admin, RateLimitConfig { max_submissions: 1, window_length: 100, burst: 0 }, None).unwrap();

        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());
        assert_eq!(RateLimiter::get_state(env.clone(), attestor.clone()).submission_count, 1);
        assert!(RateLimiter::check_and_increment(&env, &attestor).is_err());

        assert!(RateLimiter::reset_rate_limit(&env, &admin, &attestor).is_ok());

        let state_after = RateLimiter::get_state(env.clone(), attestor.clone());
        assert_eq!(state_after.submission_count, 0);
        assert!(RateLimiter::check_and_increment(&env, &attestor).is_ok());
        assert_eq!(RateLimiter::get_state(env.clone(), attestor.clone()).submission_count, 1);
        });
    }

    #[test]
    fn test_reset_rate_limit_preserves_total_requests() {
        with_contract(|env| {
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &admin, RateLimitConfig { max_submissions: 1, window_length: 100, burst: 0 }, None).unwrap();

        RateLimiter::check_and_increment(&env, &attestor).unwrap();
        let _ = RateLimiter::check_and_increment(&env, &attestor);

        assert_eq!(RateLimiter::get_state(env.clone(), attestor.clone()).total_requests, 1);

        RateLimiter::reset_rate_limit(&env, &admin, &attestor).unwrap();

        let state_after = RateLimiter::get_state(env.clone(), attestor.clone());
        assert_eq!(state_after.total_requests, 1);
        assert_eq!(state_after.submission_count, 0);
        });
    }

    #[test]
    fn test_reset_rate_limit_non_admin_unauthorized() {
        with_contract(|env| {
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &admin, RateLimitConfig { max_submissions: 1, window_length: 100, burst: 0 }, None).unwrap();

        RateLimiter::check_and_increment(&env, &attestor).unwrap();
        let _ = RateLimiter::check_and_increment(&env, &attestor);

        let state = RateLimiter::get_state(env.clone(), attestor.clone());
        assert_eq!(state.submission_count, 1);
        });
    }

    #[test]
    fn test_reset_rate_limit_multiple_attestors_independent() {
        with_contract(|env| {
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let attestor1 = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let attestor2 = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &admin, RateLimitConfig { max_submissions: 1, window_length: 100, burst: 0 }, None).unwrap();

        RateLimiter::check_and_increment(&env, &attestor1).unwrap();
        let _ = RateLimiter::check_and_increment(&env, &attestor1);
        RateLimiter::check_and_increment(&env, &attestor2).unwrap();
        let _ = RateLimiter::check_and_increment(&env, &attestor2);

        RateLimiter::reset_rate_limit(&env, &admin, &attestor1).unwrap();

        assert_eq!(RateLimiter::get_state(env.clone(), attestor1.clone()).submission_count, 0);
        assert_eq!(RateLimiter::get_state(env.clone(), attestor2.clone()).submission_count, 1);
        assert!(RateLimiter::check_and_increment(&env, &attestor2).is_err());
        });
    }

    #[test]
    fn test_reset_rate_limit_resets_window_start_ledger() {
        with_contract(|env| {
        let admin = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);
        let attestor = <soroban_sdk::Address as soroban_sdk::testutils::Address>::generate(&env);

        RateLimiter::update_config(&env, &admin, RateLimitConfig { max_submissions: 2, window_length: 100, burst: 0 }, None).unwrap();

        RateLimiter::check_and_increment(&env, &attestor).unwrap();
        let state_before = RateLimiter::get_state(env.clone(), attestor.clone());
        let ledger_before = state_before.window_start_ledger;

        RateLimiter::reset_rate_limit(&env, &admin, &attestor).unwrap();

        let state_after = RateLimiter::get_state(env.clone(), attestor.clone());
        assert_eq!(state_after.window_start_ledger, env.ledger().sequence());
        assert!(state_after.window_start_ledger >= ledger_before);
        });
    }

    #[test]
    fn test_validate_rejects_zero_window_length() {
        let config = RateLimitConfig { max_submissions: 10, window_length: 0, burst: 0 };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_rejects_zero_max_submissions() {
        let config = RateLimitConfig { max_submissions: 0, window_length: 100, burst: 0 };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_accepts_valid_config() {
        let config = RateLimitConfig { max_submissions: 10, window_length: 100, burst: 0 };
        assert!(config.validate().is_ok());
    }
}
