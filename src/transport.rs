extern crate alloc;

use crate::errors::Error;
use crate::types::{HealthStatus, Quote};
use soroban_sdk::{Bytes, Env, String};

/// Transport request types
#[derive(Clone, Debug, PartialEq)]
pub enum TransportRequest {
    GetQuote {
        endpoint: String,
        base_asset: String,
        quote_asset: String,
        amount: u64,
    },
    SubmitAttestation {
        endpoint: String,
        payload: Bytes,
    },
    CheckHealth {
        endpoint: String,
    },
    VerifyKYC {
        endpoint: String,
        subject_id: String,
    },
}

/// Transport response types
#[derive(Clone, Debug, PartialEq)]
pub enum TransportResponse {
    Quote(Quote),
    AttestationConfirmed { transaction_id: String },
    Health(HealthStatus),
    KYCVerified { status: String, level: String },
    Error { code: u32, message: String },
}

/// Transport abstraction trait for anchor communication
/// This allows for both real HTTP implementations and mock implementations for testing
pub trait AnchorTransport {
    /// Send a request to an anchor and receive a response
    fn send_request(
        &mut self,
        env: &Env,
        request: TransportRequest,
    ) -> Result<TransportResponse, Error>;

    /// Send a request with timeout enforcement
    fn send_request_with_timeout(
        &mut self,
        env: &Env,
        request: TransportRequest,
        timeout_seconds: u32,
    ) -> Result<TransportResponse, Error>;

    /// Check if the transport is available
    fn is_available(&self) -> bool;

    /// Get transport name for debugging
    fn name(&self) -> &str;
}

/// Mock transport implementation for deterministic testing
/// Allows pre-configured responses without actual HTTP calls
pub struct MockTransport {
    responses: alloc::vec::Vec<(TransportRequest, TransportResponse)>,
    call_count: u32,
    should_fail: bool,
    simulate_timeout: bool,
    simulated_delay_seconds: u32,
}

impl MockTransport {
    /// Create a new mock transport
    pub fn new() -> Self {
        Self {
            responses: alloc::vec::Vec::new(),
            call_count: 0,
            should_fail: false,
            simulate_timeout: false,
            simulated_delay_seconds: 0,
        }
    }

    /// Add a mock response for a specific request
    pub fn add_response(&mut self, request: TransportRequest, response: TransportResponse) {
        self.responses.push((request, response));
    }

    /// Configure the transport to fail all requests
    pub fn set_should_fail(&mut self, should_fail: bool) {
        self.should_fail = should_fail;
    }

    /// Configure the transport to simulate timeout
    pub fn set_simulate_timeout(&mut self, simulate: bool, delay_seconds: u32) {
        self.simulate_timeout = simulate;
        self.simulated_delay_seconds = delay_seconds;
    }

    /// Get the number of requests made
    pub fn get_call_count(&self) -> u32 {
        self.call_count
    }

    /// Reset the mock transport state
    pub fn reset(&mut self) {
        self.responses.clear();
        self.call_count = 0;
        self.should_fail = false;
        self.simulate_timeout = false;
        self.simulated_delay_seconds = 0;
    }

    /// Find matching response for a request
    fn find_response(&self, request: &TransportRequest) -> Option<TransportResponse> {
        for (req, resp) in &self.responses {
            if Self::requests_match(req, request) {
                return Some(resp.clone());
            }
        }
        None
    }

    /// Check if two requests match (for mock lookup)
    fn requests_match(a: &TransportRequest, b: &TransportRequest) -> bool {
        match (a, b) {
            (
                TransportRequest::GetQuote {
                    endpoint: e1,
                    base_asset: b1,
                    quote_asset: q1,
                    amount: a1,
                },
                TransportRequest::GetQuote {
                    endpoint: e2,
                    base_asset: b2,
                    quote_asset: q2,
                    amount: a2,
                },
            ) => e1 == e2 && b1 == b2 && q1 == q2 && a1 == a2,
            (
                TransportRequest::SubmitAttestation {
                    endpoint: e1,
                    payload: p1,
                },
                TransportRequest::SubmitAttestation {
                    endpoint: e2,
                    payload: p2,
                },
            ) => e1 == e2 && p1 == p2,
            (
                TransportRequest::CheckHealth { endpoint: e1 },
                TransportRequest::CheckHealth { endpoint: e2 },
            ) => e1 == e2,
            (
                TransportRequest::VerifyKYC {
                    endpoint: e1,
                    subject_id: s1,
                },
                TransportRequest::VerifyKYC {
                    endpoint: e2,
                    subject_id: s2,
                },
            ) => e1 == e2 && s1 == s2,
            _ => false,
        }
    }
}

