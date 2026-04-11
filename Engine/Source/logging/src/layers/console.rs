use tracing::{Event, Level, Subscriber};
use tracing_subscriber::{Layer, layer::Context};

use super::format::{extract_event_data, format_level, format_line, format_timestamp};

/// A [`tracing_subscriber::Layer`] that immediately formats events and
/// prints them to the terminal with ANSI color codes.
///
/// Output routing (preserved from the original logger):
/// - `ERROR` → **stdout** (`println!`)
/// - All other levels → **stderr** (`eprintln!`)
pub struct ConsoleLayer;

impl<S: Subscriber> Layer<S> for ConsoleLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let (message, category, timestamp) = extract_event_data(event);
        let level = event.metadata().level();

        let ts_str = format_timestamp(&timestamp);
        let level_str = format_level(level, /* colored = */ true);
        let line = format_line(&ts_str, &level_str, &category, &message);

        // TODO: not this:
        // Errors go to stdout so they can be piped/captured separately;
        // everything else goes to stderr to avoid polluting stdout.
        if *level == Level::ERROR {
            println!("{line}");
        } else {
            eprintln!("{line}");
        }
    }
}
