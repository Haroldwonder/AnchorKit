//! `anchorkit validate` — validate JSON/TOML config files.

use regex::Regex;
use crate::exit_code;

pub fn run_validate(path: &str) {
    let p = std::path::Path::new(path);
    if p.is_dir() {
        let read_dir = match std::fs::read_dir(p) {
            Ok(rd) => rd,
            Err(e) => {
                eprintln!("error: cannot read directory '{}': {}", path, e);
                std::process::exit(exit_code::CONFIG);
            }
        };
        let mut entries: Vec<_> = read_dir
            .filter_map(|e| e.ok())
            .filter(|e| {
                matches!(
                    e.path().extension().and_then(|s| s.to_str()),
                    Some("json") | Some("toml")
                )
            })
            .collect();
        entries.sort_by_key(|e| e.path());
        if entries.is_empty() {
            println!("No .json or .toml files found in {}", path);
            return;
        }
        let mut all_ok = true;
        for entry in entries {
            all_ok &= validate_file(&entry.path());
        }
        if !all_ok {
            std::process::exit(exit_code::USAGE);
        }
    } else if !p.exists() {
        eprintln!("error: config path '{}' does not exist", path);
        std::process::exit(exit_code::CONFIG);
    } else if !validate_file(p) {
        std::process::exit(exit_code::USAGE);
    }
}

pub fn validate_file(path: &std::path::Path) -> bool {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            println!("✖ {}: cannot read file: {}", path.display(), e);
            return false;
        }
    };
    
    // First check syntax
    let config_value = match ext {
        "json" => {
            match serde_json::from_str::<serde_json::Value>(&content) {
                Ok(v) => v,
                Err(e) => {
                    println!("✖ {}: invalid JSON at line {}, column {}: {}", 
                             path.display(), e.line(), e.column(), e);
                    return false;
                }
            }
        }
        "toml" => {
            match toml::from_str::<toml::Value>(&content) {
                Ok(v) => serde_json::to_value(v).unwrap(),
                Err(e) => {
                    if let Some(span) = e.span() {
                        let line = content[..span.start].chars().filter(|&c| c == '\n').count() + 1;
                        println!("✖ {}: invalid TOML at line {}: {}", 
                                 path.display(), line, e.message());
                    } else {
                        println!("✖ {}: invalid TOML: {}", path.display(), e);
                    }
                    return false;
                }
            }
        }
        _ => {
            println!("✖ {}: unsupported format (expected .json or .toml)", path.display());
            return false;
        }
    };
    
    // Now validate schema and business rules
    validate_config_schema(path, &config_value)
}

fn validate_config_schema(path: &std::path::Path, config: &serde_json::Value) -> bool {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    // Validate required top-level fields
    let obj = match config.as_object() {
        Some(o) => o,
        None => {
            println!("✖ {}: root must be an object", path.display());
            return false;
        }
    };
    
    // Check required top-level fields
    for required_field in &["contract", "attestors", "sessions"] {
        if !obj.contains_key(*required_field) {
            errors.push(format!("field '{}' is missing", required_field));
        }
    }
    
    // Validate contract section
    if let Some(contract) = obj.get("contract") {
        validate_contract_section(contract, &mut errors, &mut warnings);
    }
    
    // Validate attestors section
    if let Some(attestors) = obj.get("attestors") {
        validate_attestors_section(attestors, &mut errors, &mut warnings);
    }
    
    // Validate sessions section
    if let Some(sessions) = obj.get("sessions") {
        validate_sessions_section(sessions, &mut errors, &mut warnings);
    }

    // Validate deployment section when present (optional — added after `anchorkit config init`)
    if let Some(deployment) = obj.get("deployment") {
        validate_deployment_section(deployment, &mut errors, &mut warnings);
    }

    // Print results
    if !warnings.is_empty() {
        for warning in &warnings {
            println!("⚠  {}: {}", path.display(), warning);
        }
    }
    
    if errors.is_empty() {
        println!("✔ {}: valid configuration", path.display());
        true
    } else {
        println!("✖ {}: invalid configuration", path.display());
        for error in &errors {
            println!("  • {}", error);
        }
        false
    }
}

