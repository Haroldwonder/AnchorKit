//! Output formatting shared by several subcommands.

// ── ascii table renderer ─────────────────────────────────────────────────────

/// Renders a simple ASCII table from headers and rows.
///
/// Each inner `Vec<String>` is one row; the number of elements must match
/// `headers.len()`.  Column widths are computed automatically.
pub fn print_ascii_table(headers: &[&str], rows: &[Vec<String>]) {
    // Compute column widths: max of header width and all cell widths.
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    let separator = {
        let parts: Vec<String> = widths.iter().map(|w| "-".repeat(w + 2)).collect();
        format!("+{}+", parts.join("+"))
    };

    // Header row
    println!("{}", separator);
    let header_row: Vec<String> = headers
        .iter()
        .zip(widths.iter())
        .map(|(h, w)| format!(" {:w$} ", h, w = w))
        .collect();
    println!("|{}|", header_row.join("|"));
    println!("{}", separator);

    // Data rows
    for row in rows {
        let cells: Vec<String> = row
            .iter()
            .zip(widths.iter())
            .map(|(c, w)| format!(" {:w$} ", c, w = w))
            .collect();
        println!("|{}|", cells.join("|"));
    }
    println!("{}", separator);
}

// ── output format validation ──────────────────────────────────────────────────

/// Returns `true` if the format string is one of the three accepted values.
/// Prints an error and exits on invalid input.
pub fn validate_output_format(output: &str) {
    if !matches!(output, "json" | "text" | "table") {
        eprintln!("❌ Invalid output format '{}'. Use 'json', 'text', or 'table'", output);
        std::process::exit(1);
    }
}

// ── timestamps ────────────────────────────────────────────────────────────────

pub fn format_timestamp(ts: u64) -> String {
    // ts is Unix timestamp in seconds
    let secs_per_day = 86400;
    let secs_per_hour = 3600;
    let secs_per_minute = 60;

    let days = ts / secs_per_day;
    let hours = (ts % secs_per_day) / secs_per_hour;
    let minutes = (ts % secs_per_hour) / secs_per_minute;
    let seconds = ts % secs_per_minute;

    format!("{}d {:02}h {:02}m {:02}s", days, hours, minutes, seconds)
}
