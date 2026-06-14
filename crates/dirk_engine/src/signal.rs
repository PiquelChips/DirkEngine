//! Operating system signal integration.
//!
//! The signal handler installed by this module does not touch engine state from
//! inside the OS signal context. `signal-hook` writes notifications into a pipe,
//! a background thread forwards them to this subsystem, and the subsystem asks
//! the engine to exit during the normal tick flow.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread::{self, JoinHandle},
};

use dirk_universe::Universe;
use signal_hook::{
    iterator::{Handle as SignalIteratorHandle, Signals},
    low_level::{emulate_default_handler, signal_name},
};
use tracing::{debug, error, info, warn};

use crate::{EngineBuilder, EngineHandle, EnginePlugin, Subsystem};

#[cfg(windows)]
use signal_hook::consts::signal::{SIGABRT, SIGBREAK, SIGINT, SIGTERM};
#[cfg(not(windows))]
use signal_hook::consts::signal::{SIGABRT, SIGHUP, SIGINT, SIGQUIT, SIGTERM};

/// Registers operating system signal handlers with the engine.
///
/// The first handled signal requests graceful engine shutdown. Handled signals
/// include `SIGINT`, `SIGTERM`, and `SIGABRT`, plus platform-specific console
/// termination signals where available.
pub struct OperatingSystemSignalPlugin;

impl EnginePlugin for OperatingSystemSignalPlugin {
    fn name(&self) -> &'static str {
        "operating-system-signals"
    }

    fn build(&self, builder: &mut EngineBuilder) -> anyhow::Result<()> {
        builder.add_subsystem(|_ctx| OperatingSystemSignalSubsystem::install());
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OperatingSystemSignal {
    number: i32,
}

impl OperatingSystemSignal {
    fn new(number: i32) -> Self {
        Self { number }
    }

    fn name(self) -> &'static str {
        signal_name(self.number).unwrap_or("unknown signal")
    }
}

struct SignalListener {
    handle: SignalIteratorHandle,
    thread: Option<JoinHandle<()>>,
}

impl SignalListener {
    fn install(sender: mpsc::Sender<OperatingSystemSignal>) -> anyhow::Result<Self> {
        let mut signals = Signals::new(handled_signals())?;
        let handle = signals.handle();
        let shutdown_requested = AtomicBool::new(false);
        let thread = thread::Builder::new()
            .name("dirk-os-signals".to_owned())
            .spawn(move || {
                for signal in signals.forever() {
                    if shutdown_requested.swap(true, Ordering::SeqCst) {
                        warn!(
                            signal,
                            name = OperatingSystemSignal::new(signal).name(),
                            "additional operating system signal received; using default handler",
                        );
                        if let Err(err) = emulate_default_handler(signal) {
                            error!(
                                signal,
                                error = %err,
                                "failed to emulate default signal handler; exiting process",
                            );
                        }
                        std::process::exit(128 + signal);
                    }

                    if sender.send(OperatingSystemSignal::new(signal)).is_err() {
                        break;
                    }
                }
            })?;

        Ok(Self {
            handle,
            thread: Some(thread),
        })
    }

    fn shutdown(&mut self) {
        self.handle.close();
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            error!("operating system signal listener thread panicked");
        }
    }
}

impl Drop for SignalListener {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct OperatingSystemSignalSubsystem {
    receiver: Receiver<OperatingSystemSignal>,
    listener: Option<SignalListener>,
}

impl OperatingSystemSignalSubsystem {
    fn install() -> anyhow::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let listener = SignalListener::install(sender)?;

        info!(signals = ?handled_signals(), "installed operating system signal handlers");
        Ok(Self {
            receiver,
            listener: Some(listener),
        })
    }

    #[cfg(test)]
    fn from_receiver(receiver: Receiver<OperatingSystemSignal>) -> Self {
        Self {
            receiver,
            listener: None,
        }
    }
}

impl Subsystem for OperatingSystemSignalSubsystem {
    fn name(&self) -> &'static str {
        "operating-system-signals"
    }

    fn tick(
        &mut self,
        _delta_time: f64,
        handle: &EngineHandle,
        _universe: &mut Universe,
    ) -> anyhow::Result<()> {
        if let Some(signal) = self.receiver.try_iter().next() {
            warn!(
                signal = signal.number,
                name = signal.name(),
                "operating system signal received; requesting engine shutdown",
            );
            handle.exit();
        }

        Ok(())
    }

    fn shutdown(&mut self, _handle: &EngineHandle, _universe: &mut Universe) -> anyhow::Result<()> {
        if let Some(mut listener) = self.listener.take() {
            listener.shutdown();
            debug!("shut down operating system signal listener");
        }

        Ok(())
    }
}

#[cfg(not(windows))]
fn handled_signals() -> &'static [i32] {
    &[SIGINT, SIGTERM, SIGABRT, SIGHUP, SIGQUIT]
}

#[cfg(windows)]
fn handled_signals() -> &'static [i32] {
    &[SIGINT, SIGTERM, SIGABRT, SIGBREAK]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EngineStatus;

    #[test]
    fn received_signal_requests_engine_exit() -> anyhow::Result<()> {
        let (sender, receiver) = mpsc::channel();
        sender.send(OperatingSystemSignal::new(SIGINT))?;

        let mut engine = crate::tests::engine_with_subsystems(vec![Box::new(
            OperatingSystemSignalSubsystem::from_receiver(receiver),
        )]);

        assert_eq!(engine.tick()?, EngineStatus::ExitRequested);
        engine.shutdown()?;
        Ok(())
    }
}
