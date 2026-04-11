use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::Mutex,
};

use tracing::{Event, Subscriber};
use tracing_subscriber::{Layer, layer::Context};

use super::format::{
    extract_event_data, format_level, format_line, format_timestamp, format_timestamp_filename,
};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// FileLayer
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A [`tracing_subscriber::Layer`] that writes **plain-text** (no ANSI codes)
/// log lines to two files simultaneously:
///
/// | File | Behaviour |
/// |------|-----------|
/// | `<log_dir>/YYYY-MM-DD_HH-MM-SS.log` | Created fresh each run; never truncated |
/// | `<log_dir>/latest.log` | **Truncated** at startup so it always reflects the current session |
pub struct FileLayer {
    /// Both file handles are guarded by a single `Mutex` so that writes to the
    /// two files are always in lock-step (no interleaving across threads).
    writers: Mutex<FileWriters>,
}

struct FileWriters {
    timestamped: File,
    latest: File,
}

impl FileLayer {
    /// Initialise the layer, creating `log_dir` if needed and opening both
    /// output files.
    ///
    /// Returns an error if the directory cannot be created or either file
    /// cannot be opened.
    pub fn new(log_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let dir = log_dir.as_ref();
        std::fs::create_dir_all(dir)?;

        let ts = format_timestamp_filename(&time::OffsetDateTime::now_utc());

        // Append to the timestamped archive (so multiple runs on the same
        // second don't clobber each other, though that is unlikely).
        let timestamped = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join(format!("{ts}.log")))?;

        // Truncate latest.log so it only contains the current session.
        let latest = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dir.join("latest.log"))?;

        Ok(Self {
            writers: Mutex::new(FileWriters {
                timestamped,
                latest,
            }),
        })
    }
}

impl<S: Subscriber> Layer<S> for FileLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let (message, category, timestamp) = extract_event_data(event);
        let level = event.metadata().level();

        let ts_str = format_timestamp(&timestamp);
        // `colored = false` — ANSI escape codes must not appear in log files.
        let level_str = format_level(level, /* colored = */ false);
        let line = format_line(&ts_str, &level_str, &category, &message);

        // Best-effort writes: silently ignore I/O errors so logging never
        // panics the application.
        if let Ok(mut w) = self.writers.lock() {
            let _ = writeln!(w.timestamped, "{line}");
            let _ = writeln!(w.latest, "{line}");
        }
    }
}
