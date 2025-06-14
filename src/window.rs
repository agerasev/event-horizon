use std::{
    cell::RefCell,
    mem,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use crate::app::{AppProxy, AppState};

pub struct Window {
    state: Rc<RefCell<AppState>>,
}

impl Window {
    pub(crate) fn new(app: AppProxy) -> Self {
        Self { state: app.state }
    }

    pub fn request_render(&mut self) -> RenderFuture<'_> {
        if let Some(window) = self.state.borrow().window.as_ref() {
            window.request_redraw();
        }
        RenderFuture { state: &self.state }
    }

    pub fn is_closed(&self) -> bool {
        self.state.borrow().close_requested
    }
}

pub struct RenderFuture<'a> {
    state: &'a RefCell<AppState>,
}

impl<'a> Future for RenderFuture<'a> {
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