fn validate_contract_section(contract: &serde_json::Value, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let obj = match contract.as_object() {
        Some(o) => o,
        None => {
            errors.push("field 'contract' must be an object".to_string());
            return;
        }
    };
    
    // Required fields
    for required in &["name", "version", "network"] {
        if !obj.contains_key(*required) {
            errors.push(format!("field 'contract.{}' is missing", required));
        }
    }
    
    // Validate name
    if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
        let name_regex = Regex::new(r"^[a-z0-9-]+$").unwrap();
        if !name_regex.is_match(name) {
            errors.push("field 'contract.name' must contain only lowercase letters, numbers, and hyphens".to_string());
        }
        if name.is_empty() || name.len() > 64 {
            errors.push(format!("field 'contract.name' length must be 1-64 characters, got {}", name.len()));
        }
    } else if obj.contains_key("name") {
        errors.push("field 'contract.name' must be a string".to_string());
    }
    
    // Validate version
    if let Some(version) = obj.get("version").and_then(|v| v.as_str()) {
        let version_regex = Regex::new(r"^\d+\.\d+\.\d+$").unwrap();
        if !version_regex.is_match(version) {
            errors.push("field 'contract.version' must follow semantic versioning (e.g., 1.0.0)".to_string());
        }
    } else if obj.contains_key("version") {
        errors.push("field 'contract.version' must be a string".to_string());
    }
    
    // Validate network
    if let Some(network) = obj.get("network").and_then(|v| v.as_str()) {
        let valid_networks = ["stellar-testnet", "stellar-mainnet", "stellar-futurenet"];
        if !valid_networks.contains(&network) {
            errors.push(format!("field 'contract.network' must be one of: {}", valid_networks.join(", ")));
        }
    } else if obj.contains_key("network") {
        errors.push("field 'contract.network' must be a string".to_string());
    }
    
    // Validate description (optional)
    if let Some(desc) = obj.get("description") {
        if let Some(desc_str) = desc.as_str() {
            if desc_str.len() > 256 {
                errors.push(format!("field 'contract.description' exceeds maximum length of 256 characters"));
            }
        } else {
            errors.push("field 'contract.description' must be a string".to_string());
        }
    }
    
    let _ = warnings; // Suppress unused warning
}

