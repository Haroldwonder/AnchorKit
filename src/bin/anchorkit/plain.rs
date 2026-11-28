//! Plain output (NO_COLOR / non-TTY / `--plain`), recovered from #1609 (issue #1497).

use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by `--plain` / `--no-emoji`.
pub static FORCE_PLAIN: AtomicBool = AtomicBool::new(false);

/// Whether output written to stdout (`to_stdout`) or stderr should be plain.
///
/// Plain output is used when `--plain`/`--no-emoji` was passed, when the
/// `NO_COLOR` environment variable is set to a non-empty value
/// (https://no-color.org), or when the target stream is not a terminal.
fn plain_output(to_stdout: bool) -> bool {
    if FORCE_PLAIN.load(Ordering::Relaxed) {
        return true;
    }
    if std::env::var_os("NO_COLOR").map_or(false, |v| !v.is_empty()) {
        return true;
    }
    if to_stdout {
        !std::io::stdout().is_terminal()
    } else {
        !std::io::stderr().is_terminal()
    }
}

/// Emoji and pictographic symbols that are dropped entirely in plain mode.
fn is_decorative_char(c: char) -> bool {
    matches!(
        c as u32,
        0x1F000..=0x1FAFF // emoji & pictographs (🔍 🏥 💡 📋 ...)
            | 0x2600..=0x27BF // misc symbols & dingbats
            | 0x2300..=0x23FF // misc technical (⏱)
            | 0x25A0..=0x25FF // geometric shapes (◈)
            | 0xFE0F          // emoji variation selector
            | 0x200D          // zero-width joiner
    )
}

/// Replace status glyphs with ASCII labels, box-drawing characters with ASCII
/// equivalents, and strip any remaining emoji.
fn to_plain_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        let replacement = match c {
            '✅' | '✔' => Some("[OK]"),
            '❌' => Some("[ERROR]"),
            '✖' | '✗' => Some("[FAIL]"),
            '⚠' => Some("[WARN]"),
            'ℹ' => Some("[INFO]"),
            '•' => Some("-"),
            '→' => Some("->"),
            '─' => Some("-"),
            '│' => Some("|"),
            '┌' | '├' | '└' => Some("+"),
            _ => None,
        };
        if let Some(r) = replacement {
            out.push_str(r);
        } else if is_decorative_char(c) {
            // Drop the glyph together with the single space that follows it,
            // so "🔍 Running" becomes "Running".
            if chars.peek() == Some(&' ') {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Apply plain-mode filtering to a formatted message if required.
pub fn render_output(s: String, to_stdout: bool) -> String {
    if plain_output(to_stdout) {
        to_plain_text(&s)
    } else {
        s
    }
}

// The macros below shadow std's printing macros for every module declared
// after `#[macro_use] mod plain;` in main.rs, so every message honours
// NO_COLOR, non-TTY streams and `--plain` without touching each call site.
macro_rules! println {
    () => { std::println!() };
    ($($arg:tt)*) => {
        std::println!("{}", crate::plain::render_output(format!($($arg)*), true))
    };
}

macro_rules! eprintln {
    () => { std::eprintln!() };
    ($($arg:tt)*) => {
        std::eprintln!("{}", crate::plain::render_output(format!($($arg)*), false))
    };
}

macro_rules! print {
    ($($arg:tt)*) => {
        std::print!("{}", crate::plain::render_output(format!($($arg)*), true))
    };
}

macro_rules! eprint {
    ($($arg:tt)*) => {
        std::eprint!("{}", crate::plain::render_output(format!($($arg)*), false))
    };
}
