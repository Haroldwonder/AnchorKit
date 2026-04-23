//! Domain validation utility for anchor domain input
//!
//! Validates anchor domain URLs before making requests to ensure:
//! - Proper URL format
//! - HTTPS-only connections
//! - Rejection of malformed domains


extern crate alloc;
use alloc::vec::Vec;

use crate::errors::AnchorKitError;

/// Validates an anchor domain URL
///
/// # Requirements
/// - Must be a valid URL format
/// - Must use HTTPS protocol only
/// - Must have a valid domain structure
/// - Must not contain malformed components
///
/// # Arguments
/// * `domain` - The domain URL to validate
///
/// # Returns
/// * `Ok(())` if the domain is valid
/// * `Err(AnchorKitError)` if validation fails
///
/// # Examples
/// ```ignore
/// use anchorkit::domain_validator::validate_anchor_domain;
///
/// assert!(validate_anchor_domain("https://example.com").is_ok());
/// assert!(validate_anchor_domain("http://example.com").is_err());
/// assert!(validate_anchor_domain("not-a-url").is_err());
/// ```
pub fn validate_anchor_domain(domain: &str) -> Result<(), AnchorKitError> {
    // Check for empty or whitespace-only input
    if domain.is_empty() || domain.trim().is_empty() {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Check minimum length for valid HTTPS URL
    if domain.len() < 10 {
        // "https://a.b" is minimum valid
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Check maximum reasonable length
    if domain.len() > 2048 {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Ensure HTTPS protocol
    if !domain.starts_with("https://") {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Extract domain part after protocol
    let domain_part = &domain[8..]; // Skip "https://"

    // Check for empty domain after protocol
    if domain_part.is_empty() {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Split by '/' to get the host part, but also handle query params
    let host_with_query = match domain_part.split('/').next() {
        Some(h) if !h.is_empty() => h,
        _ => return Err(AnchorKitError::invalid_endpoint_format()),
    };
    
    // Remove query parameters and fragments from host
    let host = host_with_query
        .split('?').next().unwrap_or(host_with_query)
        .split('#').next().unwrap_or(host_with_query);

    // Validate host structure
    validate_host(host)?;

    // Check for invalid characters in the full URL
    validate_url_characters(domain)?;

    Ok(())
}

/// Validates the host portion of a URL
fn validate_host(host: &str) -> Result<(), AnchorKitError> {
    // Check for empty host
    if host.is_empty() {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Check for spaces in host
    if host.contains(' ') {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Check for port specification (optional)
    let domain_without_port = if let Some(colon_pos) = host.rfind(':') {
        // Validate port number
        let port_str = &host[colon_pos + 1..];
        if port_str.is_empty() {
            return Err(AnchorKitError::invalid_endpoint_format());
        }
        
        // Check if port is numeric
        for c in port_str.chars() {
            if !c.is_ascii_digit() {
                return Err(AnchorKitError::invalid_endpoint_format());
            }
        }
        
        // Validate port range (1-65535)
        if let Ok(port) = port_str.parse::<u32>() {
            if port == 0 || port > 65535 {
                return Err(AnchorKitError::invalid_endpoint_format());
            }
        } else {
            return Err(AnchorKitError::invalid_endpoint_format());
        }
        
        &host[..colon_pos]
    } else {
        host
    };

    // Check for valid domain structure
    if domain_without_port.is_empty() {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // #267: Reject loopback hostnames explicitly before any other checks.
    // "localhost" is caught by the single-label check below, but
    // "localhost.localdomain" and similar variants must also be rejected.
    {
        let d = domain_without_port;
        let is_localhost = d.eq_ignore_ascii_case("localhost")
            || d.len() > 9 && d[..9].eq_ignore_ascii_case("localhost") && d.as_bytes()[9] == b'.'
            || d.len() > 9 && d[d.len()-9..].eq_ignore_ascii_case("localhost") && d.as_bytes()[d.len()-10] == b'.';
        if is_localhost {
            return Err(AnchorKitError::invalid_endpoint_format());
        }
    }

    // Must contain at least one dot for valid domain (rejects single-label hostnames
    // like "anchor", "localhost", "intranet" which have no TLD — issue #275)
    if !domain_without_port.contains('.') {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Must have at least two non-empty labels (e.g. "anchor.example"), rejecting
    // single-label domains even when a trailing dot is somehow present
    {
        let labels: Vec<&str> = domain_without_port.split('.').collect();
        let non_empty_labels = labels.iter().filter(|l| !l.is_empty()).count();
        if non_empty_labels < 2 {
            return Err(AnchorKitError::invalid_endpoint_format());
        }
    }

    // Check for consecutive dots
    if domain_without_port.contains("..") {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Check for leading or trailing dots
    if domain_without_port.starts_with('.') || domain_without_port.ends_with('.') {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    // Validate each label in the domain
    let labels: Vec<&str> = domain_without_port.split('.').collect();

    // Reject pure IPv4 addresses (all labels are numeric)
    if labels.iter().all(|l| l.chars().all(|c| c.is_ascii_digit())) {
        return Err(AnchorKitError::invalid_endpoint_format());
    }

    for label in &labels {
        if label.is_empty() || label.len() > 63 {
            return Err(AnchorKitError::invalid_endpoint_format());
        }

        let first_char = label.chars().next().unwrap();
        let last_char = label.chars().last().unwrap();

        if !first_char.is_ascii_alphanumeric() || !last_char.is_ascii_alphanumeric() {
            return Err(AnchorKitError::invalid_endpoint_format());
        }

        // #268: ASCII alphanumeric + hyphen only.
        // Unicode labels are rejected here; callers must normalize to Punycode
        // (xn-- prefix) before passing to this function.  We additionally
        // reject any label that starts with "xn--" to prevent homograph
        // attacks via crafted Punycode that encodes visually-similar characters.
        if label.len() >= 4 && label[..4].eq_ignore_ascii_case("xn--") {
            return Err(AnchorKitError::invalid_endpoint_format());
        }

        for c in label.chars() {
            if !c.is_ascii_alphanumeric() && c != '-' {
                return Err(AnchorKitError::invalid_endpoint_format());
            }
        }
    }

    Ok(())
}

/// Validates URL characters
fn validate_url_characters(url: &str) -> Result<(), AnchorKitError> {
    // #269: Reject percent-encoded null byte before iterating chars so that
    // the encoded form (%00) is caught even though '%', '0' are individually valid.
    if url.contains("%00") {
        return Err(AnchorKitError::invalid_endpoint_format());
    }
    for c in url.chars() {
        if c.is_control() || matches!(c, '<' | '>' | '{' | '}' | '|' | '\\') {
            return Err(AnchorKitError::invalid_endpoint_format());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn test_valid_domains() {
        // Basic valid domains
        assert!(validate_anchor_domain("https://example.com").is_ok());
        assert!(validate_anchor_domain("https://api.example.com").is_ok());
        assert!(validate_anchor_domain("https://sub.domain.example.com").is_ok());
        
        // With paths
        assert!(validate_anchor_domain("https://example.com/path").is_ok());
        assert!(validate_anchor_domain("https://example.com/path/to/resource").is_ok());
        
        // With ports
        assert!(validate_anchor_domain("https://example.com:8080").is_ok());
        assert!(validate_anchor_domain("https://example.com:443").is_ok());
        
        // With query parameters
        assert!(validate_anchor_domain("https://example.com?param=value").is_ok());
        assert!(validate_anchor_domain("https://example.com/path?param=value").is_ok());
        
        // With hyphens in domain
        assert!(validate_anchor_domain("https://my-domain.com").is_ok());
        assert!(validate_anchor_domain("https://api-v2.example.com").is_ok());
    }

    #[test]
    fn test_https_only() {
        // HTTP should be rejected
        assert!(validate_anchor_domain("http://example.com").is_err());
        assert!(validate_anchor_domain("http://secure.example.com").is_err());
        
        // Other protocols should be rejected
        assert!(validate_anchor_domain("ftp://example.com").is_err());
        assert!(validate_anchor_domain("ws://example.com").is_err());
        assert!(validate_anchor_domain("wss://example.com").is_err());
    }

    #[test]
    fn test_malformed_domains() {
        // Empty or whitespace
        assert!(validate_anchor_domain("").is_err());
        assert!(validate_anchor_domain("   ").is_err());
        
        // Missing protocol
        assert!(validate_anchor_domain("example.com").is_err());
        assert!(validate_anchor_domain("www.example.com").is_err());
        
        // Protocol only
        assert!(validate_anchor_domain("https://").is_err());
        
        // Invalid domain structure
        assert!(validate_anchor_domain("https://.example.com").is_err());
        assert!(validate_anchor_domain("https://example.com.").is_err());
        assert!(validate_anchor_domain("https://example..com").is_err());
        
        // Issue #275: single-label domains (no TLD) must be rejected — they are
        // internal hostnames and not valid anchor endpoints
        assert!(validate_anchor_domain("https://localhost").is_err());
        assert!(validate_anchor_domain("https://example").is_err());
        assert!(validate_anchor_domain("https://anchor").is_err());
        assert!(validate_anchor_domain("https://anchor/sep6").is_err());
        assert!(validate_anchor_domain("https://intranet/path").is_err());
        
        // Spaces in domain
        assert!(validate_anchor_domain("https://example .com").is_err());
        assert!(validate_anchor_domain("https://exam ple.com").is_err());
        
        // Invalid characters
        assert!(validate_anchor_domain("https://example$.com").is_err());
        assert!(validate_anchor_domain("https://example@.com").is_err());
        
        // Too short
        assert!(validate_anchor_domain("https://a").is_err());
        assert!(validate_anchor_domain("https://a.").is_err());
    }

    #[test]
    fn test_port_validation() {
        // Valid ports
        assert!(validate_anchor_domain("https://example.com:1").is_ok());
        assert!(validate_anchor_domain("https://example.com:80").is_ok());
        assert!(validate_anchor_domain("https://example.com:443").is_ok());
        assert!(validate_anchor_domain("https://example.com:8080").is_ok());
        assert!(validate_anchor_domain("https://example.com:65535").is_ok());
        
        // Invalid ports
        assert!(validate_anchor_domain("https://example.com:0").is_err());
        assert!(validate_anchor_domain("https://example.com:65536").is_err());
        assert!(validate_anchor_domain("https://example.com:99999").is_err());
        assert!(validate_anchor_domain("https://example.com:").is_err());
        assert!(validate_anchor_domain("https://example.com:abc").is_err());
    }

    #[test]
    fn test_length_limits() {
        // Too long
        let long_domain = format!("https://{}.com", "a".repeat(2048));
        assert!(validate_anchor_domain(&long_domain).is_err());
        
        // Maximum acceptable length
        let max_domain = format!("https://{}.com", "a".repeat(2000));
        assert!(validate_anchor_domain(&max_domain).is_ok());
    }

    #[test]
    fn test_control_characters() {
        // Control characters should be rejected
        assert!(validate_anchor_domain("https://example.com\n").is_err());
        assert!(validate_anchor_domain("https://example.com\r").is_err());
        assert!(validate_anchor_domain("https://example.com\t").is_err());
        assert!(validate_anchor_domain("https://\0example.com").is_err());
        // #269: percent-encoded null byte must also be rejected
        assert!(validate_anchor_domain("https://example.com/%00").is_err());
        assert!(validate_anchor_domain("https://example.com/path%00/resource").is_err());
        assert!(validate_anchor_domain("https://example.com?q=%00").is_err());
    }

    #[test]
    fn test_double_slashes() {
        // Double slashes in paths are technically allowed in URLs
        // but may indicate a mistake - for now we allow them
        assert!(validate_anchor_domain("https://example.com//path").is_ok());
        assert!(validate_anchor_domain("https://example.com/path//resource").is_ok());
    }

    #[test]
    fn test_edge_cases() {
        // Minimum valid domain
        assert!(validate_anchor_domain("https://a.b").is_ok());
        
        // Multiple subdomains
        assert!(validate_anchor_domain("https://a.b.c.d.example.com").is_ok());
        
        // Numbers in domain
        assert!(validate_anchor_domain("https://api2.example.com").is_ok());
        assert!(validate_anchor_domain("https://123.example.com").is_ok());
        
        // Hyphens at various positions (but not at start/end of label)
        assert!(validate_anchor_domain("https://my-api.example.com").is_ok());
        assert!(validate_anchor_domain("https://-example.com").is_err());
        assert!(validate_anchor_domain("https://example-.com").is_err());
    }

    #[test]
    fn test_unicode_idn_domains() {
        // Unicode/IDN domains should be rejected (not supported)
        assert!(validate_anchor_domain("https://münchen.de").is_err());
        assert!(validate_anchor_domain("https://例え.jp").is_err());
        assert!(validate_anchor_domain("https://россия.рф").is_err());
        assert!(validate_anchor_domain("https://example.测试").is_err());
    }

    // #268: Punycode-encoded labels (xn--) must be rejected to prevent homograph attacks.
    #[test]
    fn test_punycode_homograph_rejected() {
        // xn--e1afmapc.com is the Punycode for россия.com (Cyrillic lookalike)
        assert!(validate_anchor_domain("https://xn--e1afmapc.com").is_err());
        // xn--bcher-kva.example is Punycode for bücher.example
        assert!(validate_anchor_domain("https://xn--bcher-kva.example.com").is_err());
        // Mixed: one normal label, one xn-- label
        assert!(validate_anchor_domain("https://api.xn--e1afmapc.com").is_err());
    }

    #[test]
    fn test_ip_address_inputs() {
        // IPv4 addresses should be rejected (not valid domain format)
        assert!(validate_anchor_domain("https://192.168.1.1").is_err());
        assert!(validate_anchor_domain("https://10.0.0.1").is_err());
        assert!(validate_anchor_domain("https://127.0.0.1").is_err());
        
        // IPv6 addresses should be rejected
        assert!(validate_anchor_domain("https://[::1]").is_err());
        assert!(validate_anchor_domain("https://[2001:db8::1]").is_err());
    }

    // #267: Loopback hostnames must be rejected.
    #[test]
    fn test_loopback_addresses_rejected() {
        // Plain localhost (single-label, already caught, but explicit)
        assert!(validate_anchor_domain("https://localhost").is_err());
        // localhost with path
        assert!(validate_anchor_domain("https://localhost/sep6").is_err());
        // localhost with port
        assert!(validate_anchor_domain("https://localhost:8080").is_err());
        // localhost.localdomain — two labels, would pass single-label check without #267 fix
        assert!(validate_anchor_domain("https://localhost.localdomain").is_err());
        // subdomain of localhost
        assert!(validate_anchor_domain("https://api.localhost").is_err());
        // 127.0.0.1 — already rejected as pure IPv4
        assert!(validate_anchor_domain("https://127.0.0.1").is_err());
        // ::1 — already rejected via IPv6 bracket syntax
        assert!(validate_anchor_domain("https://[::1]").is_err());
    }

    #[test]
    fn test_trailing_slashes() {
        // Trailing slashes should be allowed
        assert!(validate_anchor_domain("https://example.com/").is_ok());
        assert!(validate_anchor_domain("https://example.com/path/").is_ok());
        assert!(validate_anchor_domain("https://example.com/path/to/resource/").is_ok());
        
        // Multiple trailing slashes
        assert!(validate_anchor_domain("https://example.com//").is_ok());
    }

    #[test]
    fn test_length_boundaries() {
        // "https://" (8) + label (2036) + ".com" (4) = 2048 exactly (should pass)
        let max_valid_domain = format!("https://{}.com", "a".repeat(2036));
        assert!(validate_anchor_domain(&max_valid_domain).is_ok());

        // One char over 2048 (should fail)
        let too_long_domain = format!("https://{}.com", "a".repeat(2037));
        assert!(validate_anchor_domain(&too_long_domain).is_err());

        // Very short valid domains
        assert!(validate_anchor_domain("https://a.b").is_ok());
        assert!(validate_anchor_domain("https://ab.cd").is_ok());

        // DNS limits: 63 per label, 253 for full domain
        
        // 63-char label (valid)
        let label_63 = "a".repeat(63);
        let domain_63 = format!("https://{}.com", label_63);
        assert!(validate_anchor_domain(&domain_63).is_ok());

        // 64-char label (invalid)
        let label_64 = "a".repeat(64);
        let domain_64 = format!("https://{}.com", label_64);
        assert!(validate_anchor_domain(&domain_64).is_err());

        // 253-char domain (valid)
        let domain_part_253 = format!("{}.com", "a".repeat(249)); // 249 + 1 (.) + 3 (com) = 253
        let full_url_253 = format!("https://{}", domain_part_253);
        assert!(validate_anchor_domain(&full_url_253).is_ok());

        // 254-char domain (invalid)
        let domain_part_254 = format!("{}.com", "a".repeat(250));
        let full_url_254 = format!("https://{}", domain_part_254);
        assert!(validate_anchor_domain(&full_url_254).is_err());
    }

    #[test]
    fn test_query_parameters_and_fragments() {
        // Query parameters should be allowed
        assert!(validate_anchor_domain("https://example.com?param=value").is_ok());
        assert!(validate_anchor_domain("https://example.com?param1=value1&param2=value2").is_ok());
        
        // Fragments should be allowed
        assert!(validate_anchor_domain("https://example.com#section").is_ok());
        assert!(validate_anchor_domain("https://example.com/path#section").is_ok());
        
        // Both query and fragment
        assert!(validate_anchor_domain("https://example.com?param=value#section").is_ok());
    }

    #[test]
    fn test_special_characters_in_path() {
        // Valid special characters in paths
        assert!(validate_anchor_domain("https://example.com/path-with-dash").is_ok());
        assert!(validate_anchor_domain("https://example.com/path_with_underscore").is_ok());
        assert!(validate_anchor_domain("https://example.com/path.with.dot").is_ok());
        assert!(validate_anchor_domain("https://example.com/path~tilde").is_ok());
        assert!(validate_anchor_domain("https://example.com/path%20encoded").is_ok());
        
        // Invalid characters in paths
        assert!(validate_anchor_domain("https://example.com/path<invalid>").is_err());
        assert!(validate_anchor_domain("https://example.com/path{invalid}").is_err());
        assert!(validate_anchor_domain("https://example.com/path|pipe").is_err());
        assert!(validate_anchor_domain("https://example.com/path\\backslash").is_err());
    }

    #[test]
    fn test_port_edge_cases() {
        // Valid port ranges
        assert!(validate_anchor_domain("https://example.com:1").is_ok());
        assert!(validate_anchor_domain("https://example.com:80").is_ok());
        assert!(validate_anchor_domain("https://example.com:443").is_ok());
        assert!(validate_anchor_domain("https://example.com:8080").is_ok());
        assert!(validate_anchor_domain("https://example.com:65535").is_ok());
        
        // Invalid port ranges
        assert!(validate_anchor_domain("https://example.com:0").is_err());
        assert!(validate_anchor_domain("https://example.com:65536").is_err());
        assert!(validate_anchor_domain("https://example.com:99999").is_err());
        
        // Port with path
        assert!(validate_anchor_domain("https://example.com:8080/path").is_ok());
        assert!(validate_anchor_domain("https://example.com:8080/path?query=value").is_ok());
    }

    #[test]
    fn test_whitespace_variations() {
        // Leading/trailing whitespace should be rejected
        assert!(validate_anchor_domain(" https://example.com").is_err());
        assert!(validate_anchor_domain("https://example.com ").is_err());
        assert!(validate_anchor_domain("  https://example.com  ").is_err());
        
        // Internal whitespace should be rejected
        assert!(validate_anchor_domain("https://example .com").is_err());
        assert!(validate_anchor_domain("https://exam ple.com").is_err());
    }

    #[test]
    fn test_protocol_variations() {
        // Only HTTPS should be allowed
        assert!(validate_anchor_domain("https://example.com").is_ok());
        
        // All other protocols should be rejected
        assert!(validate_anchor_domain("http://example.com").is_err());
        assert!(validate_anchor_domain("ftp://example.com").is_err());
        assert!(validate_anchor_domain("ws://example.com").is_err());
        assert!(validate_anchor_domain("wss://example.com").is_err());
        assert!(validate_anchor_domain("file://example.com").is_err());
        assert!(validate_anchor_domain("mailto:example@example.com").is_err());
    }

    #[test]
    fn test_domain_label_edge_cases() {
        // Valid labels
        assert!(validate_anchor_domain("https://a-b-c.example.com").is_ok());
        assert!(validate_anchor_domain("https://123-456.example.com").is_ok());
        assert!(validate_anchor_domain("https://a1b2c3.example.com").is_ok());
        
        // Invalid labels
        assert!(validate_anchor_domain("https://-abc.example.com").is_err());
        assert!(validate_anchor_domain("https://abc-.example.com").is_err());
        assert!(validate_anchor_domain("https://a--b.example.com").is_ok()); // Double hyphens allowed in middle
        assert!(validate_anchor_domain("https://.example.com").is_err());
        assert!(validate_anchor_domain("https://example..com").is_err());
    }
}
