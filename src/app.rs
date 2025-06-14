use std::{
    cell::RefCell,
    rc::Rc,
    task::{Poll, Waker},
};

use fxhash::FxHashSet as HashSet;
use winit::{
    application::ApplicationHandler,
    error::EventLoopError,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

use crate::executor::{Executor, TaskId};

#[derive(Debug)]
pub struct UserEvent {
    pub task_id: TaskId,
}

#[derive(Default, Debug)]
pub struct AppState {
    pub window: Option<Window>,
    pub redraw_waker: Option<Waker>,
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
    state: Rc<RefCell<AppState>>,
    executor: Executor,
    tasks_to_poll: HashSet<TaskId>,
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
            state: self.state,
            executor,
            tasks_to_poll: HashSet::default(),
        };
        self.event_loop.set_control_flow(ControlFlow::Wait);
        self.event_loop.run_app(&mut app)
    }
}

impl ApplicationHandler<UserEvent> for AppRuntime {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state.borrow_mut().window = Some(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let mut state = self.state.borrow_mut();
        match event {
            WindowEvent::CloseRequested => {
                println!("Close requested");
                state.close_requested = true;
                if let Some(waker) = state.redraw_waker.take() {
                    waker.wake()
                }
            }
            WindowEvent::RedrawRequested => {
                println!("Redraw requested");
                state.redraw_requested = true;
                if let Some(waker) = state.redraw_waker.take() {
                    waker.wake()
                }
            }
            _ => (),
        }
        drop(state);
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        println!("User event: {event:?}");
        self.tasks_to_poll.insert(event.task_id);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let poll = self.executor.poll(self.tasks_to_poll.iter().copied());
        self.tasks_to_poll.clear();
        match poll {
            Poll::Pending => (),
            Poll::Ready(()) => event_loop.exit(),
        }
    }
}
