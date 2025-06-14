use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use futures::task::{ArcWake, waker};
use winit::event_loop::EventLoopProxy;

use crate::{
    app::{App, AppProxy, UserEvent},
    window::Window,
};

struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
    waker: Waker,
}

struct TaskData {
    event_loop: EventLoopProxy<UserEvent>,
}

impl ArcWake for TaskData {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        if arc_self.event_loop.send_event(UserEvent {}).is_err() {
            panic!("Event loop closed");
        }
    }
}

impl Drop for TaskData {
    fn drop(&mut self) {
        println!("Task data dropped");
    }
}

impl Task {
    fn new(app: &AppProxy, future: Pin<Box<dyn Future<Output = ()>>>) -> Self {
        let data = Arc::new(TaskData {
            event_loop: app.event_loop.clone(),
        });
        Self {
            future,
            waker: waker(data),
        }
    }

    fn poll(&mut self) -> Poll<()> {
        println!("Poll");
        let mut cx = Context::from_waker(&self.waker);
        match self.future.as_mut().poll(&mut cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(()) => Poll::Ready(()),
        }
    }
}

pub(crate) struct Executor {
    app: AppProxy,
    tasks: Vec<Task>,
}

impl Executor {
    fn new(app: AppProxy) -> Self {
        Self {
            app,
            tasks: Vec::new(),
        }
    }

    pub fn spawn<F: Future<Output = ()> + 'static>(&mut self, future: F) {
        self.tasks.push(Task::new(&self.app, Box::pin(future)));
    }
}

impl Executor {
    pub fn poll(&mut self) -> Poll<()> {
        // TODO: Route to specific task
        self.tasks.retain_mut(|task| task.poll().is_pending());
        if self.tasks.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub fn enter<F: AsyncFnOnce(Window) + 'static>(main: F) {
    let app = App::new();
    let window = Window::new(app.handle());

    let mut executor = Executor::new(app.handle());
    executor.spawn(main(window));

    if executor.poll().is_pending() {
        app.run(executor).unwrap();
    }
}
