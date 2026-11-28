//! AnchorKit anchor health → Prometheus exporter example.
//!
//! This example periodically reads the health of every routing anchor
//! (`get_health_status` / `get_anchor_health_score`) and exposes Prometheus
//! gauges:
//!
//!   - anchor_health_score     health score (0-100)
//!   - anchor_latency_ms       observed latency in milliseconds
//!   - anchor_failure_count    cumulative failure count
//!   - anchor_active           1 when the anchor is active, 0 otherwise
//!
//! Run:
//!   cargo run --example anchor_health_exporter
//!   # then scrape: http://localhost:9090/metrics
//!
//! Environment:
//!   ANCHORKIT_EXPORTER_ADDR   listen address (default 0.0.0.0:9090)
//!   ANCHORKIT_ANCHORS         comma-separated anchor addresses
//!   ANCHORKIT_SCRAPE_INTERVAL_SECS  scrape period (default 15)
//!
//! In a real deployment the health values come from the AnchorKit contract
//! via `soroban contract invoke ... get_anchor_health_score` / RPC.  This
//! example simulates the read so it can run without a live contract.

#![allow(clippy::too_many_arguments)]

use std::collections::HashMap;
use std::env;
use std::fmt::Write;
use std::net::{TcpListener, TcpStream};
use std::io::{Read, Write as IoWrite};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

const DEFAULT_ADDR: &str = "0.0.0.0:9090";
const DEFAULT_INTERVAL_SECS: u64 = 15;

// Default (simulated) anchor registry used when ANCHORKIT_ANCHORS is unset.
const DEFAULT_ANCHORS: &[&str] = &[
    "GDUMMY1AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "GDUMMY2AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    "GDUMMY3AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
];

/// Latest scraped health sample for one anchor.
#[derive(Clone, Default)]
struct AnchorHealth {
    address: String,
    health_score: u64,
    latency_ms: u64,
    failure_count: u64,
    active: bool,
}

struct ExporterState {
    anchors: Vec<String>,
    samples: RwLock<HashMap<String, AnchorHealth>>,
}

impl ExporterState {
    fn scrape_once(&self) {
        let addrs = self.anchors.clone();
        let mut samples = self.samples.write().unwrap();
        for addr in &addrs {
            let sample = read_anchor_health(addr);
            samples.insert(addr.clone(), sample);
        }
    }

    fn render_prometheus(&self) -> String {
        let samples = self.samples.read().unwrap();
        let mut out = String::new();
        let _ = writeln!(out, "# HELP anchor_health_score Anchor health score (0-100).");
        let _ = writeln!(out, "# TYPE anchor_health_score gauge");
        let _ = writeln!(out, "# HELP anchor_latency_ms Anchor observed latency in milliseconds.");
        let _ = writeln!(out, "# TYPE anchor_latency_ms gauge");
        let _ = writeln!(out, "# HELP anchor_failure_count Anchor cumulative failure count.");
        let _ = writeln!(out, "# TYPE anchor_failure_count gauge");
        let _ = writeln!(out, "# HELP anchor_active Anchor is active (1) or not (0).");
        let _ = writeln!(out, "# TYPE anchor_active gauge");
        for s in samples.values() {
            let _ = writeln!(
                out,
                r#"anchor_health_score{{anchor="{}"}} {}"#,
                s.address, s.health_score
            );
            let _ = writeln!(
                out,
                r#"anchor_latency_ms{{anchor="{}"}} {}"#,
                s.address, s.latency_ms
            );
            let _ = writeln!(
                out,
                r#"anchor_failure_count{{anchor="{}"}} {}"#,
                s.address, s.failure_count
            );
            let _ = writeln!(
                out,
                r#"anchor_active{{anchor="{}"}} {}"#,
                s.address,
                if s.active { 1 } else { 0 }
            );
        }
        out
    }
}

/// Read the health of one anchor.
///
/// Wires to `get_health_status` / `get_anchor_health_score` on the AnchorKit
/// contract (via soroban RPC / `soroban contract invoke`).  Here the read is
/// simulated so the example runs standalone.
fn read_anchor_health(address: &str) -> AnchorHealth {
    // Simulated scrape: derive stable pseudo-values from the address bytes so
    // each anchor gets a distinct, deterministic health profile.
    let mut h: u64 = 0;
    for b in address.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as u64);
    }
    let latency = (h % 400) + 50;          // 50..449 ms
    let failures = (h % 12);               // 0..11
    let availability = 100u64.saturating_sub(failures * 4); // availability %
    let health_score = availability.min(100);
    let active = health_score > 50;
    AnchorHealth {
        address: address.to_string(),
        health_score,
        latency_ms: latency,
        failure_count: failures,
        active,
    }
}

fn handle_request(mut stream: TcpStream, state: &ExporterState) {
    let mut buf = [0u8; 1024];
    let _ = stream.read(&mut buf);
    let request = String::from_utf8_lossy(&buf);
    let path = request
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .unwrap_or("/");

    let body = match path {
        "/metrics" => state.render_prometheus(),
        "/health" => "ok\n".to_string(),
        _ => format!(
            "AnchorKit health exporter\n  GET /metrics  - Prometheus gauges\n  GET /health   - liveness\n",
        ),
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn accept_loop(listener: TcpListener, state: Arc<ExporterState>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let state = Arc::clone(&state);
                thread::spawn(move || handle_request(stream, &state));
            }
            Err(e) => eprintln!("[exporter] accept error: {e}"),
        }
    }
}

fn main() {
    let addr = env::var("ANCHORKIT_EXPORTER_ADDR")
        .unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let interval_secs = env
        .var("ANCHORKIT_SCRAPE_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_INTERVAL_SECS);
    let anchors: Vec<String> = env
        .var("ANCHORKIT_ANCHORS")
        .map(|v| {
            v.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_else(|_| DEFAULT_ANCHORS.iter().map(|s| s.to_string()).collect());

    let state = Arc::new(ExporterState {
        anchors,
        samples: RwLock::new(HashMap::new()),
    });

    // Initial scrape before serving.
    state.scrape_once();

    // Background scraper refreshes samples every interval.
    let state_clone = Arc::clone(&state);
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(interval_secs));
        state_clone.scrape_once();
    });

    let listener = TcpListener::bind(&addr).expect("exporter: cannot bind listen address");
    println!(
        "[exporter] listening on http://{} (scrape every {}s, {} anchors)",
        addr,
        interval_secs,
        state.anchors.len()
    );
    accept_loop(listener, state);
}