fn validate_attestors_section(attestors: &serde_json::Value, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let obj = match attestors.as_object() {
        Some(o) => o,
        None => {
            errors.push("field 'attestors' must be an object".to_string());
            return;
        }
    };
    
    // Required field: registry
    if !obj.contains_key("registry") {
        errors.push("field 'attestors.registry' is missing".to_string());
        return;
    }
    
    let registry = match obj.get("registry").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => {
            errors.push("field 'attestors.registry' must be an array".to_string());
            return;
        }
    };
    
    if registry.is_empty() {
        errors.push("field 'attestors.registry' must contain at least one attestor".to_string());
        return;
    }
    
    if registry.len() > 100 {
        errors.push(format!("field 'attestors.registry' exceeds maximum of 100 attestors, got {}", registry.len()));
    }
    
    // Track duplicates
    let mut names = Vec::new();
    let mut addresses = Vec::new();
    let mut has_enabled = false;
    
    for (idx, attestor) in registry.iter().enumerate() {
        let attestor_obj = match attestor.as_object() {
            Some(o) => o,
            None => {
                errors.push(format!("field 'attestors.registry[{}]' must be an object", idx));
                continue;
            }
        };
        
        let attestor_name = attestor_obj.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&format!("attestor-{}", idx));
        
        // Required fields
        for required in &["name", "address", "endpoint", "role", "enabled"] {
            if !attestor_obj.contains_key(*required) {
                errors.push(format!("field 'attestors.registry[{}].{}' is missing", idx, required));
            }
        }
        
        // Validate name
        if let Some(name) = attestor_obj.get("name").and_then(|v| v.as_str()) {
            let name_regex = Regex::new(r"^[a-z0-9-]+$").unwrap();
            if !name_regex.is_match(name) {
                errors.push(format!("field 'attestors.registry[{}].name' must contain only lowercase letters, numbers, and hyphens", idx));
            }
            if name.is_empty() || name.len() > 64 {
                errors.push(format!("field 'attestors.registry[{}].name' length must be 1-64 characters, got {}", idx, name.len()));
            }
            names.push(name.to_string());
        }
        
        // Validate address
        if let Some(address) = attestor_obj.get("address").and_then(|v| v.as_str()) {
            let addr_regex = Regex::new(r"^G[A-Z2-7]{55}$").unwrap();
            if !addr_regex.is_match(address) {
                errors.push(format!("field 'attestors.registry[{}].address' has invalid Stellar address format (must start with 'G' and be 56 characters)", idx));
            }
            let addr_len = address.len();
            if addr_len < 54 || addr_len > 56 {
                errors.push(format!("field 'attestors.registry[{}].address' length must be 54-56 characters, got {}", idx, addr_len));
            }
            addresses.push(address.to_string());
        }
        
        // Validate endpoint
        if let Some(endpoint) = attestor_obj.get("endpoint").and_then(|v| v.as_str()) {
            if let Some(error) = validate_endpoint_url(endpoint) {
                errors.push(format!("field 'attestors.registry[{}].endpoint' has invalid URL '{}' — {}", idx, endpoint, error));
            }
        }
        
        // Validate role
        if let Some(role) = attestor_obj.get("role").and_then(|v| v.as_str()) {
            let valid_roles = ["kyc-issuer", "transfer-verifier", "compliance-approver", "rate-provider", "attestor"];
            if !valid_roles.contains(&role) {
                errors.push(format!("field 'attestors.registry[{}].role' must be one of: {}", idx, valid_roles.join(", ")));
            }
        }
        
        // Check if enabled
        if let Some(enabled) = attestor_obj.get("enabled").and_then(|v| v.as_bool()) {
            if enabled {
                has_enabled = true;
            }
        }
        
        // Validate description (optional)
        if let Some(desc) = attestor_obj.get("description") {
            if let Some(desc_str) = desc.as_str() {
                if desc_str.len() > 256 {
                    errors.push(format!("field 'attestors.registry[{}].description' exceeds maximum length of 256 characters", idx));
                }
            }
        }
    }
    
    // Check for duplicates
    for name in &names {
        if names.iter().filter(|n| *n == name).count() > 1 {
            errors.push(format!("duplicate attestor name found: '{}'", name));
            break;
        }
    }
    
    for address in &addresses {
        if addresses.iter().filter(|a| *a == address).count() > 1 {
            errors.push(format!("duplicate attestor address found: '{}'", address));
            break;
        }
    }
    
    if !has_enabled {
        errors.push("at least one attestor must be enabled (attestors.registry[].enabled = true)".to_string());
    }
    
    let _ = warnings; // Suppress unused warning
}

