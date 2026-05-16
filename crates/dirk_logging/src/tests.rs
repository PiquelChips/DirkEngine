//! Unit and integration tests for the `logging` crate.
//!
//! Test organisation:
//! - [`log_level`]   — `LogLevel` ordering, display, and conversion from `tracing::Level`.
//! - [`log_filter`]  — every `LogFilter` predicate, AND composition, and edge cases.
//! - [`log_store`]   — `LogStore` thread-safety and append-only semantics.
//! - [`store_filter`] (editor only) — `StoreFilter` execute / last / count against a live store.

#![cfg(test)]

use time::OffsetDateTime;

use crate::{LogEntry, LogLevel, filter::LogFilter};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a `LogEntry` stamped at `now_utc()`.
fn make_entry(level: LogLevel, target: &str, message: &str) -> LogEntry {
    LogEntry {
        level,
        target: target.to_string(),
        timestamp: OffsetDateTime::now_utc(),
        message: message.to_string(),
    }
}

/// Build a `LogEntry` with an explicit timestamp.
fn make_entry_at(
    level: LogLevel,
    target: &str,
    message: &str,
    timestamp: OffsetDateTime,
) -> LogEntry {
    LogEntry {
        level,
        target: target.to_string(),
        timestamp,
        message: message.to_string(),
    }
}

// ── LogLevel ──────────────────────────────────────────────────────────────────

mod log_level {
    use super::*;

    #[test]
    fn severity_ordering_is_error_first() {
        // Error is the most severe (smallest discriminant).
        assert!(LogLevel::Error < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Trace);
    }

    #[test]
    fn same_level_is_equal() {
        assert_eq!(LogLevel::Info, LogLevel::Info);
    }

    #[test]
    fn display_matches_tracing_convention() {
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
        assert_eq!(LogLevel::Warn.to_string(), "WARN");
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Trace.to_string(), "TRACE");
    }

    #[test]
    fn from_tracing_level_all_variants() {
        assert_eq!(LogLevel::from(&tracing::Level::ERROR), LogLevel::Error);
        assert_eq!(LogLevel::from(&tracing::Level::WARN), LogLevel::Warn);
        assert_eq!(LogLevel::from(&tracing::Level::INFO), LogLevel::Info);
        assert_eq!(LogLevel::from(&tracing::Level::DEBUG), LogLevel::Debug);
        assert_eq!(LogLevel::from(&tracing::Level::TRACE), LogLevel::Trace);
    }
}

// ── LogFilter ─────────────────────────────────────────────────────────────────

mod log_filter {
    use super::*;

    // ── Empty filter ──────────────────────────────────────────────────────

    #[test]
    fn empty_filter_passes_every_entry() {
        let f = LogFilter::new();
        // Representative sample of each level and a variety of targets.
        for level in [
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Trace,
        ] {
            assert!(f.filter(&make_entry(level, "AnyTarget", "any message")));
        }
    }

    // ── of_target ───────────────────────────────────────────────────────

    #[test]
    fn of_target_exact_match_passes() {
        let f = LogFilter::new().of_target("Rendering");
        assert!(f.filter(&make_entry(LogLevel::Info, "Rendering", "msg")));
    }

    #[test]
    fn of_target_different_target_fails() {
        let f = LogFilter::new().of_target("Rendering");
        assert!(!f.filter(&make_entry(LogLevel::Info, "Audio", "msg")));
    }

    #[test]
    fn of_target_substring_does_not_match() {
        // "Render" is a substring of "Rendering" but of_target requires exact equality.
        let f = LogFilter::new().of_target("Render");
        assert!(!f.filter(&make_entry(LogLevel::Info, "Rendering", "msg")));
    }

    #[test]
    fn of_target_superstring_does_not_match() {
        let f = LogFilter::new().of_target("RenderingSystem");
        assert!(!f.filter(&make_entry(LogLevel::Info, "Rendering", "msg")));
    }

    // ── target_contains ─────────────────────────────────────────────────