impl AnchorTransport for MockTransport {
    fn send_request(
        &mut self,
        _env: &Env,
        request: TransportRequest,
    ) -> Result<TransportResponse, Error> {
        self.call_count += 1;

        if self.should_fail {
            return Err(Error::endpoint_not_found());
        }

        match self.find_response(&request) {
            Some(response) => Ok(response),
            None => Err(Error::endpoint_not_found()),
        }
    }

    fn send_request_with_timeout(
        &mut self,
        env: &Env,
        request: TransportRequest,
        timeout_seconds: u32,
    ) -> Result<TransportResponse, Error> {
        // Check if simulated delay exceeds timeout
        if self.simulate_timeout && self.simulated_delay_seconds > timeout_seconds {
            self.call_count += 1;
            return Err(Error::transport_timeout());
        }

        // Otherwise proceed with normal request
        self.send_request(env, request)
    }

    fn is_available(&self) -> bool {
        !self.should_fail
    }

    fn name(&self) -> &str {
        "MockTransport"
    }
}

impl Default for MockTransport {
    fn default() -> Self {
        Self::new()
    }
}

// ── Optional OpenTelemetry tracing for anchor HTTP calls ─────────────────────
//
// Enabled with the `otel` Cargo feature (off by default). `OtelTransport` wraps
// any `AnchorTransport` and emits one span per anchor request carrying the
// anchor domain, endpoint, attempt number and request status. The current
// `traceparent` can be injected into outgoing HTTP requests via
// `OtelTransport::traceparent_headers`, and spans can be exported to an OTLP
// collector with `opentelemetry_otlp`.
#[cfg(feature = "otel")]
pub mod otel {
    use super::{AnchorTransport, Error, TransportRequest, TransportResponse};
    use opentelemetry::KeyValue;
    use opentelemetry::propagation::Setter;
    use opentelemetry::trace::{Span, Tracer, TraceContextExt};
    use opentelemetry::Context;
    use soroban_sdk::Env;
    use std::cell::Cell;

    const ANCHOR_DOMAIN: &str = "anchor.domain";
    const ENDPOINT: &str = "anchor.endpoint";
    const HTTP_STATUS: &str = "http.status_code";
    const ATTEMPT: &str = "anchor.attempt";

