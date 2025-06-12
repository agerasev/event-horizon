use std::{
    cell::RefCell,
    future::Future,
    ops::ControlFlow,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
};

use crate::{
    app::{App, AppHandle},
    window::Window,
};

struct Runtime<F: Future<Output = ()>> {
    main: Pin<Box<F>>,
    app: AppHandle,
    waker: Waker,
}

impl<F: Future<Output = ()>> Runtime<F> {
    fn new(main: F, app: AppHandle) -> Self {
        Self {
            main: Box::pin(main),
            app,
            // TODO: On `wake` send UserEvent in App
            waker: Waker::noop().clone(),
        }
    }

    fn poll(&mut self) -> ControlFlow<()> {
        println!("Poll");
        let mut cx = Context::from_waker(&self.waker);
        match F::poll(self.main.as_mut(), &mut cx) {
            Poll::Pending => ControlFlow::Continue(()),
            Poll::Ready(()) => ControlFlow::Break(()),
        }
    }
}

pub fn enter<F: AsyncFnOnce(Window) + 'static>(main: F) {
    let mut app = App::default();

    let window = Window::new(app.handle());

    let rt = Rc::new(RefCell::new(Runtime::new(main(window), app.handle())));

    if rt.borrow_mut().poll().is_continue() {
        app.run(move || rt.borrow_mut().poll()).unwrap();
    }
}
