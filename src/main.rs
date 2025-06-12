use event_horizon::{runtime::enter, window::Window};

async fn main_(mut window: Window) {
    while !window.closed() {
        window.render().await;
        println!("Rendered");
    }
    println!("Closed");
}

fn main() {
    enter(main_);
}
