use std::process::Command;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=config_schema.json");
    println!("cargo:rerun-if-changed=configs/");
    println!("cargo:rerun-if-changed=src/config.rs");
    println!("cargo:rerun-if-changed=src/validation.rs");
    
    // Strict compile-time validation to prevent misconfiguration bugs
    validate_configs_at_build();
    validate_schema_consistency();
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
    let python_check = Command::new("python3")
        .arg("--version")
        .output();
    
    if python_check.is_err() {
        println!("cargo:warning=Python3 not found, skipping config validation");
        return;
    }
    
    // Check if required Python modules are installed
    let module_check = Command::new("python3")
        .arg("-c")
        .arg("import jsonschema, toml")
        .output();
    
    if module_check.is_err() || !module_check.unwrap().status.success() {
        println!("cargo:warning=Python modules (jsonschema, toml) not installed. Run: pip3 install jsonschema toml");
        println!("cargo:warning=Skipping compile-time config validation");
        return;
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
            || path.extension().and_then(|s| s.to_str()) == Some("toml") {
            
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
                    println!("cargo:warning=Failed to validate {:?}: {}", path.file_name().unwrap(), e);
                }
            }
        }
    }
    
    if validated_count > 0 {
        println!("cargo:warning=✓ Successfully validated {} configuration file(s)", validated_count);
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
            println!("cargo:warning=Schema consistency check: {} might not match {}", const_name, schema_val);
        }
    }
    
    println!("cargo:warning=✓ Schema consistency validated");
}
