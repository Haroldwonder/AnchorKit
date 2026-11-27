//! Example: exporting AnchorKit anchor-request traces to an OTLP collector.
//!
//! Run with the `otel` feature enabled:
//!
//! ```sh
//! cargo run --example otel_http_collector --features otel
//! ```
//!
//! This example configures an OTLP exporter so spans emitted by
//! [`OtelTransport`](anchorkit::transport::otel::OtelTransport) are exported to
//! an OTLP collector (e.g. the OpenTelemetry Collector). Each anchor request is
//! traced with the anchor domain, endpoint, attempt number and request status,
//! and `traceparent` headers are propagated to outbound HTTP calls.

use opentelemetry::sdk::trace::TracerProvider;
use opentelemetry_sdk::trace::Sampler;
use opentelemetry_otlp::WithExport;

fn main() {
    // ── OTLP exporter ──────────────────────────────────────────────────────
    // Spans are exported to an OTLP collector over gRPC. Point this at your
    // collector (default local endpoint shown below).
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://localhost:4317");

    let provider = TracerProvider::builder()
        .with_exporter(exporter)
        .with_sampler(Sampler::AlwaysOn)
        .build();

    opentelemetry::global::set_tracer_provider(provider);

    // ── Wrap the anchor transport ──────────────────────────────────────────
    // Behind `--features otel`, `OtelTransport` wraps any `AnchorTransport`
    // (SEP-6 / SEP-10 / health / KYC) and emits one span per request carrying
    // `anchor.domain`, `anchor.endpoint`, `anchor.attempt` and `status`.
    //
    // let mut inner = MockTransport::new();
    // let mut transport = OtelTransport::new(inner);
    //
    // let attempt = 0;
    // let (response, traceparent_headers) =
    //     transport.send_request_traced(&env, request, attempt);
    //
    // Attach `traceparent_headers` to your outbound HTTP request so the anchor
    // and intermediaries join the same trace:
    //
    // for (key, value) in &traceparent_headers {
    //     http_request.headers().insert(key, value.parse().unwrap());
    // }
}
