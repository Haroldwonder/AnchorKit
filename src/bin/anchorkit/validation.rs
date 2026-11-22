//! Input validators shared by several subcommands.

use regex::Regex;

/// Returns `true` if `addr` is a syntactically valid Stellar public key:
/// prefix 'G', exactly 56 characters, remaining 55 chars from the RFC4648
/// base32 strkey alphabet (A-Z, 2-7).  Digits 0, 1, 8, 9 and any lowercase
/// letter are not part of that alphabet and will cause this to return `false`.
pub fn is_valid_g_address(addr: &str) -> bool {
    addr.starts_with('G')
        && addr.len() == 56
        && addr[1..].chars().all(|c| matches!(c, 'A'..='Z' | '2'..='7'))
}

/// Validate a Stellar public key (admin / contract-id format).
pub fn validate_stellar_address(s: &str) -> Result<String, String> {
    // Stellar public keys are RFC4648 base32 strkeys: prefix 'G' + 55 chars from A-Z2-7
    let re = Regex::new(r"^G[A-Z2-7]{55}$").unwrap();
    if re.is_match(s) {
        Ok(s.to_string())
    } else {
        Err(format!(
            "Invalid Stellar public key — must start with 'G' followed by 55 base32 characters (A-Z, 2-7) (56 chars total), got '{}'",
            s
        ))
    }
}

/// Validate a Soroban contract ID.
/// Contract IDs on Stellar are the same format as Stellar account addresses:
/// a 56-character strkey starting with 'C'.
pub fn validate_contract_id(s: &str) -> Result<String, String> {
    let re = Regex::new(r"^C[A-Z2-7]{55}$").unwrap();
    if re.is_match(s) {
        Ok(s.to_string())
    } else {
        Err(format!(
            "Invalid contract ID — must start with 'C' and be exactly 56 uppercase base-32 characters, got '{}'",
            s
        ))
    }
}