    #[test]
    fn target_contains_exact_passes() {
        let f = LogFilter::new().target_contains("Rendering");
        assert!(f.filter(&make_entry(LogLevel::Info, "Rendering", "msg")));
    }

    #[test]
    fn target_contains_prefix_passes() {
        let f = LogFilter::new().target_contains("Render");
        assert!(f.filter(&make_entry(LogLevel::Info, "Rendering", "msg")));
        assert!(f.filter(&make_entry(LogLevel::Info, "Rendering/Shadows", "msg")));
    }

    #[test]
    fn target_contains_unrelated_fails() {
        let f = LogFilter::new().target_contains("Physics");
        assert!(!f.filter(&make_entry(LogLevel::Info, "Rendering", "msg")));
    }

    // ── of_level ──────────────────────────────────────────────────────────

    #[test]
    fn of_level_passes_exact_level_only() {
        let f = LogFilter::new().of_level(LogLevel::Warn);

        assert!(f.filter(&make_entry(LogLevel::Warn, "C", "m"))); // exact ✓
        assert!(!f.filter(&make_entry(LogLevel::Error, "C", "m"))); // more severe ✗
        assert!(!f.filter(&make_entry(LogLevel::Info, "C", "m"))); // less severe ✗
        assert!(!f.filter(&make_entry(LogLevel::Debug, "C", "m")));
        assert!(!f.filter(&make_entry(LogLevel::Trace, "C", "m")));
    }

    // ── min_level ─────────────────────────────────────────────────────────

    #[test]
    fn min_level_warn_keeps_error_and_warn() {
        let f = LogFilter::new().min_level(LogLevel::Warn);

        assert!(f.filter(&make_entry(LogLevel::Error, "C", "m")));
        assert!(f.filter(&make_entry(LogLevel::Warn, "C", "m")));
        assert!(!f.filter(&make_entry(LogLevel::Info, "C", "m")));
        assert!(!f.filter(&make_entry(LogLevel::Debug, "C", "m")));
        assert!(!f.filter(&make_entry(LogLevel::Trace, "C", "m")));
    }

    #[test]
    fn min_level_error_keeps_only_error() {
        let f = LogFilter::new().min_level(LogLevel::Error);

        assert!(f.filter(&make_entry(LogLevel::Error, "C", "m")));
        assert!(!f.filter(&make_entry(LogLevel::Warn, "C", "m")));
        assert!(!f.filter(&make_entry(LogLevel::Info, "C", "m")));
    }

    #[test]
    fn min_level_trace_keeps_everything() {
        let f = LogFilter::new().min_level(LogLevel::Trace);

        for level in [
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Trace,
        ] {
            assert!(f.filter(&make_entry(level, "C", "m")));
        }
    }

    // ── since / until ─────────────────────────────────────────────────────

    #[test]
    fn since_excludes_older_entries() {
        let cutoff = OffsetDateTime::now_utc();
        let f = LogFilter::new().since(cutoff);

        let before = make_entry_at(
            LogLevel::Info,
            "C",
            "m",
            cutoff - time::Duration::seconds(1),
        );
        assert!(!f.filter(&before));
    }

    #[test]
    fn since_includes_equal_and_newer_entries() {
        let cutoff = OffsetDateTime::now_utc();
        let f = LogFilter::new().since(cutoff);

        let at = make_entry_at(LogLevel::Info, "C", "m", cutoff);
        let after = make_entry_at(
            LogLevel::Info,
            "C",
            "m",
            cutoff + time::Duration::seconds(1),
        );
        assert!(f.filter(&at));
        assert!(f.filter(&after));
    }

    #[test]
    fn until_excludes_newer_entries() {
        let cutoff = OffsetDateTime::now_utc();
        let f = LogFilter::new().until(cutoff);

        let after = make_entry_at(
            LogLevel::Info,
            "C",
            "m",
            cutoff + time::Duration::seconds(1),
        );
        assert!(!f.filter(&after));
    }

