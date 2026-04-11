#![cfg(test)]

use super::*;

/// Verify that the timestamp format string compiles without panicking.
#[test]
fn timestamp_format_compiles() {
    let ts = time::OffsetDateTime::now_utc();
    let s = layers::format::format_timestamp(&ts);
    assert!(!s.is_empty());
}

/// Verify the level-bracket format without colors.
#[test]
fn level_format_plain() {
    let s = layers::format::format_level(&tracing::Level::WARN, false);
    assert_eq!(s, "[WARN]");
}

/// Verify the level-bracket format with colors contains the level name.
#[test]
fn level_format_colored() {
    let s = layers::format::format_level(&tracing::Level::ERROR, true);
    assert!(s.contains("ERROR"));
    assert!(s.contains('\x1b'));
}

/// Verify the assembled log line structure.
#[test]
fn format_line_structure() {
    let line =
        layers::format::format_line("2024/01/15 12:34:56", "[INFO]", "Rendering", "Mesh loaded");
    assert_eq!(line, "2024/01/15 12:34:56 [INFO] [Rendering] Mesh loaded");
}

/// Smoke test: `LogLevel` ordering (`Error` < `Warn` < … < `Trace`).
#[test]
fn log_level_ordering() {
    assert!(LogLevel::Error < LogLevel::Warn);
    assert!(LogLevel::Warn < LogLevel::Info);
    assert!(LogLevel::Info < LogLevel::Debug);
    assert!(LogLevel::Debug < LogLevel::Trace);
}
