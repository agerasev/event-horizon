use std::{
    cell::{Cell, RefCell},
    cmp::Reverse,
    collections::{BinaryHeap, binary_heap::PeekMut, hash_map::Entry},
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Waker},
    time::Instant,
};

use futures::task::{ArcWake, waker};
use fxhash::{FxHashMap as HashMap, FxHashSet as HashSet};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};

type FutureObj = Pin<Box<dyn Future<Output = ()>>>;

use crate::app::UserEvent;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub struct TaskId(u64);

impl TaskId {
    fn get_and_inc(&mut self) -> TaskId {
        let id = *self;
        self.0 = self.0.wrapping_add(1);
        id
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

impl Task {
    fn new(
        id: TaskId,
        event_loop: EventLoopProxy<UserEvent>,
        future: Pin<Box<dyn Future<Output = ()>>>,
    ) -> Self {
        let data = Arc::new(TaskData { id, event_loop });
        Self {
            future,
            waker: waker(data),
        }
    }

    fn poll(&mut self) -> Poll<()> {
        let mut cx = Context::from_waker(&self.waker);
        match self.future.as_mut().poll(&mut cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(()) => Poll::Ready(()),
        }
    }
}

#[derive(Clone)]
pub struct Timer {
    pub timestamp: Instant,
    pub waker: Rc<Cell<Option<Waker>>>,
}

impl PartialEq for Timer {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.timestamp.eq(&other.timestamp)
    }
}

impl Eq for Timer {}

impl PartialOrd for Timer {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timer {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

pub struct Executor {
    event_loop: EventLoopProxy<UserEvent>,
    tasks: HashMap<TaskId, Task>,
    tasks_to_poll: HashSet<TaskId>,
    timers: BinaryHeap<Reverse<Timer>>,
    proxy: Rc<RefCell<ExecutorProxy>>,
}

#[derive(Default)]
pub struct ExecutorProxy {
    task_counter: TaskId,
    new_tasks: Vec<(TaskId, FutureObj)>,
    new_timers: Vec<Timer>,
}

impl Executor {
    pub fn new(event_loop: EventLoopProxy<UserEvent>) -> Self {
        Self {
            event_loop,
            tasks: HashMap::default(),
            tasks_to_poll: HashSet::default(),
            timers: BinaryHeap::default(),
            proxy: Rc::new(RefCell::new(ExecutorProxy::default())),
        }
    }

    pub fn proxy(&self) -> Rc<RefCell<ExecutorProxy>> {
        self.proxy.clone()
    }

    fn sync_proxy(&mut self) {
        let mut proxy = self.proxy.borrow_mut();

        for (id, future) in proxy.new_tasks.drain(..) {
            let task = Task::new(id, self.event_loop.clone(), future);
            assert!(self.tasks.insert(id, task).is_none());
            self.tasks_to_poll.insert(id);
        }

        for timer in proxy.new_timers.drain(..) {
            self.timers.push(Reverse(timer));
        }
    }

    fn wake_timers(&mut self) {
        let now = Instant::now();
        while let Some(peek) = self.timers.peek_mut() {
            if peek.0.timestamp <= now {
                if let Some(waker) = peek.0.waker.take() {
                    waker.wake();
                }
                PeekMut::pop(peek);
            } else {
                break;
            }
        }
    }

    pub fn add_task_to_poll(&mut self, task_id: TaskId) {
        self.tasks_to_poll.insert(task_id);
    }

    pub fn poll(&mut self, event_loop: &ActiveEventLoop) -> Poll<()> {
        self.sync_proxy();

        for id in self.tasks_to_poll.drain() {
            if let Entry::Occupied(mut entry) = self.tasks.entry(id) {
                if entry.get_mut().poll().is_ready() {
                    entry.remove();
                }
            }
        }

        self.wake_timers();
        if let Some(Reverse(timer)) = self.timers.peek() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(timer.timestamp))
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

impl ExecutorProxy {
    pub fn spawn<F: Future<Output = ()> + 'static>(&mut self, future: F) -> TaskId {
        let id = self.task_counter.get_and_inc();
        let future = Box::pin(future);
        self.new_tasks.push((id, future));
        id
    }

    pub fn add_timer(&mut self, timestamp: Instant) -> Timer {
        let timer = Timer {
            timestamp,
            waker: Default::default(),
        };
        self.new_timers.push(timer.clone());
        timer
    }
}
