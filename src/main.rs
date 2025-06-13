use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

use event_horizon::{executor::enter, window::Window};

#[derive(Default)]
struct LoaderInfo {
    waker: Option<Waker>,
    complete: bool,
}

struct Loader {
    info: Arc<Mutex<LoaderInfo>>,
}

impl Loader {
    fn new(delay: Duration) -> Self {
        let info = Arc::new(Mutex::new(LoaderInfo::default()));
        thread::spawn({
            let info = info.clone();
            move || {
                thread::sleep(delay);
                let mut guard = info.lock().unwrap();
                guard.complete = true;
                if let Some(waker) = guard.waker.take() {
                    waker.wake();
                }
            }
        });
        Self { info }
    }
}

impl Future for Loader {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.info.lock().unwrap();
        if guard.complete {
            Poll::Ready(())
        } else {
            guard.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

async fn main_(mut window: Window) {
    println!("Started");
    Loader::new(Duration::from_secs(1)).await;
    println!("Loaded");
    while !window.closed() {
        window.render().await;
        println!("Rendered");
    }
    println!("Closed");
}

fn main() {
    enter(main_);
}
