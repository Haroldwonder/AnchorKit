#![cfg(test)]

use crate::{
    transport::{AnchorTransport, MockTransport, TransportRequest, TransportResponse},
    types::{HealthStatus, QuoteData, ServiceType},
    Error,
};
use soroban_sdk::{testutils::Address as _, Address, Bytes, Env, String};

/// Test Goal 1: Ensure requests pass through abstraction
#[test]
fn test_request_passes_through_abstraction() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint = String::from_str(&env, "https://anchor.example.com/api");
    let base_asset = String::from_str(&env, "USD");
    let quote_asset = String::from_str(&env, "USDC");

    // Create a quote request
    let request = TransportRequest::GetQuote {
        endpoint: endpoint.clone(),
        base_asset: base_asset.clone(),
        quote_asset: quote_asset.clone(),
        amount: 5000,
    };

    // Setup mock response
    let anchor = Address::generate(&env);
    let quote = QuoteData {
        anchor: anchor.clone(),
        base_asset: base_asset.clone(),
        quote_asset: quote_asset.clone(),
        rate: 10050,
        fee_percentage: 30,
        minimum_amount: 100,
        maximum_amount: 100000,
        valid_until: env.ledger().timestamp() + 3600,
        quote_id: 42,
    };

    transport.add_response(request.clone(), TransportResponse::Quote(quote.clone()));

    // Send request through abstraction
    let result = transport.send_request(&env, request);

    // Verify request passed through
    assert!(result.is_ok());
    assert_eq!(transport.get_call_count(), 1);

    // Verify response
    match result.unwrap() {
        TransportResponse::Quote(returned_quote) => {
            assert_eq!(returned_quote.anchor, anchor);
            assert_eq!(returned_quote.rate, 10050);
            assert_eq!(returned_quote.fee_percentage, 30);
            assert_eq!(returned_quote.quote_id, 42);
        }
        _ => panic!("Expected Quote response"),
    }
}

/// Test Goal 2: Validate responses without HTTP calls
#[test]
fn test_validate_responses_without_http() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint = String::from_str(&env, "https://anchor.example.com");
    let anchor = Address::generate(&env);

    // Test 1: Health check response
    let health_request = TransportRequest::CheckHealth {
        endpoint: endpoint.clone(),
    };

    let health_response = HealthStatus {
        anchor: anchor.clone(),
        latency_ms: 45,
        failure_count: 0,
        availability_percent: 9950,
        last_check: env.ledger().timestamp(),
    };

    transport.add_response(
        health_request.clone(),
        TransportResponse::Health(health_response.clone()),
    );

    let result = transport.send_request(&env, health_request);
    assert!(result.is_ok());

    match result.unwrap() {
        TransportResponse::Health(health) => {
            assert_eq!(health.anchor, anchor);
            assert_eq!(health.latency_ms, 45);
            assert_eq!(health.failure_count, 0);
            assert_eq!(health.availability_percent, 9950);
        }
        _ => panic!("Expected Health response"),
    }

    // Test 2: KYC verification response
    let kyc_request = TransportRequest::VerifyKYC {
        endpoint: endpoint.clone(),
        subject_id: String::from_str(&env, "user_12345"),
    };

    transport.add_response(
        kyc_request.clone(),
        TransportResponse::KYCVerified {
            status: String::from_str(&env, "approved"),
            level: String::from_str(&env, "intermediate"),
        },
    );

    let result = transport.send_request(&env, kyc_request);
    assert!(result.is_ok());

    match result.unwrap() {
        TransportResponse::KYCVerified { status, level } => {
            assert_eq!(status, String::from_str(&env, "approved"));
            assert_eq!(level, String::from_str(&env, "intermediate"));
        }
        _ => panic!("Expected KYCVerified response"),
    }

    // Verify no actual HTTP calls were made (deterministic)
    assert_eq!(transport.get_call_count(), 2);
    assert_eq!(transport.name(), "MockTransport");
}

/// Test: Multiple sequential requests
#[test]
fn test_multiple_sequential_requests() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint = String::from_str(&env, "https://anchor.example.com");
    let anchor = Address::generate(&env);

    // Setup multiple responses
    for i in 1..=5 {
        let request = TransportRequest::GetQuote {
            endpoint: endpoint.clone(),
            base_asset: String::from_str(&env, "USD"),
            quote_asset: String::from_str(&env, "USDC"),
            amount: i * 1000,
        };

        let quote = QuoteData {
            anchor: anchor.clone(),
            base_asset: String::from_str(&env, "USD"),
            quote_asset: String::from_str(&env, "USDC"),
            rate: 10000 + (i * 10),
            fee_percentage: 25,
            minimum_amount: 100,
            maximum_amount: 100000,
            valid_until: 1000000,
            quote_id: i,
        };

        transport.add_response(request, TransportResponse::Quote(quote));
    }

    // Make sequential requests
    for i in 1..=5 {
        let request = TransportRequest::GetQuote {
            endpoint: endpoint.clone(),
            base_asset: String::from_str(&env, "USD"),
            quote_asset: String::from_str(&env, "USDC"),
            amount: i * 1000,
        };

        let result = transport.send_request(&env, request);
        assert!(result.is_ok());

        if let Ok(TransportResponse::Quote(quote)) = result {
            assert_eq!(quote.quote_id, i);
            assert_eq!(quote.rate, 10000 + (i * 10));
        }
    }

    assert_eq!(transport.get_call_count(), 5);
}

