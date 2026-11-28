//! `anchorkit session create|get|list` — manage interaction sessions via Soroban RPC.

use crate::output::format_timestamp;
use crate::soroban_rpc::{create_session_rpc, get_session_rpc, list_sessions_rpc};
use crate::exit_code;

/// CLI representation of a session
#[derive(serde::Serialize, serde::Deserialize)]
struct SessionRecord {
    session_id: u64,
    initiator: String,
    created_at: u64,
    operation_count: u64,
    expires_at: u64,
}

// ── Session management commands ──────────────────────────────────────────────

pub fn run_session_create(initiator: Option<String>) {
    println!("◈ Creating new session");
    println!();

    match &initiator {
        Some(addr) => {
            println!("  Initiator: {}", addr);
        }
        None => {
            println!("  Initiator: using STELLAR_SECRET_KEY");
        }
    }

    match create_session_on_chain(initiator) {
        Ok(session_id) => {
            println!();
            println!("✔ Session created successfully");
            println!("  Session ID: {}", session_id);
            println!();
            println!("Use this session ID for subsequent operations:");
            println!("  anchorkit session get {}", session_id);
        }
        Err(e) => {
            eprintln!("✖ Failed to create session: {}", e);
            std::process::exit(exit_code::exit_code_for_rpc_error(&e));
        }
    }
}

pub fn run_session_get(session_id: u64) {
    println!("◈ Fetching session {}", session_id);
    println!();

    match fetch_session_from_chain(session_id) {
        Ok(Some(session)) => {
            print_session(&session);
            println!();
            println!("✔ Session retrieved successfully");
        }
        Ok(None) => {
            eprintln!("✖ Session {} not found", session_id);
            std::process::exit(exit_code::CONTRACT);
        }
        Err(e) => {
            eprintln!("✖ Failed to fetch session {}: {}", session_id, e);
            std::process::exit(exit_code::exit_code_for_rpc_error(&e));
        }
    }
}

pub fn run_session_list(format: &str, pretty: bool, limit: Option<u64>) {
    let limit = limit.unwrap_or(100);
    println!("◈ Fetching sessions (limit: {})", limit);

    // Validate format
    if !["text", "json", "csv"].contains(&format) {
        eprintln!("error: unsupported format '{}'. Use 'text', 'json', or 'csv'", format);
        std::process::exit(exit_code::USAGE);
    }

    let sessions = match fetch_sessions_from_chain(limit) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("✖ Failed to fetch sessions: {}", e);
            std::process::exit(exit_code::exit_code_for_rpc_error(&e));
        }
    };

    if sessions.is_empty() {
        println!("\n✗ No sessions found");
        return;
    }

    println!();

    match format {
        "text" => {
            println!("┌─ Active Sessions ──────────────────────────────────────────────");
            for (idx, session) in sessions.iter().enumerate() {
                if idx > 0 {
                    println!("├───────────────────────────────────────────────────────────────");
                }
                print_session_compact(&session);
            }
            println!("└───────────────────────────────────────────────────────────────────");
            println!();
            println!("✔ Retrieved {} session{}", 
                     sessions.len(), 
                     if sessions.len() == 1 { "" } else { "s" });
        }
        "json" => {
            let json_output = if pretty {
                serde_json::to_string_pretty(&sessions)
                    .unwrap_or_else(|_| "[]".to_string())
            } else {
                serde_json::to_string(&sessions)
                    .unwrap_or_else(|_| "[]".to_string())
            };
            println!("{}", json_output);
        }
        "csv" => {
            println!("session_id,initiator,created_at,operation_count,expires_at");
            for session in &sessions {
                println!(
                    "{},{},{},{},{}",
                    session.session_id,
                    session.initiator,
                    session.created_at,
                    session.operation_count,
                    session.expires_at,
                );
            }
        }
        _ => unreachable!(),
    }
}

// ── Session helper functions ─────────────────────────────────────────────────

fn print_session(session: &SessionRecord) {
    println!("  Session ID:       {}", session.session_id);
    println!("  Initiator:        {}", session.initiator);
    println!("  Created At:       {} ({})", 
             session.created_at, 
             format_timestamp(session.created_at));
    println!("  Operation Count:  {}", session.operation_count);
    println!("  Expires At:       {} ({})", 
             session.expires_at,
             format_timestamp(session.expires_at));
}

fn print_session_compact(session: &SessionRecord) {
    println!("│ Session: {}  Initiator: {}...", 
             session.session_id,
             &session.initiator.chars().take(16).collect::<String>());
    println!("│ Created: {}  Operations: {}  Expires: {}", 
             session.created_at,
             session.operation_count,
             session.expires_at);
}

fn create_session_on_chain(initiator: Option<String>) -> Result<u64, String> {
    create_session_rpc(initiator)
}

fn fetch_session_from_chain(session_id: u64) -> Result<Option<SessionRecord>, String> {
    match get_session_rpc(session_id) {
        Ok(session_data) => Ok(Some(SessionRecord {
            session_id: session_data.session_id,
            initiator: session_data.initiator,
            created_at: session_data.created_at,
            operation_count: session_data.operation_count,
            expires_at: session_data.expires_at,
        })),
        // #914: surface the descriptive RPC error instead of silently returning None
        Err(e) => Err(e),
    }
}

fn fetch_sessions_from_chain(limit: u64) -> Result<Vec<SessionRecord>, String> {
    match list_sessions_rpc(limit) {
        Ok(sessions) => Ok(sessions
            .into_iter()
            .map(|s| SessionRecord {
                session_id: s.session_id,
                initiator: s.initiator,
                created_at: s.created_at,
                operation_count: s.operation_count,
                expires_at: s.expires_at,
            })
            .collect()),
        // #914: surface the descriptive RPC error instead of silently returning vec![]
        Err(e) => Err(e),
    }
}
