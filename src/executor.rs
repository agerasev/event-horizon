use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Weak},
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use winit::event_loop::EventLoopProxy;

use crate::{
    app::{App, AppProxy, UserEvent},
    window::Window,
};

struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
    _data: Arc<TaskData>,
    waker: Waker,
}

struct TaskData {
    event_loop: EventLoopProxy<UserEvent>,
}

impl TaskData {
    fn wake(&self) {
        if self.event_loop.send_event(UserEvent {}).is_err() {
            panic!("Event loop closed");
        }
    }
}

impl Drop for TaskData {
    fn drop(&mut self) {
        println!("Task data dropped");
    }
}

unsafe fn make_raw_waker(data: Weak<TaskData>) -> RawWaker {
    unsafe fn clone_ptr(ptr: *const ()) -> RawWaker {
        let data = unsafe { Weak::<TaskData>::from_raw(ptr.cast()) };
        let cloned = unsafe { make_raw_waker(data.clone()) };
        let _leak = data.into_raw();
        cloned
    }
    unsafe fn wake_ptr(ptr: *const ()) {
        let data = unsafe { Weak::<TaskData>::from_raw(ptr.cast()) };
        if let Some(data) = data.upgrade() {
            data.wake();
        }
    }
    unsafe fn wake_ptr_by_ref(ptr: *const ()) {
        let data = unsafe { Weak::<TaskData>::from_raw(ptr.cast()) };
        if let Some(data) = data.upgrade() {
            data.wake();
        }
        let _leak = data.into_raw();
    }
    unsafe fn drop_ptr(ptr: *const ()) {
        drop(unsafe { Weak::<TaskData>::from_raw(ptr.cast()) });
    }

    const VTABLE: RawWakerVTable =
        RawWakerVTable::new(clone_ptr, wake_ptr, wake_ptr_by_ref, drop_ptr);

    RawWaker::new(data.into_raw().cast(), &VTABLE)
}

impl Task {
    fn new(app: &AppProxy, future: Pin<Box<dyn Future<Output = ()>>>) -> Self {
        let data = Arc::new(TaskData {
            event_loop: app.event_loop.clone(),
        });
        Self {
            future,
            waker: unsafe { Waker::from_raw(make_raw_waker(Arc::downgrade(&data))) },
            _data: data,
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
