use anchorkit::{AnchorKitContract, LoggingConfig, Logger, RequestId};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

/// Example demonstrating structured logging with debug mode toggle
/// and request/response logging with sensitive data redaction
fn main() {
    println!("🚀 AnchorKit Structured Logging Example");
    println!("========================================");

    let env = Env::default();
    let admin = Address::generate(&env);
    let contract = AnchorKitContract;

    // 1. Initialize contract with logging
    println!("\n📋 Step 1: Initialize contract with logging");
    match contract.initialize(env.clone(), admin.clone()) {
        Ok(_) => println!("✅ Contract initialized successfully"),
        Err(e) => println!("❌ Initialization failed: {:?}", e),
    }

    // 2. Configure logging settings
    println!("\n⚙️  Step 2: Configure logging settings");
    let logging_config = LoggingConfig {
        debug_mode: true,
        log_requests: true,
        log_responses: true,
        redact_sensitive: true,
        max_log_size: 2048,
    };

    match contract.configure_logging(env.clone(), logging_config) {
        Ok(_) => println!("✅ Logging configured successfully"),
        Err(e) => println!("❌ Logging configuration failed: {:?}", e),
    }

    // 3. Demonstrate different log levels
    println!("\n📝 Step 3: Demonstrate different log levels");
    let request_id = RequestId::generate(&env);

    Logger::info(&env, String::from_str(&env, "This is an info message"), Some(request_id));
    Logger::warn(&env, String::from_str(&env, "This is a warning message"), Some(request_id));
    Logger::debug(&env, String::from_str(&env, "This is a debug message (visible in debug mode)"), Some(request_id));
    Logger::trace(&env, String::from_str(&env, "This is a trace message (visible in debug mode)"), Some(request_id));

    println!("✅ Log messages sent (check Soroban events for output)");

    // 4. Attestor registration (SEP-10 JWT required on-chain)
    println!("\n🔄 Step 4: Attestor registration");
    println!("ℹ️  On-chain registration requires admin + set_sep10_jwt_verifying_key + register_attestor(..., token, issuer).");
    println!("   See docs/features/SEP10_AUTH.md and the Sep10AuthFlow UI for the full SEP-10 challenge → JWT flow.");

    // 5. Demonstrate request/response logging
    println!("\n🌐 Step 5: Demonstrate request/response logging");
    let request_id = RequestId::generate(&env);

    // Simulate HTTP request logging
    let request_payload = soroban_sdk::Bytes::from_slice(&env, 
        b"{\"base_asset\":\"USD\",\"quote_asset\":\"USDC\",\"amount\":1000}");
    
    Logger::log_request(
        &env,
        request_id,
        String::from_str(&env, "GET_QUOTE"),
        String::from_str(&env, "https://anchor.example.com/quote"),
        Some(request_payload),
    );

    // Simulate HTTP response logging
    let response_payload = soroban_sdk::Bytes::from_slice(&env, 
        b"{\"rate\":\"1.05\",\"expires_at\":1234567890,\"fee\":\"0.01\"}");
    
    Logger::log_response(
        &env,
        request_id,
        String::from_str(&env, "200_OK"),
        250, // 250ms response time
        Some(response_payload),
    );

    println!("✅ Request/response logged with timing information");

    // 6. Demonstrate sensitive data redaction
    println!("\n🔒 Step 6: Demonstrate sensitive data redaction");
    let sensitive_request_id = RequestId::generate(&env);

    // This payload contains sensitive data that should be redacted
    let sensitive_payload = soroban_sdk::Bytes::from_slice(&env, 
        b"{\"username\":\"user123\",\"password\":\"secret123\",\"token\":\"abc123xyz\"}");
    
    Logger::log_request(
        &env,
        sensitive_request_id,
        String::from_str(&env, "POST_AUTH"),
        String::from_str(&env, "https://anchor.example.com/auth"),
        Some(sensitive_payload),
    );

    println!("✅ Sensitive data logged with redaction enabled");

    // 7. Toggle debug mode off
    println!("\n🔧 Step 7: Toggle debug mode off");
    let production_config = LoggingConfig {
        debug_mode: false,
        log_requests: true,
        log_responses: true,
        redact_sensitive: true,
        max_log_size: 1024,
    };

    match contract.configure_logging(env.clone(), production_config) {
        Ok(_) => println!("✅ Debug mode disabled for production"),
        Err(e) => println!("❌ Configuration update failed: {:?}", e),
    }

    // These debug messages should now be filtered out
    Logger::debug(&env, String::from_str(&env, "This debug message should be filtered out"), None);
    Logger::trace(&env, String::from_str(&env, "This trace message should be filtered out"), None);
    Logger::info(&env, String::from_str(&env, "This info message should still appear"), None);

    println!("✅ Debug messages filtered out in production mode");

    println!("\n🎉 Logging example completed!");
    println!("📊 Check the Soroban events output to see the structured logs");
    println!("💡 In a real deployment, these logs would be captured by your monitoring system");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_example() {
        // Run the example as a test to ensure it doesn't panic
        main();
    }
}