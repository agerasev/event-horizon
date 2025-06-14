use std::{
    cell::RefCell,
    mem,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use crate::{
    app::{AppProxy, AppState},
    executor::{ExecutorProxy, Timer},
};

/// Handle to underlying async runtime.
#[derive(Clone)]
pub struct Runtime {
    executor: Rc<RefCell<ExecutorProxy>>,
    state: Rc<RefCell<AppState>>,
}

impl Runtime {
    pub(crate) fn new(executor: Rc<RefCell<ExecutorProxy>>, app: AppProxy) -> Self {
        Self {
            executor,
            state: app.state,
        }
    }

    pub fn request_render(&self) -> RequestRenderFuture<'_> {
        if let Some(window) = self.state.borrow().window.as_ref() {
            window.request_redraw();
        }
        RequestRenderFuture { state: &self.state }
    }

    pub fn is_closed(&self) -> bool {
        self.state.borrow().close_requested
    }

    // TODO: Return JoinHandle
    pub fn spawn<F: Future<Output = ()> + 'static>(&self, future: F) {
        self.executor.borrow_mut().spawn(future);
    }

    pub fn sleep(&self, timeout: Duration) -> SleepFuture {
        let timestamp = Instant::now().checked_add(timeout).unwrap();
        let timer = self.executor.borrow_mut().add_timer(timestamp);
        SleepFuture { timestamp, timer }
    }
}

pub struct RequestRenderFuture<'a> {
    state: &'a RefCell<AppState>,
}

impl<'a> Future for RequestRenderFuture<'a> {
    type Output = Option<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.borrow_mut();
        if mem::replace(&mut state.redraw_requested, false) || state.close_requested {
            Poll::Ready(if !state.close_requested {
                Some(())
            } else {
                None
            })
        } else {
            state.redraw_waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct SleepFuture {
    timestamp: Instant,
    timer: Rc<RefCell<Timer>>,
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.timestamp {
            Poll::Ready(())
        } else {
            self.timer.borrow_mut().waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
