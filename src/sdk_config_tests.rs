extern crate alloc;

#[cfg(test)]
mod tests {
    use crate::sdk_config::{Network, SdkConfig, MAX_ANCHOR_LEN};
    use soroban_sdk::{Env, String};

    #[test]
    fn test_valid_testnet_config() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Testnet,
            30,
            3,
            String::from_str(&env, "https://testanchor.stellar.org"),
        );
        assert!(config.is_ok());
    }

    #[test]
    fn test_valid_mainnet_config() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Mainnet,
            60,
            5,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_ok());
    }

    #[test]
    fn test_timeout_too_low() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Testnet,
            2,
            3,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_err());
    }

    #[test]
    fn test_timeout_too_high() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Testnet,
            400,
            3,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_err());
    }

    #[test]
    fn test_retry_too_high() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Testnet,
            30,
            15,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_err());
    }

    #[test]
    fn test_anchor_too_short() {
        let env = Env::default();
        let config = SdkConfig::new(Network::Testnet, 30, 3, String::from_str(&env, "ab"));
        assert!(config.is_err());
    }

    #[test]
    fn test_min_retry_zero() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Testnet,
            30,
            0,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_ok());
    }

    #[test]
    fn test_max_retry_ten() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Mainnet,
            30,
            10,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_ok());
    }

    #[test]
    fn test_min_timeout_five() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Testnet,
            5,
            3,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_ok());
    }

    #[test]
    fn test_max_timeout_300() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Mainnet,
            300,
            3,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_ok());
    }

    #[test]
    fn test_default_timeout_is_10_seconds() {
        use crate::sdk_config::DEFAULT_TIMEOUT_SECONDS;
        assert_eq!(DEFAULT_TIMEOUT_SECONDS, 10);
    }

    #[test]
    fn test_with_defaults_uses_default_timeout() {
        let env = Env::default();
        let config = SdkConfig::with_defaults(
            Network::Testnet,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.timeout_seconds, 10);
        assert_eq!(config.retry_attempts, 3);
    }

    #[test]
    fn test_custom_timeout_overrides_default() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Testnet,
            25,
            3,
            String::from_str(&env, "https://anchor.stellar.org"),
        );
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.timeout_seconds, 25);
    }

    // --- Anchor length boundary tests ---

    /// A default_anchor of exactly MAX_ANCHOR_LEN (256) bytes must be accepted
    /// when it is also a valid HTTPS URL.  We construct a URL of exactly 256
    /// bytes: "https://anchor.example.com/" (27 bytes) + path padding to 256.
    #[test]
    fn test_anchor_exactly_max_len_accepted() {
        let env = Env::default();
        let prefix = "https://anchor.example.com/";
        let padding_len = MAX_ANCHOR_LEN as usize - prefix.len();
        let anchor: alloc::string::String = prefix.to_owned() + &"a".repeat(padding_len);
        assert_eq!(anchor.len(), MAX_ANCHOR_LEN as usize);
        let config = SdkConfig::new(
            Network::Testnet,
            30,
            3,
            String::from_str(&env, &anchor),
        );
        assert!(config.is_ok(), "anchor of exactly {MAX_ANCHOR_LEN} bytes must be accepted");
    }

    /// A default_anchor one byte over MAX_ANCHOR_LEN (257 bytes) must be
    /// rejected by the length guard before domain validation even runs.
    #[test]
    fn test_anchor_too_long_rejected() {
        let env = Env::default();
        let prefix = "https://anchor.example.com/";
        let padding_len = MAX_ANCHOR_LEN as usize - prefix.len() + 1; // 257 total
        let anchor: alloc::string::String = prefix.to_owned() + &"a".repeat(padding_len);
        assert_eq!(anchor.len(), MAX_ANCHOR_LEN as usize + 1);
        let config = SdkConfig::new(
            Network::Testnet,
            30,
            3,
            String::from_str(&env, &anchor),
        );
        assert!(config.is_err(), "anchor longer than {MAX_ANCHOR_LEN} bytes must be rejected");
    }

    // --- Anchor format validation tests ---

    /// A value that passes the length check but has no HTTPS scheme must be
    /// rejected by domain-format validation.
    #[test]
    fn test_anchor_no_https_scheme_rejected() {
        let env = Env::default();
        // "anchor.stellar.org" is 18 bytes — passes MIN/MAX but is not a URL
        let config = SdkConfig::new(
            Network::Testnet,
            30,
            3,
            String::from_str(&env, "anchor.stellar.org"),
        );
        assert!(config.is_err(), "anchor without https:// scheme must be rejected");
    }

    /// A value with HTTP (not HTTPS) must be rejected.
    #[test]
    fn test_anchor_http_scheme_rejected() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Testnet,
            30,
            3,
            String::from_str(&env, "http://anchor.stellar.org"),
        );
        assert!(config.is_err(), "http:// anchor must be rejected; only https:// is allowed");
    }

    /// Arbitrary garbage that meets the length bounds must be rejected.
    #[test]
    fn test_anchor_garbage_string_rejected() {
        let env = Env::default();
        let config = SdkConfig::new(
            Network::Testnet,
            30,
            3,
            String::from_str(&env, "not a domain!!"),
        );
        assert!(config.is_err(), "garbage anchor string must be rejected by format validation");
    }
}
