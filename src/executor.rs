use std::{
    collections::hash_map::Entry,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll, Waker},
};

use futures::task::{ArcWake, waker};
use fxhash::FxHashMap as HashMap;
use winit::event_loop::EventLoopProxy;

use crate::{
    app::{App, AppProxy, UserEvent},
    window::Window,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TaskId(u32);

impl TaskId {
    fn inc(&mut self) -> TaskId {
        Self(self.0.wrapping_add(1))
    }
}

struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
    waker: Waker,
}

struct TaskData {
    id: TaskId,
    event_loop: EventLoopProxy<UserEvent>,
}

impl ArcWake for TaskData {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        if arc_self
            .event_loop
            .send_event(UserEvent {
                task_id: arc_self.id,
            })
            .is_err()
        {
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
    fn new(id: TaskId, app: &AppProxy, future: Pin<Box<dyn Future<Output = ()>>>) -> Self {
        let data = Arc::new(TaskData {
            id,
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
    tasks: HashMap<TaskId, Task>,
    task_counter: TaskId,
}

impl Executor {
    fn new(app: AppProxy) -> Self {
        Self {
            app,
            tasks: HashMap::default(),
            task_counter: TaskId(0),
        }
    }

    pub fn get_free_id(&mut self) -> TaskId {
        while self.tasks.contains_key(&self.task_counter) {
            self.task_counter.inc();
        }
        let id = self.task_counter;
        self.task_counter.inc();
        id
    }

    pub fn spawn<F: Future<Output = ()> + 'static>(&mut self, future: F) -> TaskId {
        let id = self.get_free_id();
        let task = Task::new(id, &self.app, Box::pin(future));
        assert!(self.tasks.insert(id, task).is_none());
        id
    }
}

impl Executor {
    pub fn poll(&mut self, tasks: impl IntoIterator<Item = TaskId>) -> Poll<()> {
        for id in tasks {
            if let Entry::Occupied(mut entry) = self.tasks.entry(id) {
                if entry.get_mut().poll().is_ready() {
                    entry.remove();
                }
            }
        }
        if self.tasks.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    pub fn poll_all(&mut self) -> Poll<()> {
        self.tasks.retain(|_id, task| task.poll().is_pending());
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
    let task_id = executor.spawn(main(window));

    if executor.poll([task_id]).is_pending() {
        app.run(executor).unwrap();
    }
}
