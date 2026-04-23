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
        if label.is_empty() {
            return Err(AnchorKitError::invalid_endpoint_format());
        }

        let first_char = label.chars().next().unwrap();
        let last_char = label.chars().last().unwrap();

        if !first_char.is_ascii_alphanumeric() || !last_char.is_ascii_alphanumeric() {
            return Err(AnchorKitError::invalid_endpoint_format());
        }

        // ASCII alphanumeric + hyphen only; rejects unicode
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