/// Test: Attestation submission request
#[test]
fn test_attestation_submission_request() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint = String::from_str(&env, "https://anchor.example.com/attest");
    let payload = Bytes::from_array(&env, &[1, 2, 3, 4, 5]);

    let request = TransportRequest::SubmitAttestation {
        endpoint: endpoint.clone(),
        payload: payload.clone(),
    };

    let response = TransportResponse::AttestationConfirmed {
        transaction_id: String::from_str(&env, "tx_abc123"),
    };

    transport.add_response(request.clone(), response);

    let result = transport.send_request(&env, request);
    assert!(result.is_ok());

    match result.unwrap() {
        TransportResponse::AttestationConfirmed { transaction_id } => {
            assert_eq!(transaction_id, String::from_str(&env, "tx_abc123"));
        }
        _ => panic!("Expected AttestationConfirmed response"),
    }
}

/// Test: Error response handling
#[test]
fn test_error_response_handling() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint = String::from_str(&env, "https://anchor.example.com");

    let request = TransportRequest::CheckHealth {
        endpoint: endpoint.clone(),
    };

    let error_response = TransportResponse::Error {
        code: 500,
        message: String::from_str(&env, "Internal server error"),
    };

    transport.add_response(request.clone(), error_response);

    let result = transport.send_request(&env, request);
    assert!(result.is_ok());

    match result.unwrap() {
        TransportResponse::Error { code, message } => {
            assert_eq!(code, 500);
            assert_eq!(message, String::from_str(&env, "Internal server error"));
        }
        _ => panic!("Expected Error response"),
    }
}

/// Test: Transport failure simulation
#[test]
fn test_transport_failure_simulation() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    // Configure transport to fail
    transport.set_should_fail(true);
    assert!(!transport.is_available());

    let endpoint = String::from_str(&env, "https://anchor.example.com");
    let request = TransportRequest::CheckHealth { endpoint };

    let result = transport.send_request(&env, request);
    assert_eq!(result, Err(Error::EndpointNotFound));
}

/// Test: Request not found (no mock configured)
#[test]
fn test_request_not_found() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint = String::from_str(&env, "https://unknown.example.com");
    let request = TransportRequest::CheckHealth { endpoint };

    let result = transport.send_request(&env, request);
    assert_eq!(result, Err(Error::EndpointNotFound));
    assert_eq!(transport.get_call_count(), 1);
}

/// Test: Transport reset functionality
#[test]
fn test_transport_reset() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint = String::from_str(&env, "https://anchor.example.com");
    let anchor = Address::generate(&env);

    // Add response and make request
    let request = TransportRequest::CheckHealth {
        endpoint: endpoint.clone(),
    };

    let health = HealthStatus {
        anchor,
        latency_ms: 50,
        failure_count: 0,
        availability_percent: 9999,
        last_check: 1000,
    };

    transport.add_response(request.clone(), TransportResponse::Health(health));
    let _ = transport.send_request(&env, request.clone());

    assert_eq!(transport.get_call_count(), 1);

    // Reset transport
    transport.reset();
    assert_eq!(transport.get_call_count(), 0);
    assert!(transport.is_available());

    // Request should now fail (no mock configured)
    let result = transport.send_request(&env, request);
    assert_eq!(result, Err(Error::EndpointNotFound));
}

/// Test: Different endpoints for same request type
#[test]
fn test_different_endpoints_same_request_type() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint1 = String::from_str(&env, "https://anchor1.example.com");
    let endpoint2 = String::from_str(&env, "https://anchor2.example.com");

    let anchor1 = Address::generate(&env);
    let anchor2 = Address::generate(&env);

    // Setup responses for different endpoints
    let request1 = TransportRequest::CheckHealth {
        endpoint: endpoint1.clone(),
    };
    let health1 = HealthStatus {
        anchor: anchor1.clone(),
        latency_ms: 30,
        failure_count: 0,
        availability_percent: 9999,
        last_check: 1000,
    };
    transport.add_response(request1.clone(), TransportResponse::Health(health1));

    let request2 = TransportRequest::CheckHealth {
        endpoint: endpoint2.clone(),
    };
    let health2 = HealthStatus {
        anchor: anchor2.clone(),
        latency_ms: 60,
        failure_count: 2,
        availability_percent: 9800,
        last_check: 1000,
    };
    transport.add_response(request2.clone(), TransportResponse::Health(health2));

    // Verify different responses for different endpoints
    let result1 = transport.send_request(&env, request1);
    assert!(result1.is_ok());
    if let Ok(TransportResponse::Health(health)) = result1 {
        assert_eq!(health.anchor, anchor1);
        assert_eq!(health.latency_ms, 30);
    }

    let result2 = transport.send_request(&env, request2);
    assert!(result2.is_ok());
    if let Ok(TransportResponse::Health(health)) = result2 {
        assert_eq!(health.anchor, anchor2);
        assert_eq!(health.latency_ms, 60);
    }

    assert_eq!(transport.get_call_count(), 2);
}

