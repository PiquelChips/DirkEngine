# logging

Engine logging built on [`tracing`], with structured targets, ANSI
console output, rotating file output, and (in editor builds) a queryable
in-memory store.

## Initialisation

Call [`Logger::new`] once at startup. It installs the global
[`tracing`] subscriber, so calling it a second time returns
[`InitError::AlreadyInitialised`].

```rust
// Verbose mode enables DEBUG and TRACE levels; pass `false` for INFO+.
let logger = dirk_logging::Logger::new().verbose(true).init().expect("logger init failed");
```

## Emitting log events

Use standard [`tracing`] macros. Assign a target via the `target:`
directive (falls back to the Rust module path):

```rust
# let id = 5;
# let frame = 5;
tracing::error!(target: "Physics", "Broad-phase overflow");
tracing::warn!(target: "Audio",   "Buffer underrun on stream {}", id);
tracing::info!(target: "Rendering", "Frame {} complete", frame);
```

## Querying the log store (editor builds only)

In editor builds the [`Logger`] holds a shared `LogStore` that
captures every event. Use [`Logger::query`] with a [`Filter`] to search it:

```rust
# let logger = dirk_logging::Logger::new().init().unwrap();
use dirk_logging::{Filter, LogLevel};

// The 50 most recent warnings or worse from the Rendering target:
# #[cfg(feature = "editor")]
let entries = logger
    .query(
        Filter::new()
            .of_target("Rendering")
            .min_level(LogLevel::Warn),
    )
    .last(50);

// Count all errors since the session started:
# #[cfg(feature = "editor")]
let error_count = logger
    .query(Filter::new().of_level(LogLevel::Error))
    .count();
```

## Target convention

Prefer short, stable target names that map to engine subsystems
(`"Rendering"`, `"Physics"`, `"Audio"`, …). Hierarchical names like
`"Rendering::Shadows"` work well with [`Filter::target_contains`].
