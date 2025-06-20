use std::{
    cell::{Ref, RefCell, RefMut},
    mem,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

use winit::{
    error::OsError,
    event_loop::ActiveEventLoop,
    window::{WindowAttributes, WindowId},
};

use crate::app::{AppProxy, AppState, WindowState};

pub struct Window {
    id: WindowId,
    app: Rc<RefCell<AppState>>,
}

impl Drop for Window {
    fn drop(&mut self) {
        assert!(self.app.borrow_mut().windows.remove(&self.id).is_some())
    }
}

impl Window {
    pub(crate) fn new(
        app: AppProxy,
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
    ) -> Result<Self, OsError> {
        let app = app.state.clone();
        let window = event_loop.create_window(attributes)?;
        let id = window.id();
        assert!(
            app.borrow_mut()
                .windows
                .insert(id, WindowState::new(window))
                .is_none()
        );
        Ok(Self {
            id,
            app: app.clone(),
        })
    }

    fn state(&self) -> Ref<'_, WindowState> {
        Ref::map(self.app.borrow(), |state| {
            state
                .windows
                .get(&self.id)
                .unwrap_or_else(|| panic!("Window not found: {:?}", self.id))
        })
    }
    fn state_mut(&mut self) -> RefMut<'_, WindowState> {
        RefMut::map(self.app.borrow_mut(), |state| {
            state
                .windows
                .get_mut(&self.id)
                .unwrap_or_else(|| panic!("Window not found: {:?}", self.id))
        })
    }

    pub fn request_render(&mut self) -> RequestRender<'_> {
        self.state().window.request_redraw();
        RequestRender { owner: self }
    }

    pub fn is_closed(&self) -> bool {
        self.state().close_requested
    }
    pub fn closed(&mut self) -> Closed<'_> {
        Closed { owner: self }
    }
}

pub struct RequestRender<'a> {
    owner: &'a mut Window,
}

impl<'a> Future for RequestRender<'a> {
    type Output = Option<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.owner.state_mut();
        if mem::replace(&mut state.redraw_requested, false) || state.close_requested {
            Poll::Ready(if !state.close_requested {
                Some(())
            } else {
                None
            })
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

pub struct Closed<'a> {
    owner: &'a mut Window,
}

impl<'a> Future for Closed<'a> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.owner.state_mut();
        if state.close_requested {
            Poll::Ready(())
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
