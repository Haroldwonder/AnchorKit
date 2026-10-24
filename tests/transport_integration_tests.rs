#[cfg(test)]
mod transport_integration_tests {
    use soroban_sdk::{testutils::Address as _, Address, Env, String as SorobanString, Bytes};
    use anchorkit::transport::{AnchorTransport, MockTransport, TransportRequest, TransportResponse};
    use anchorkit::types::{HealthStatus, Quote};
    use anchorkit::errors::Error;

    #[test]
    fn test_transport_with_contract_communication_flow() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com/api");
        let base_asset = SorobanString::from_str(&env, "USD");
        let quote_asset = SorobanString::from_str(&env, "USDC");

        let anchor = Address::generate(&env);
        let quote = Quote {
            anchor: anchor.clone(),
            base_asset: base_asset.clone(),
            quote_asset: quote_asset.clone(),
            rate: 10000,
            fee_percentage: 25,
            minimum_amount: 100,
            maximum_amount: 1000000,
            valid_until: 1000000,
            quote_id: 1,
        };

        let quote_request = TransportRequest::GetQuote {
            endpoint: endpoint.clone(),
            base_asset: base_asset.clone(),
            quote_asset: quote_asset.clone(),
            amount: 50000,
        };

        transport.add_response(quote_request.clone(), TransportResponse::Quote(quote));

        let result = transport.send_request(&env, quote_request);
        assert!(result.is_ok());
        assert_eq!(transport.get_call_count(), 1);
    }

    #[test]
    fn test_transport_health_check_integration() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com/api");
        let anchor = Address::generate(&env);

        let health_status = HealthStatus {
            anchor: anchor.clone(),
            latency_ms: 150,
            failure_count: 0,
            availability_percent: 9999,
        };

        let health_request = TransportRequest::CheckHealth {
            endpoint: endpoint.clone(),
        };

        transport.add_response(health_request.clone(), TransportResponse::Health(health_status));

        let result = transport.send_request(&env, health_request);
        assert!(result.is_ok());

        if let Ok(TransportResponse::Health(status)) = result {
            assert_eq!(status.latency_ms, 150);
            assert_eq!(status.availability_percent, 9999);
        } else {
            panic!("Expected Health response");
        }
    }

    #[test]
    fn test_transport_sequential_requests() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com/api");
        let anchor = Address::generate(&env);

        let health = HealthStatus {
            anchor: anchor.clone(),
            latency_ms: 100,
            failure_count: 0,
            availability_percent: 9999,
        };

        let health_request = TransportRequest::CheckHealth {
            endpoint: endpoint.clone(),
        };

        transport.add_response(health_request.clone(), TransportResponse::Health(health));

        let kyc_request = TransportRequest::VerifyKYC {
            endpoint: endpoint.clone(),
            subject_id: SorobanString::from_str(&env, "user_123"),
        };

        transport.add_response(
            kyc_request.clone(),
            TransportResponse::KYCVerified {
                status: SorobanString::from_str(&env, "verified"),
                level: SorobanString::from_str(&env, "advanced"),
            },
        );

        let health_result = transport.send_request(&env, health_request);
        assert!(health_result.is_ok());
        assert_eq!(transport.get_call_count(), 1);

        let kyc_result = transport.send_request(&env, kyc_request);
        assert!(kyc_result.is_ok());
        assert_eq!(transport.get_call_count(), 2);
    }

    #[test]
    fn test_transport_timeout_handling() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        transport.set_simulate_timeout(true, 20);

        let endpoint = SorobanString::from_str(&env, "https://slow-anchor.example.com/api");
        let request = TransportRequest::CheckHealth { endpoint };

        let result_short = transport.send_request_with_timeout(&env, request.clone(), 5);
        assert_eq!(result_short, Err(Error::transport_timeout()));

        let mut transport2 = MockTransport::new();
        transport2.set_simulate_timeout(true, 3);

        let endpoint2 = SorobanString::from_str(&env, "https://fast-anchor.example.com/api");
        let request2 = TransportRequest::CheckHealth { endpoint: endpoint2 };

        let health = HealthStatus {
            anchor: Address::generate(&env),
            latency_ms: 50,
            failure_count: 0,
            availability_percent: 9999,
        };

        transport2.add_response(request2.clone(), TransportResponse::Health(health));
        let result_long = transport2.send_request_with_timeout(&env, request2, 10);
        assert!(result_long.is_ok());
    }

    #[test]
    fn test_transport_error_handling() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        transport.set_should_fail(true);

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com/api");
        let request = TransportRequest::CheckHealth { endpoint };

        let result = transport.send_request(&env, request);
        assert_eq!(result, Err(Error::endpoint_not_found()));
        assert!(!transport.is_available());
    }

    #[test]
    fn test_transport_multiple_endpoints() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        let endpoint1 = SorobanString::from_str(&env, "https://anchor1.example.com/api");
        let endpoint2 = SorobanString::from_str(&env, "https://anchor2.example.com/api");
        let anchor = Address::generate(&env);

        let health1 = HealthStatus {
            anchor: anchor.clone(),
            latency_ms: 50,
            failure_count: 0,
            availability_percent: 9999,
        };

        let health2 = HealthStatus {
            anchor: anchor.clone(),
            latency_ms: 100,
            failure_count: 2,
            availability_percent: 9980,
        };

        let request1 = TransportRequest::CheckHealth {
            endpoint: endpoint1.clone(),
        };
        let request2 = TransportRequest::CheckHealth {
            endpoint: endpoint2.clone(),
        };

        transport.add_response(request1.clone(), TransportResponse::Health(health1));
        transport.add_response(request2.clone(), TransportResponse::Health(health2));

        let result1 = transport.send_request(&env, request1);
        let result2 = transport.send_request(&env, request2);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert_eq!(transport.get_call_count(), 2);
    }
}
