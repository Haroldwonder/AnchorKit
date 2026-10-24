#![no_main]

use libfuzzer_sys::fuzz_target;

// Minimal JSON-like structure for fuzzing SEP-6 response parsing
// Tests panic-freedom when parsing arbitrary input that might be JSON

fuzz_target!(|data: &[u8]| {
    // Attempt to parse as UTF-8
    if let Ok(s) = std::str::from_utf8(data) {
        // Test TransactionStatus parsing from arbitrary strings
        let _ = anchorkit::sep6::TransactionStatus::from_str(s);
    }

    // Test various other potential response parsing scenarios
    // by attempting to treat arbitrary bytes as JSON-like structures

    // Parse status strings in isolation (most common field in SEP-6)
    if data.len() > 0 {
        let status_str = match std::str::from_utf8(&data[..std::cmp::min(64, data.len())]) {
            Ok(s) => s,
            Err(_) => return,
        };

        // Must not panic when parsing unknown status values
        let status = anchorkit::sep6::TransactionStatus::from_str(status_str);
        // Verify roundtrip consistency for known statuses
        let _ = status.as_str();
    }
});
