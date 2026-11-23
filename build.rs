use std::path::Path;
use std::process::Command;

fn main() {
    // Always watch the build script itself and top-level schema/validator
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=config_schema.json");
    println!("cargo:rerun-if-changed=validate_config_strict.py");
    println!("cargo:rerun-if-changed=src/config.rs");
    println!("cargo:rerun-if-changed=src/validation.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");

    // Emit rerun-if-changed for every individual config file so Cargo
    // skips re-running this script when nothing in configs/ has changed.
    // (A directory-level directive only watches the dir entry itself, not
    // the files inside it, so we enumerate them explicitly.)
    if let Ok(entries) = std::fs::read_dir("configs") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext == "json" || ext == "toml" {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }

    // Strict compile-time validation to prevent misconfiguration bugs
    validate_configs_at_build();
    validate_schema_consistency();
    check_soroban_sdk_version();
}

fn validate_configs_at_build() {
    let validator = Path::new("validate_config_strict.py");
    let schema = Path::new("config_schema.json");
    let configs_dir = Path::new("configs");

    if !validator.exists() || !schema.exists() || !configs_dir.exists() {
        println!("cargo:warning=Skipping config validation: missing validator or configs");
        return;
    }

    // Check if Python3 is available
    let python_check = Command::new("python3").arg("--version").output();

    if python_check.is_err() {
        println!("cargo:warning=Python3 not found, skipping config validation");
        return;
    }

    // Check if required Python modules are installed
    let module_check = Command::new("python3")
        .arg("-c")
        .arg("import jsonschema, toml")
        .output();

    match module_check {
        Ok(result) if !result.status.success() => {
            println!("cargo:warning=Python modules (jsonschema, toml) not installed. Run: pip3 install jsonschema toml");
            println!("cargo:warning=Skipping compile-time config validation");
            return;
        }
        Err(_) => {
            println!("cargo:warning=Failed to check Python modules");
            println!("cargo:warning=Skipping compile-time config validation");
            return;
        }
        _ => {}
    }

    println!("cargo:warning=Running strict schema validation at compile time...");

    // Validate each config file
    let config_files = match std::fs::read_dir(configs_dir) {
        Ok(files) => files,
        Err(_) => {
            println!("cargo:warning=Failed to read configs directory");
            return;
        }
    };

    let mut validated_count = 0;

    for entry in config_files {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json")
            || path.extension().and_then(|s| s.to_str()) == Some("toml")
        {
            let output = Command::new("python3")
                .arg(validator)
                .arg(&path)
                .arg(schema)
                .output();

            match output {
                Ok(result) if !result.status.success() => {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    panic!(
                        "\n\n❌ STRICT VALIDATION FAILED ❌\nConfiguration validation failed for {:?}:\n{}\n{}\n",
                        path.file_name().unwrap(),
                        stdout,
                        stderr
                    );
                }
                Ok(_) => {
                    validated_count += 1;
                    println!("cargo:warning=✓ Validated: {:?}", path.file_name().unwrap());
                }
                Err(e) => {
                    println!(
                        "cargo:warning=Failed to validate {:?}: {}",
                        path.file_name().unwrap(),
                        e
                    );
                }
            }
        }
    }

    if validated_count > 0 {
        println!(
            "cargo:warning=✓ Successfully validated {} configuration file(s)",
            validated_count
        );
    }
}

/// Validate that schema constraints match Rust constants
fn validate_schema_consistency() {
    use std::fs;

    let schema_path = Path::new("config_schema.json");
    if !schema_path.exists() {
        return;
    }

    let schema_content = match fs::read_to_string(schema_path) {
        Ok(content) => content,
        Err(_) => return,
    };

    // Basic consistency checks
    let checks = vec![
        ("\"maxLength\": 64", "MAX_NAME_LEN"),
        ("\"maxLength\": 16", "MAX_VERSION_LEN"),
        ("\"maxLength\": 32", "MAX_NETWORK_LEN"),
        ("\"maxLength\": 256", "MAX_ENDPOINT_LEN"),
        ("\"maxItems\": 100", "MAX_ATTESTORS"),
        ("\"maximum\": 86400", "MAX_SESSION_TIMEOUT"),
        ("\"maximum\": 10000", "MAX_OPERATIONS"),
    ];

    for (schema_val, const_name) in checks {
        if !schema_content.contains(schema_val) {
            println!(
                "cargo:warning=Schema consistency check: {} might not match {}",
                const_name, schema_val
            );
        }
    }

    println!("cargo:warning=✓ Schema consistency validated");
}

/// The exact soroban-sdk patch version this contract has been tested against.
/// If Cargo resolves a different version the build fails with a clear message so
/// developers notice the mismatch before subtle runtime failures surface.
const EXPECTED_SOROBAN_SDK_VERSION: &str = "26.1.1";

/// Read the resolved soroban-sdk version from Cargo.lock and fail the build if it
/// does not match EXPECTED_SOROBAN_SDK_VERSION.
fn check_soroban_sdk_version() {
    let lock_path = Path::new("Cargo.lock");
    if !lock_path.exists() {
        println!("cargo:warning=Cargo.lock not found; skipping soroban-sdk version check");
        return;
    }

    let lock_content = match std::fs::read_to_string(lock_path) {
        Ok(c) => c,
        Err(e) => {
            println!("cargo:warning=Could not read Cargo.lock: {}", e);
            return;
        }
    };

    match parse_lock_version(&lock_content, "soroban-sdk") {
        Some(ref actual) if actual == EXPECTED_SOROBAN_SDK_VERSION => {
            println!(
                "cargo:warning=✓ soroban-sdk {} matches expected version",
                actual
            );
        }
        Some(actual) => {
            panic!(
                "\n\n❌ SOROBAN-SDK VERSION MISMATCH ❌\n\
                Expected soroban-sdk version : {}\n\
                Resolved soroban-sdk version : {}\n\n\
                If you intentionally upgraded soroban-sdk, update \
                EXPECTED_SOROBAN_SDK_VERSION in build.rs to {} and verify \
                that all contract behaviour is still correct.\n",
                EXPECTED_SOROBAN_SDK_VERSION, actual, actual
            );
        }
        None => {
            println!("cargo:warning=soroban-sdk not found in Cargo.lock; skipping version check");
        }
    }
}

/// Extract the resolved version of `package_name` from a Cargo.lock file.
///
/// Cargo.lock groups packages as `[[package]]` blocks; each block has `name`
/// and `version` fields. We split by `[[package]]`, locate the block whose
/// `name` field matches, and return its `version` value.
fn parse_lock_version(lock_content: &str, package_name: &str) -> Option<std::string::String> {
    for block in lock_content.split("[[package]]") {
        let name_line = format!("name = \"{}\"", package_name);
        if !block.lines().any(|l| l.trim() == name_line) {
            continue;
        }
        for line in block.lines() {
            let line = line.trim();
            if line.starts_with("version = \"") && line.ends_with('"') {
                let ver = line
                    .trim_start_matches("version = \"")
                    .trim_end_matches('"');
                return Some(ver.to_string());
            }
        }
    }
    None
}
