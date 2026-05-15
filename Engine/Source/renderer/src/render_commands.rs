//! This crate has the render commands system. It allows threads
//! to submit functions that will be run on the renderer. This
//! notable allows systems to interact with the renderer.
//!
//! These types are private as they are for internal renderer use
//! only. They should not be used by other engine systems.
use std::sync::mpsc::{self, Receiver, Sender};

use crate::{Renderer, Result};

type RenderCommand = Box<dyn FnOnce(&mut Renderer) -> Result<()> + Send + 'static>;

pub struct RenderCommandSender {
    tx: Sender<RenderCommand>,
}

impl RenderCommandSender {
    pub fn enqueue_command<F>(&self, command: F)
    where
        F: FnOnce(&mut Renderer) -> Result<()> + Send + 'static,
    {
        // The only real failure here is a disconnected channel (receiver was
        // dropped), which generally means the render thread has shut down.
        // Silently dropping the command is the safest thing to do.
        let _ = self.tx.send(Box::new(command));
    }
}

// RenderCommandSender is cheap to clone — each clone shares the same channel.
impl Clone for RenderCommandSender {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
        }
    }
}

pub struct RenderCommandReceiver {
    rx: Receiver<RenderCommand>,
}

impl RenderCommandReceiver {
    pub fn flush(&self, renderer: &mut Renderer) -> Result<()> {
        // `try_recv` is non-blocking: we consume whatever is in the queue
        // right now and return immediately when it's empty, so the render
        // thread is never stalled waiting for the game thread.
        while let Ok(command) = self.rx.try_recv() {
            command(renderer)?;
        }
        Ok(())
    }
}

pub fn channel() -> (RenderCommandSender, RenderCommandReceiver) {
    let (tx, rx) = mpsc::channel();
    (RenderCommandSender { tx }, RenderCommandReceiver { rx })
}
