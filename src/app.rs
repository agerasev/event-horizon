use std::{cell::RefCell, rc::Rc, task::Poll};

use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

use crate::executor::Executor;

pub struct UserEvent {}

#[derive(Default, Debug)]
pub struct AppState {
    pub redraw_requested: bool,
    pub close_requested: bool,
}

#[derive(Clone)]
pub struct AppProxy {
    pub state: Rc<RefCell<AppState>>,
    pub event_loop: EventLoopProxy<UserEvent>,
}

pub struct App {
    event_loop: EventLoop<UserEvent>,
    state: Rc<RefCell<AppState>>,
}

struct AppRuntime {
    window: Option<Window>,
    state: Rc<RefCell<AppState>>,
    executor: Executor,
}

impl App {
    pub fn new() -> Self {
        let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
        Self {
            event_loop,
            state: Default::default(),
        }
    }

    pub fn handle(&self) -> AppProxy {
        AppProxy {
            state: self.state.clone(),
            event_loop: self.event_loop.create_proxy(),
        }
    }

    pub fn run(self, executor: Executor) -> Result<(), EventLoopError> {
        let mut app = AppRuntime {
            window: None,
            state: self.state,
            executor,
        };
        self.event_loop.set_control_flow(ControlFlow::Poll);
        self.event_loop.run_app(&mut app)
    }
}

impl ApplicationHandler<UserEvent> for AppRuntime {
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

        match self.executor.poll() {
            Poll::Pending => (),
            Poll::Ready(()) => event_loop.exit(),
        }
    }
}