    #[test]
    fn until_includes_equal_and_older_entries() {
        let cutoff = OffsetDateTime::now_utc();
        let f = LogFilter::new().until(cutoff);

        let at = make_entry_at(LogLevel::Info, "C", "m", cutoff);
        let before = make_entry_at(
            LogLevel::Info,
            "C",
            "m",
            cutoff - time::Duration::seconds(1),
        );
        assert!(f.filter(&at));
        assert!(f.filter(&before));
    }

    #[test]
    fn since_and_until_form_a_closed_time_window() {
        let now = OffsetDateTime::now_utc();
        let lo = now - time::Duration::seconds(10);
        let hi = now + time::Duration::seconds(10);
        let f = LogFilter::new().since(lo).until(hi);

        // Inside the window.
        assert!(f.filter(&make_entry_at(LogLevel::Info, "C", "m", now)));
        // Before the window.
        assert!(!f.filter(&make_entry_at(
            LogLevel::Info,
            "C",
            "m",
            lo - time::Duration::seconds(1)
        )));
        // After the window.
        assert!(!f.filter(&make_entry_at(
            LogLevel::Info,
            "C",
            "m",
            hi + time::Duration::seconds(1)
        )));
    }

    // ── within_last_seconds ───────────────────────────────────────────────

    #[test]
    fn within_last_seconds_includes_recent_entry() {
        let f = LogFilter::new().within_last_seconds(60);
        let recent = make_entry_at(
            LogLevel::Info,
            "C",
            "m",
            OffsetDateTime::now_utc() - time::Duration::seconds(30),
        );
        assert!(f.filter(&recent));
    }

    #[test]
    fn within_last_seconds_excludes_old_entry() {
        let f = LogFilter::new().within_last_seconds(10);
        let old = make_entry_at(
            LogLevel::Info,
            "C",
            "m",
            OffsetDateTime::now_utc() - time::Duration::seconds(60),
        );
        assert!(!f.filter(&old));
    }

    // ── matching ──────────────────────────────────────────────────────────

    #[test]
    fn matching_substring_passes() {
        let f = LogFilter::new().matching("overflow");
        assert!(f.filter(&make_entry(LogLevel::Error, "C", "Broad-phase overflow")));
    }

    #[test]
    fn matching_full_message_passes() {
        let f = LogFilter::new().matching("exact message");
        assert!(f.filter(&make_entry(LogLevel::Info, "C", "exact message")));
    }

    #[test]
    fn matching_absent_pattern_fails() {
        let f = LogFilter::new().matching("overflow");
        assert!(!f.filter(&make_entry(LogLevel::Error, "C", "All contacts resolved")));
    }

    #[test]
    fn matching_is_case_sensitive() {
        let f = LogFilter::new().matching("Overflow");
        // Lower-case "overflow" must NOT match the pattern "Overflow".
        assert!(!f.filter(&make_entry(LogLevel::Error, "C", "overflow detected")));
    }

    // ── AND composition ───────────────────────────────────────────────────

    #[test]
    fn composed_filter_requires_all_predicates() {
        let f = LogFilter::new()
            .of_target("Rendering")
            .min_level(LogLevel::Warn);

        // Both predicates pass → included.
        assert!(f.filter(&make_entry(LogLevel::Error, "Rendering", "GPU fault")));
        assert!(f.filter(&make_entry(LogLevel::Warn, "Rendering", "slow frame")));

        // Wrong target → excluded.
        assert!(!f.filter(&make_entry(LogLevel::Error, "Audio", "clip")));

        // Level too low → excluded.
        assert!(!f.filter(&make_entry(LogLevel::Info, "Rendering", "frame ok")));
    }

