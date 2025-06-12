use std::{
    cell::RefCell,
    mem,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use crate::app::{AppHandle, AppState};

pub struct Window {
    state: Rc<RefCell<AppState>>,
}

impl Window {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { state: app }
    }

    pub fn render(&mut self) -> RenderFuture<'_> {
        RenderFuture { state: &self.state }
    }

    pub fn closed(&self) -> bool {
        self.state.borrow().close_requested
    }
}

pub struct RenderFuture<'a> {
    state: &'a RefCell<AppState>,
}

impl<'a> Future for RenderFuture<'a> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if mem::replace(&mut self.state.borrow_mut().redraw_requested, false)
            || self.state.borrow().close_requested
        {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}
