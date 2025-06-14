use std::{
    cell::RefCell,
    collections::{BTreeMap, hash_map::Entry},
    future::Future,
    mem,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::Instant,
};

use futures::task::{ArcWake, waker};
use fxhash::FxHashMap as HashMap;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};

use crate::{
    app::{App, AppProxy, UserEvent},
    runtime::Runtime,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
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

#[derive(Default)]
pub(crate) struct Timer {
    pub waker: Option<Waker>,
}

pub(crate) struct Executor {
    app: AppProxy,
    tasks: HashMap<TaskId, Task>,
    task_counter: TaskId,
    timers: BTreeMap<Instant, Rc<RefCell<Timer>>>,
}

impl Executor {
    fn new(app: AppProxy) -> Self {
        Self {
            app,
            tasks: HashMap::default(),
            task_counter: TaskId(0),
            timers: BTreeMap::default(),
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

    pub fn add_timer(&mut self, timestamp: Instant) -> Rc<RefCell<Timer>> {
        self.timers.entry(timestamp).or_default().clone()
    }

    fn wake_timers(&mut self) {
        let mut old_timers = self.timers.split_off(&Instant::now());
        mem::swap(&mut self.timers, &mut old_timers);
        for timer in old_timers.into_values() {
            if let Some(waker) = timer.borrow_mut().waker.take() {
                waker.wake();
            }
        }
    }

    pub fn poll(
        &mut self,
        event_loop: &ActiveEventLoop,
        tasks: impl IntoIterator<Item = TaskId>,
    ) -> Poll<()> {
        for id in tasks {
            if let Entry::Occupied(mut entry) = self.tasks.entry(id) {
                if entry.get_mut().poll().is_ready() {
                    entry.remove();
                }
            }
        }

        self.wake_timers();
        if let Some(entry) = self.timers.first_entry() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(*entry.key()))
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }

        if self.tasks.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

pub fn enter<F: AsyncFnOnce(Runtime) + 'static>(main: F) {
    let app = App::new();

    let executor = Rc::new(RefCell::new(Executor::new(app.proxy())));

    let runtime = Runtime::new(executor.clone(), app.proxy());

    executor.borrow_mut().spawn(main(runtime));

    app.run(executor).unwrap();
}