    #[test]
    fn many_predicates_all_must_pass() {
        let now = OffsetDateTime::now_utc();
        let f = LogFilter::new()
            .of_target("Physics")
            .min_level(LogLevel::Warn)
            .matching("stack")
            .since(now - time::Duration::seconds(5));

        let passing = make_entry_at(LogLevel::Error, "Physics", "stack overflow in solver", now);
        assert!(f.filter(&passing));

        // Fails the target predicate.
        let wrong_cat = make_entry_at(LogLevel::Error, "Audio", "stack overflow", now);
        assert!(!f.filter(&wrong_cat));

        // Fails the message predicate.
        let wrong_msg = make_entry_at(LogLevel::Error, "Physics", "collision missed", now);
        assert!(!f.filter(&wrong_msg));

        // Fails the time predicate.
        let too_old = make_entry_at(
            LogLevel::Error,
            "Physics",
            "stack overflow",
            now - time::Duration::seconds(60),
        );
        assert!(!f.filter(&too_old));
    }
}

// ── LogStore (editor only) ────────────────────────────────────────────────────

#[cfg(feature = "editor")]
mod log_store {
    use super::*;
    use crate::store::LogStore;
    use std::sync::Arc;

    #[test]
    fn new_store_is_empty() {
        let store = LogStore::new();
        store.with_entries(|entries| assert!(entries.is_empty()));
    }

