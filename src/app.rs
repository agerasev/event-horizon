use std::{cell::RefCell, ops::ControlFlow, rc::Rc};

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow as EventLoopControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[derive(Default, Debug)]
pub struct AppState {
    pub redraw_requested: bool,
    pub close_requested: bool,
}

pub type AppHandle = Rc<RefCell<AppState>>;

#[derive(Default)]
pub struct App {
    window: Option<Window>,
    state: Rc<RefCell<AppState>>,
    poll: Option<Box<dyn FnMut() -> ControlFlow<()>>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let mut state = self.state.borrow_mut();
        match event {
            WindowEvent::CloseRequested => {
                println!("Close requested");
                state.close_requested = true;
            }
            WindowEvent::RedrawRequested => {
                println!("Redraw requested");
                state.redraw_requested = true;
            }
            _ => (),
        }
        drop(state);

        if let Some(poll) = &mut self.poll {
            match poll() {
                ControlFlow::Continue(()) => (),
                ControlFlow::Break(()) => event_loop.exit(),
            }
        }
    }
}

impl App {
    pub fn handle(&self) -> AppHandle {
        self.state.clone()
    }

    pub fn run<F: FnMut() -> ControlFlow<()> + 'static>(
        &mut self,
        poll: F,
    ) -> Result<(), EventLoopError> {
        self.poll = Some(Box::new(poll));
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(EventLoopControlFlow::Poll);
        event_loop.run_app(self)
    }
}