    /// Carrier used to inject a `traceparent` header into outgoing requests.
    struct HeaderSetter<'a>(&'a mut Vec<(String, String)>);

    impl<'a> Setter<Vec<(String, String)>> for HeaderSetter<'a> {
        fn set(&mut self, carrier: &mut Vec<(String, String)>, key: &str, value: String) {
            carrier.push((key.to_string(), value));
        }
    }

    fn endpoint_domain(endpoint: &str) -> String {
        let rest = endpoint.split_once("://").map(|(_, r)| r).unwrap_or(endpoint);
        rest.split('/').next().unwrap_or(rest).to_string()
    }

    fn request_endpoint(request: &TransportRequest) -> String {
        match request {
            TransportRequest::GetQuote { endpoint, .. }
            | TransportRequest::SubmitAttestation { endpoint, .. }
            | TransportRequest::CheckHealth { endpoint }
            | TransportRequest::VerifyKYC { endpoint, .. } => endpoint.clone(),
        }
    }

    fn response_status(response: &TransportResponse) -> i64 {
        match response {
            TransportResponse::Quote { .. }
            | TransportResponse::AttestationConfirmed { .. }
            | TransportResponse::Health { .. }
            | TransportResponse::KYCVerified { .. } => 200,
            TransportResponse::Error { code, .. } => *code as i64,
        }
    }

    /// Wraps an inner [`AnchorTransport`] and emits an OpenTelemetry span per
    /// request. Spans carry `anchor.domain`, `anchor.endpoint`, `anchor.attempt`
    /// and `http.status_code`. Call `traceparent_headers` to obtain headers to
    /// attach to the raw outbound HTTP call for context propagation.
    pub struct OtelTransport<T: AnchorTransport> {
        inner: T,
        tracer: opentelemetry::global::BoxedTracer,
        attempt: Cell<u32>,
    }

    impl<T: AnchorTransport> OtelTransport<T> {
        pub fn new(inner: T) -> Self {
            Self {
                inner,
                tracer: opentelemetry::global::tracer("anchorkit"),
                attempt: Cell::new(0),
            }
        }

        /// Execute a request inside a span. `attempt` is the current retry
        /// attempt (0-based). Returns the transport response plus the
        /// `traceparent` headers to attach to the outgoing HTTP request.
        pub fn send_request_traced(
            &mut self,
            env: &Env,
            request: TransportRequest,
            attempt: u32,
        ) -> Result<(TransportResponse, Vec<(String, String)>), Error> {
            self.attempt.set(attempt);

            let cx = Context::current();
            let mut span = self
                .tracer
                .build("anchor.request")
                .with_context(cx)
                .start(&self.tracer);

            let endpoint = request_endpoint(&request);
            span.set_attribute(KeyValue::new(ANCHOR_DOMAIN, endpoint_domain(&endpoint)));
            span.set_attribute(KeyValue::new(ENDPOINT, endpoint));
            span.set_attribute(KeyValue::new(ATTEMPT, attempt as i64));

            let mut headers: Vec<(String, String)> = Vec::new();
            let setter = HeaderSetter(&mut headers);
            opentelemetry::global::get_text_map_propagator()
                .inject_context(&cx, setter);

            let result = self.inner.send_request(env, request);
            match &result {
                Ok(response) => {
                    span.set_attribute(KeyValue::new(HTTP_STATUS, response_status(response)));
                    span.set_attribute(KeyValue::new("anchor.status", "ok"));
                }
                Err(err) => {
                    span.set_attribute(KeyValue::new(HTTP_STATUS, 0));
                    span.set_attribute(KeyValue::new("anchor.status", "error"));
                    span.set_attribute(KeyValue::new("anchor.error", format!("{:?}", err)));
                }
            }
            span.end();

            result.map(|response| (response, headers))
        }

        /// Returns the `traceparent` headers for the current context so callers
        /// can attach them to raw outbound HTTP requests.
        pub fn traceparent_headers(&self) -> Vec<(String, String)> {
            let mut headers: Vec<(String, String)> = Vec::new();
            let setter = HeaderSetter(&mut headers);
            opentelemetry::global::get_text_map_propagator()
                .inject_context(&Context::current(), setter);
            headers
        }

        pub fn inner(&self) -> &T {
            &self.inner
        }
    }

    impl<T: AnchorTransport> AnchorTransport for OtelTransport<T> {
        fn send_request(
            &mut self,
            env: &Env,
            request: TransportRequest,
        ) -> Result<TransportResponse, Error> {
            self.send_request_traced(env, request, self.attempt.get())
                .map(|(response, _)| response)
        }

        fn send_request_with_timeout(
            &mut self,
            env: &Env,
            request: TransportRequest,
            timeout_seconds: u32,
        ) -> Result<TransportResponse, Error> {
            self.inner.send_request_with_timeout(env, request, timeout_seconds)
        }

        fn is_available(&self) -> bool {
            self.inner.is_available()
        }

        fn name(&self) -> &str {
            "OtelTransport"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String as SorobanString};

    #[test]
    fn test_mock_transport_creation() {
        let transport = MockTransport::new();
        assert_eq!(transport.get_call_count(), 0);
        assert!(transport.is_available());
        assert_eq!(transport.name(), "MockTransport");
    }

    #[test]
    fn test_mock_transport_add_response() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com");
        let base_asset = SorobanString::from_str(&env, "USD");
        let quote_asset = SorobanString::from_str(&env, "USDC");

        let request = TransportRequest::GetQuote {
            endpoint: endpoint.clone(),
            base_asset: base_asset.clone(),
            quote_asset: quote_asset.clone(),
            amount: 1000,
        };

        let anchor = Address::generate(&env);
        let quote = Quote {
            anchor: anchor.clone(),
            base_asset: base_asset.clone(),
            quote_asset: quote_asset.clone(),
            rate: 10000,
            fee_percentage: 25,
            minimum_amount: 100,
            maximum_amount: 10000,
            valid_until: 1000000,
            quote_id: 1,
        };

        let response = TransportResponse::Quote(quote.clone());
        transport.add_response(request.clone(), response);

        let result = transport.send_request(&env, request);
        assert!(result.is_ok());
        assert_eq!(transport.get_call_count(), 1);

        if let Ok(TransportResponse::Quote(returned_quote)) = result {
            assert_eq!(returned_quote.anchor, anchor);
            assert_eq!(returned_quote.rate, 10000);
        } else {
            panic!("Expected Quote response");
        }
    }

    #[test]
    fn test_mock_transport_not_found() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com");
        let base_asset = SorobanString::from_str(&env, "USD");
        let quote_asset = SorobanString::from_str(&env, "USDC");

        let request = TransportRequest::GetQuote {
            endpoint,
            base_asset,
            quote_asset,
            amount: 1000,
        };

        let result = transport.send_request(&env, request);
        assert_eq!(result, Err(Error::endpoint_not_found()));
        assert_eq!(transport.get_call_count(), 1);
    }

    #[test]
    fn test_mock_transport_should_fail() {
        let env = Env::default();
        let mut transport = MockTransport::new();
        transport.set_should_fail(true);

        assert!(!transport.is_available());

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com");
        let request = TransportRequest::CheckHealth { endpoint };

        let result = transport.send_request(&env, request);
        assert_eq!(result, Err(Error::endpoint_not_found()));
    }

    #[test]
    fn test_mock_transport_reset() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com");
        let request = TransportRequest::CheckHealth {
            endpoint: endpoint.clone(),
        };

        let anchor = Address::generate(&env);
        let health = HealthStatus {
            anchor,
            latency_ms: 50,
            failure_count: 0,
            availability_percent: 9999,
        };

        transport.add_response(request.clone(), TransportResponse::Health(health));
        let _ = transport.send_request(&env, request.clone());

        assert_eq!(transport.get_call_count(), 1);

        transport.reset();
        assert_eq!(transport.get_call_count(), 0);

        let result = transport.send_request(&env, request);
        assert_eq!(result, Err(Error::endpoint_not_found()));
    }

    #[test]
    fn test_mock_transport_multiple_requests() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com");
        let anchor = Address::generate(&env);

        // Add health check response
        let health_request = TransportRequest::CheckHealth {
            endpoint: endpoint.clone(),
        };
        let health = HealthStatus {
            anchor: anchor.clone(),
            latency_ms: 50,
            failure_count: 0,
            availability_percent: 9999,
        };
        transport.add_response(health_request.clone(), TransportResponse::Health(health));

        // Add KYC verification response
        let kyc_request = TransportRequest::VerifyKYC {
            endpoint: endpoint.clone(),
            subject_id: SorobanString::from_str(&env, "user123"),
        };
        transport.add_response(
            kyc_request.clone(),
            TransportResponse::KYCVerified {
                status: SorobanString::from_str(&env, "approved"),
                level: SorobanString::from_str(&env, "advanced"),
            },
        );

        // Make requests
        let health_result = transport.send_request(&env, health_request);
        assert!(health_result.is_ok());
        assert_eq!(transport.get_call_count(), 1);

        let kyc_result = transport.send_request(&env, kyc_request);
        assert!(kyc_result.is_ok());
        assert_eq!(transport.get_call_count(), 2);
    }

    #[test]
    fn test_request_timeout_exceeded() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        // Simulate a request that takes 15 seconds
        transport.set_simulate_timeout(true, 15);

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com");
        let request = TransportRequest::CheckHealth { endpoint };

        // Request with 10 second timeout should fail
        let result = transport.send_request_with_timeout(&env, request, 10);
        assert_eq!(result, Err(Error::transport_timeout()));
        assert_eq!(transport.get_call_count(), 1);
    }

    #[test]
    fn test_request_timeout_not_exceeded() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        // Simulate a request that takes 5 seconds
        transport.set_simulate_timeout(true, 5);

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com");
        let anchor = Address::generate(&env);
        let health = HealthStatus {
            anchor: anchor.clone(),
            latency_ms: 50,
            failure_count: 0,
            availability_percent: 9999,
        };

        let request = TransportRequest::CheckHealth {
            endpoint: endpoint.clone(),
        };
        transport.add_response(request.clone(), TransportResponse::Health(health));

        // Request with 10 second timeout should succeed
        let result = transport.send_request_with_timeout(&env, request, 10);
        assert!(result.is_ok());
        assert_eq!(transport.get_call_count(), 1);
    }

    #[test]
    fn test_default_timeout_from_sdk_config() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        // Simulate a slow request (15 seconds)
        transport.set_simulate_timeout(true, 15);

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com");
        let request = TransportRequest::CheckHealth { endpoint };

        // Use default timeout of 10 seconds (from SdkConfig default)
        let default_timeout = 10u32;
        let result = transport.send_request_with_timeout(&env, request, default_timeout);
        assert_eq!(result, Err(Error::transport_timeout()));
    }

    #[test]
    fn test_timeout_with_different_request_types() {
        let env = Env::default();
        let mut transport = MockTransport::new();

        transport.set_simulate_timeout(true, 20);

        let endpoint = SorobanString::from_str(&env, "https://anchor.example.com");

        // Test GetQuote timeout
        let quote_request = TransportRequest::GetQuote {
            endpoint: endpoint.clone(),
            base_asset: SorobanString::from_str(&env, "USD"),
            quote_asset: SorobanString::from_str(&env, "USDC"),
            amount: 1000,
        };
        let result = transport.send_request_with_timeout(&env, quote_request, 10);
        assert_eq!(result, Err(Error::transport_timeout()));

        // Test SubmitAttestation timeout
        let attest_request = TransportRequest::SubmitAttestation {
            endpoint: endpoint.clone(),
            payload: Bytes::new(&env),
        };
        let result = transport.send_request_with_timeout(&env, attest_request, 10);
        assert_eq!(result, Err(Error::transport_timeout()));

        // Test VerifyKYC timeout
        let kyc_request = TransportRequest::VerifyKYC {
            endpoint: endpoint.clone(),
            subject_id: SorobanString::from_str(&env, "user123"),
        };
        let result = transport.send_request_with_timeout(&env, kyc_request, 10);
        assert_eq!(result, Err(Error::transport_timeout()));
    }
}