fn validate_sessions_section(sessions: &serde_json::Value, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let obj = match sessions.as_object() {
        Some(o) => o,
        None => {
            errors.push("field 'sessions' must be an object".to_string());
            return;
        }
    };
    
    // Required fields
    for required in &["enable_session_tracking", "session_timeout_seconds", "operations_per_session", "audit_log_retention_days"] {
        if !obj.contains_key(*required) {
            errors.push(format!("field 'sessions.{}' is missing", required));
        }
    }
    
    // Validate session_timeout_seconds
    if let Some(timeout) = obj.get("session_timeout_seconds").and_then(|v| v.as_i64()) {
        if timeout < 1 {
            errors.push("field 'sessions.session_timeout_seconds' must be at least 1".to_string());
        } else if timeout < 60 {
            errors.push("field 'sessions.session_timeout_seconds' must be at least 60 seconds".to_string());
        } else if timeout > 86400 {
            warnings.push("field 'sessions.session_timeout_seconds' exceeds 24 hours — consider shorter timeouts for security".to_string());
        }
    } else if obj.contains_key("session_timeout_seconds") {
        errors.push("field 'sessions.session_timeout_seconds' must be an integer".to_string());
    }
    
    // Validate operations_per_session
    if let Some(max_ops) = obj.get("operations_per_session").and_then(|v| v.as_i64()) {
        if max_ops < 1 {
            errors.push("field 'sessions.operations_per_session' must be at least 1".to_string());
        } else if max_ops > 10000 {
            errors.push(format!("field 'sessions.operations_per_session' exceeds maximum of 10000, got {}", max_ops));
        } else if max_ops > 5000 {
            warnings.push("field 'sessions.operations_per_session' is high (>5000) and may impact performance".to_string());
        }
    } else if obj.contains_key("operations_per_session") {
        errors.push("field 'sessions.operations_per_session' must be an integer".to_string());
    }
    
    // Validate audit_log_retention_days
    if let Some(retention) = obj.get("audit_log_retention_days").and_then(|v| v.as_i64()) {
        if retention < 1 {
            errors.push("field 'sessions.audit_log_retention_days' must be at least 1".to_string());
        } else if retention > 3650 {
            errors.push(format!("field 'sessions.audit_log_retention_days' exceeds maximum of 3650 days, got {}", retention));
        }
    } else if obj.contains_key("audit_log_retention_days") {
        errors.push("field 'sessions.audit_log_retention_days' must be an integer".to_string());
    }
    
    // Validate enable_session_tracking
    if obj.contains_key("enable_session_tracking") && !obj.get("enable_session_tracking").and_then(|v| v.as_bool()).is_some() {
        errors.push("field 'sessions.enable_session_tracking' must be a boolean".to_string());
    }
}

fn validate_deployment_section(deployment: &serde_json::Value, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let obj = match deployment.as_object() {
        Some(o) => o,
        None => {
            errors.push("field 'deployment' must be an object".to_string());
            return;
        }
    };

    // admin_key — required, must look like a Stellar public key
    match obj.get("admin_key").and_then(|v| v.as_str()) {
        Some(k) if k.starts_with('G') && k.len() == 56 => {}
        Some(k) => errors.push(format!(
            "field 'deployment.admin_key' must be a 56-char Stellar public key starting with 'G', got '{}'", k
        )),
        None => errors.push("field 'deployment.admin_key' is required".to_string()),
    }

    // rpc_endpoint — required, must be HTTPS
    match obj.get("rpc_endpoint").and_then(|v| v.as_str()) {
        Some(url) => {
            if let Some(err) = validate_endpoint_url(url) {
                errors.push(format!("field 'deployment.rpc_endpoint': {}", err));
            }
        }
        None => errors.push("field 'deployment.rpc_endpoint' is required".to_string()),
    }

    // network — required
    if !obj.contains_key("network") || obj.get("network").and_then(|v| v.as_str()).is_none() {
        errors.push("field 'deployment.network' is required".to_string());
    }

    // contract_id — optional, but if present must look like a Soroban contract address
    if let Some(id) = obj.get("contract_id").and_then(|v| v.as_str()) {
        if !(id.starts_with('C') && id.len() == 56) {
            warnings.push(format!(
                "field 'deployment.contract_id' should be a 56-char Soroban contract address starting with 'C', got '{}'", id
            ));
        }
    }
}

