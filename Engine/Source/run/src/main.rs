//! Simple winit window example.

use std::error::Error;

use tracing::{error, info};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[derive(Default, Debug)]
struct App {
    window: Option<Box<dyn Window>>,
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        let window_attributes = WindowAttributes::default();
        self.window = match event_loop.create_window(window_attributes) {
            Ok(window) => Some(window),
            Err(err) => {
                error!("error creating window: {err}");
                event_loop.exit();
                return;
            }
        }
    }

    fn window_event(&mut self, event_loop: &dyn ActiveEventLoop, _: WindowId, event: WindowEvent) {
        info!("{event:?}");
        match event {
            WindowEvent::CloseRequested => {
                info!("Close was requested; stopping");
                event_loop.exit();
            }
            WindowEvent::SurfaceResized(_) => {
                self.window
                    .as_ref()
                    .expect("resize event without a window")
                    .request_redraw();
            }
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                let window = self
                    .window
                    .as_ref()
                    .expect("redraw request without a window");

                // Notify that you're about to draw.
                window.pre_present_notify();

                // For contiguous redraw loop you can request a redraw from here.
                // window.request_redraw();
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;

    // For alternative loop run options see `pump_events` and `run_on_demand` examples.
    event_loop.run_app(App::default())?;

    Ok(())
}
