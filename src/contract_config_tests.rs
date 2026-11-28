use super::*;
use crate::contract::MAX_ATTESTORS;
use crate::rate_limiter::RateLimitConfig;
use crate::storage::{
    key_clock_skew, key_health_threshold, key_replay_window, StorageKey,
};
use soroban_sdk::Env;

#[test]
fn get_config_returns_defaults() {
    let env = Env::default();
    let config = AnchorKitContract::get_config(env);
    assert_eq!(config.max_page_size, 50);
    assert_eq!(config.replay_window_seconds, 300);
    assert_eq!(config.clock_skew_seconds, 60);
    assert_eq!(config.health_failure_threshold, 0);
    assert_eq!(
        config.rate_limit_config,
        RateLimitConfig {
            max_submissions: 10,
            window_length: 100,
            burst: 0,
        }
    );
    assert_eq!(config.audit_log_max_size, 0);
    assert_eq!(config.attestor_cap, MAX_ATTESTORS);
    assert!(!config.paused);
}

#[test]
fn get_config_reflects_admin_settings() {
    let env = Env::default();
    let inst = env.storage().instance();
    inst.set(&StorageKey::MaxPageSize, &25u32);
    inst.set(&key_replay_window(&env), &120u64);
    inst.set(&key_clock_skew(&env), &15u64);
    inst.set(&key_health_threshold(&env), &70u32);
    inst.set(&StorageKey::AuditLogMaxSize, &500u64);
    inst.set(&StorageKey::IsPaused, &true);

    let config = AnchorKitContract::get_config(env);
    assert_eq!(config.max_page_size, 25);
    assert_eq!(config.replay_window_seconds, 120);
    assert_eq!(config.clock_skew_seconds, 15);
    assert_eq!(config.health_failure_threshold, 70);
    assert_eq!(config.audit_log_max_size, 500);
    assert!(config.paused);
}
