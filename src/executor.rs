use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use crate::{
    app::{App, AppProxy},
    window::Window,
};

struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
    waker: Waker,
}

impl Task {
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
        self.tasks.push(Task {
            future: Box::pin(future),
            // TODO: On `wake` send UserEvent in App
            waker: Waker::noop().clone(),
        });
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
