use std::time::Duration;

use event_horizon::{executor::enter, runtime::Runtime};

async fn main_(rt: Runtime) {
    println!("Going to sleep");
    rt.sleep(Duration::from_secs(1)).await;
    println!("Awakened");
}

fn main() {
    enter(main_);
}