    #[test]
    fn push_appends_entry() {
        let store = LogStore::new();
        store.push(make_entry(LogLevel::Info, "Cat", "hello"));
        store.with_entries(|entries| {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].message, "hello");
            assert_eq!(entries[0].target, "Cat");
            assert_eq!(entries[0].level, LogLevel::Info);
        });
    }

    #[test]
    fn multiple_pushes_preserve_order() {
        let store = LogStore::new();
        let messages = ["first", "second", "third"];
        for msg in &messages {
            store.push(make_entry(LogLevel::Info, "Cat", msg));
        }
        store.with_entries(|entries| {
            let stored: Vec<&str> = entries.iter().map(|e| e.message.as_str()).collect();
            assert_eq!(stored, messages);
        });
    }

    #[test]
    fn with_entries_does_not_clone_the_vec() {
        // Confirm that the closure can observe all entries without us returning
        // an owned Vec — the return value is threaded through the closure.
        let store = LogStore::new();
        for i in 0..10_u32 {
            store.push(make_entry(LogLevel::Debug, "Cat", &i.to_string()));
        }
        let count = store.with_entries(|entries| entries.len());
        assert_eq!(count, 10);
    }

    #[test]
    fn concurrent_pushes_are_safe() {
        use std::thread;

        let store = Arc::new(LogStore::new());
        let n_threads = 8_usize;
        let n_per_thread = 100_usize;

        let handles: Vec<_> = (0..n_threads)
            .map(|t| {
                let s = Arc::clone(&store);
                thread::spawn(move || {
                    for i in 0..n_per_thread {
                        s.push(make_entry(LogLevel::Info, "Thread", &format!("{t}/{i}")));
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread panicked");
        }

        store.with_entries(|entries| {
            assert_eq!(entries.len(), n_threads * n_per_thread);
        });
    }
}

// ── StoreFilter (editor only) ─────────────────────────────────────────────────

#[cfg(feature = "editor")]
mod store_filter {
    use super::*;
    use crate::store::LogStore;
    use std::sync::Arc;

    /// A store pre-populated with a representative set of entries.
    ///
    /// Insertion order (chronological):
    /// 0. Error   / Rendering / "GPU crash"
    /// 1. Warn    / Audio     / "Buffer underrun"
    /// 2. Info    / Rendering / "Frame complete"
    /// 3. Debug   / Physics   / "Collision resolved"
    /// 4. Trace   / Rendering / "Shader compiled"
    fn fixture() -> Arc<LogStore> {
        let store = Arc::new(LogStore::new());
        let entries = [
            (LogLevel::Error, "Rendering", "GPU crash"),
            (LogLevel::Warn, "Audio", "Buffer underrun"),
            (LogLevel::Info, "Rendering", "Frame complete"),
            (LogLevel::Debug, "Physics", "Collision resolved"),
            (LogLevel::Trace, "Rendering", "Shader compiled"),
        ];
        for (level, cat, msg) in &entries {
            store.push(make_entry(*level, cat, msg));
        }
        store
    }

    // ── execute ───────────────────────────────────────────────────────────

    #[test]
    fn execute_empty_filter_returns_all_entries() {
        let store = fixture();
        let results = LogFilter::new().with_store(Arc::clone(&store)).execute();
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn execute_preserves_chronological_order() {
        let store = fixture();
        let results = LogFilter::new().with_store(Arc::clone(&store)).execute();
        for window in results.windows(2) {
            assert!(window[0].timestamp <= window[1].timestamp);
        }
    }

    #[test]
    fn execute_filters_by_target() {
        let store = fixture();
        let results = LogFilter::new()
            .of_target("Rendering")
            .with_store(Arc::clone(&store))
            .execute();
        // Entries 0, 2, 4 are Rendering.
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|e| e.target == "Rendering"));
    }

    #[test]
    fn execute_filters_by_min_level() {
        let store = fixture();
        // Error and Warn only.
        let results = LogFilter::new()
            .min_level(LogLevel::Warn)
            .with_store(Arc::clone(&store))
            .execute();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.level <= LogLevel::Warn));
    }

    #[test]
    fn execute_combined_filter() {
        let store = fixture();
        // Rendering + Warn or worse → only entry 0 ("GPU crash").
        let results = LogFilter::new()
            .of_target("Rendering")
            .min_level(LogLevel::Warn)
            .with_store(Arc::clone(&store))
            .execute();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].message, "GPU crash");
    }

    #[test]
    fn execute_no_match_returns_empty_vec() {
        let store = fixture();
        let results = LogFilter::new()
            .of_target("NonExistentSystem")
            .with_store(Arc::clone(&store))
            .execute();
        assert!(results.is_empty());
    }

    // ── last ──────────────────────────────────────────────────────────────

    #[test]
    fn last_returns_tail_in_chronological_order() {
        let store = fixture();
        // 5 entries total; last(3) should give entries 2, 3, 4.
        let results = LogFilter::new().with_store(Arc::clone(&store)).last(3);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].message, "Frame complete");
        assert_eq!(results[1].message, "Collision resolved");
        assert_eq!(results[2].message, "Shader compiled");
    }

    #[test]
    fn last_more_than_total_returns_all() {
        let store = fixture();
        let results = LogFilter::new().with_store(Arc::clone(&store)).last(100);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn last_zero_returns_empty() {
        let store = fixture();
        let results = LogFilter::new().with_store(Arc::clone(&store)).last(0);
        assert!(results.is_empty());
    }

    #[test]
    fn last_with_filter_operates_on_matching_subset() {
        let store = fixture();
        // Rendering entries in order: GPU crash (0), Frame complete (2), Shader compiled (4).
        // last(2) of that subset → [Frame complete, Shader compiled].
        let results = LogFilter::new()
            .of_target("Rendering")
            .with_store(Arc::clone(&store))
            .last(2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].message, "Frame complete");
        assert_eq!(results[1].message, "Shader compiled");
    }

    // ── count ─────────────────────────────────────────────────────────────

    #[test]
    fn count_all_entries() {
        let store = fixture();
        let n = LogFilter::new().with_store(Arc::clone(&store)).count();
        assert_eq!(n, 5);
    }

    #[test]
    fn count_by_target() {
        let store = fixture();
        let n = LogFilter::new()
            .of_target("Rendering")
            .with_store(Arc::clone(&store))
            .count();
        assert_eq!(n, 3);
    }

    #[test]
    fn count_no_match_is_zero() {
        let store = fixture();
        let n = LogFilter::new()
            .of_target("DoesNotExist")
            .with_store(Arc::clone(&store))
            .count();
        assert_eq!(n, 0);
    }

    #[test]
    fn count_matches_execute_len() {
        // count() and execute().len() must always agree for the same filter.
        let store = fixture();
        let expected = LogFilter::new()
            .min_level(LogLevel::Info)
            .with_store(Arc::clone(&store))
            .execute()
            .len();
        let actual = LogFilter::new()
            .min_level(LogLevel::Info)
            .with_store(Arc::clone(&store))
            .count();
        assert_eq!(actual, expected);
    }
}
