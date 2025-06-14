use event_horizon::{executor::enter, runtime::Runtime};

async fn main_(rt: Runtime) {
    println!("Started");
    while let Some(_render) = rt.request_render().await {
        println!("Rendered");
    }
    println!("Closed");
}

fn main() {
    enter(main_);
}