/// Test: Request matching with different parameters
#[test]
fn test_request_matching_different_parameters() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint = String::from_str(&env, "https://anchor.example.com");
    let anchor = Address::generate(&env);

    // Setup quote for specific amount
    let request_1000 = TransportRequest::GetQuote {
        endpoint: endpoint.clone(),
        base_asset: String::from_str(&env, "USD"),
        quote_asset: String::from_str(&env, "USDC"),
        amount: 1000,
    };

    let quote_1000 = QuoteData {
        anchor: anchor.clone(),
        base_asset: String::from_str(&env, "USD"),
        quote_asset: String::from_str(&env, "USDC"),
        rate: 10000,
        fee_percentage: 25,
        minimum_amount: 100,
        maximum_amount: 100000,
        valid_until: 1000000,
        quote_id: 1,
    };

    transport.add_response(request_1000.clone(), TransportResponse::Quote(quote_1000));

    // Request with same amount should match
    let result = transport.send_request(&env, request_1000);
    assert!(result.is_ok());

    // Request with different amount should NOT match
    let request_2000 = TransportRequest::GetQuote {
        endpoint: endpoint.clone(),
        base_asset: String::from_str(&env, "USD"),
        quote_asset: String::from_str(&env, "USDC"),
        amount: 2000,
    };

    let result = transport.send_request(&env, request_2000);
    assert_eq!(result, Err(Error::EndpointNotFound));
}

/// Test: Transport availability check
#[test]
fn test_transport_availability() {
    let mut transport = MockTransport::new();

    // Initially available
    assert!(transport.is_available());

    // Set to fail
    transport.set_should_fail(true);
    assert!(!transport.is_available());

    // Reset makes it available again
    transport.reset();
    assert!(transport.is_available());
}

/// Test: Call count tracking
#[test]
fn test_call_count_tracking() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    assert_eq!(transport.get_call_count(), 0);

    let endpoint = String::from_str(&env, "https://anchor.example.com");

    // Make multiple requests (some will fail)
    for i in 0..10 {
        let request = TransportRequest::CheckHealth {
            endpoint: endpoint.clone(),
        };
        let _ = transport.send_request(&env, request);
    }

    assert_eq!(transport.get_call_count(), 10);
}

/// Test: Complex quote request with all parameters
#[test]
fn test_complex_quote_request() {
    let env = Env::default();
    let mut transport = MockTransport::new();

    let endpoint = String::from_str(&env, "https://premium-anchor.example.com/v2/quotes");
    let base_asset = String::from_str(&env, "EUR");
    let quote_asset = String::from_str(&env, "EURC");
    let amount = 50000u64;

    let request = TransportRequest::GetQuote {
        endpoint: endpoint.clone(),
        base_asset: base_asset.clone(),
        quote_asset: quote_asset.clone(),
        amount,
    };

    let anchor = Address::generate(&env);
    let quote = QuoteData {
        anchor: anchor.clone(),
        base_asset: base_asset.clone(),
        quote_asset: quote_asset.clone(),
        rate: 10025, // 1.0025 (0.25% markup)
        fee_percentage: 15, // 0.15%
        minimum_amount: 1000,
        maximum_amount: 1000000,
        valid_until: env.ledger().timestamp() + 7200,
        quote_id: 999,
    };

    transport.add_response(request.clone(), TransportResponse::Quote(quote.clone()));

    let result = transport.send_request(&env, request);
    assert!(result.is_ok());

    match result.unwrap() {
        TransportResponse::Quote(returned_quote) => {
            assert_eq!(returned_quote.base_asset, base_asset);
            assert_eq!(returned_quote.quote_asset, quote_asset);
            assert_eq!(returned_quote.rate, 10025);
            assert_eq!(returned_quote.fee_percentage, 15);
            assert_eq!(returned_quote.minimum_amount, 1000);
            assert_eq!(returned_quote.maximum_amount, 1000000);
        }
        _ => panic!("Expected Quote response"),
    }
}