pub fn validate_endpoint_url(url: &str) -> Option<String> {
    if url.is_empty() || url.trim().is_empty() {
        return Some("URL must not be empty".to_string());
    }
    
    if url.len() < 10 {
        return Some("URL too short (minimum 10 characters, e.g. https://a.b)".to_string());
    }
    
    if url.len() > 2048 {
        return Some("URL too long (maximum 2048 characters)".to_string());
    }
    
    // HTTPS is required
    if !url.starts_with("https://") {
        return Some("URL must use HTTPS (http:// and other schemes are not allowed)".to_string());
    }
    
    // Check for forbidden characters
    if url.contains("%00") {
        return Some("URL must not contain a null byte".to_string());
    }
    
    for ch in url.chars() {
        if ch < '\x20' || ch == '\x7f' || "<>{}|\\".contains(ch) {
            return Some(format!("URL contains forbidden character '{}'", ch));
        }
    }
    
    // Extract host
    let after_scheme = &url[8..]; // skip "https://"
    let host_part = after_scheme.split('/').next()
        .and_then(|s| s.split('?').next())
        .and_then(|s| s.split('#').next())
        .unwrap_or("");
    
    if host_part.is_empty() {
        return Some("URL has no host after scheme".to_string());
    }
    
    if host_part.contains(' ') {
        return Some("URL host must not contain spaces".to_string());
    }
    
    // Handle optional port
    let domain = if let Some(colon_pos) = host_part.rfind(':') {
        let port_str = &host_part[colon_pos + 1..];
        if port_str.is_empty() {
            return Some("URL port is empty after colon".to_string());
        }
        if let Ok(port) = port_str.parse::<u16>() {
            if port == 0 {
                return Some("URL port 0 is out of valid range (1-65535)".to_string());
            }
        } else {
            return Some(format!("URL port '{}' is not numeric", port_str));
        }
        &host_part[..colon_pos]
    } else {
        host_part
    };
    
    if domain.is_empty() {
        return Some("URL has no domain".to_string());
    }
    
    // Reject loopback
    let lower = domain.to_lowercase();
    if lower == "localhost" || lower.starts_with("localhost.") || lower.ends_with(".localhost") {
        return Some("URL must not use loopback address (localhost)".to_string());
    }
    
    // Must have a TLD
    if !domain.contains('.') {
        return Some("URL domain must have a TLD (e.g. example.com, not just 'example')".to_string());
    }
    
    if domain.starts_with('.') || domain.ends_with('.') {
        return Some("URL domain must not start or end with a dot".to_string());
    }
    
    if domain.contains("..") {
        return Some("URL domain must not contain consecutive dots".to_string());
    }
    
    let labels: Vec<&str> = domain.split('.').collect();
    let non_empty: Vec<&str> = labels.iter().filter(|l| !l.is_empty()).copied().collect();
    
    if non_empty.len() < 2 {
        return Some("URL domain must have at least two labels (e.g. example.com)".to_string());
    }
    
    // Reject raw IPv4
    if labels.iter().all(|l| l.chars().all(|c| c.is_ascii_digit())) {
        return Some("URL must use a domain name, not a raw IP address".to_string());
    }
    
    for label in &labels {
        if label.is_empty() {
            return Some("URL domain contains an empty label".to_string());
        }
        if label.len() > 63 {
            return Some(format!("URL domain label '{}' exceeds 63 characters", label));
        }
        if !label.chars().next().unwrap().is_ascii_alphanumeric() {
            return Some(format!("URL domain label '{}' must start with an alphanumeric character", label));
        }
        if !label.chars().last().unwrap().is_ascii_alphanumeric() {
            return Some(format!("URL domain label '{}' must end with an alphanumeric character", label));
        }
        if label.to_lowercase().starts_with("xn--") {
            return Some(format!("URL domain label '{}' uses Punycode (xn--), which is not allowed", label));
        }
        for ch in label.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' {
                return Some(format!("URL domain label '{}' contains invalid character '{}'", label, ch));
            }
        }
    }
    
    None
}
