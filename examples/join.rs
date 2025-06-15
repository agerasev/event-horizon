use std::time::Duration;

use wgame::{executor::enter, runtime::Runtime};

async fn main_(rt: Runtime) {
    println!("Spawning new task");
    rt.spawn({
        let rt = rt.clone();
        async move {
            println!("Sleep task 1");
            rt.sleep(Duration::from_secs(1)).await;
            println!("Awakened task 1");
        }
    })
    .await;
    println!("Joined task 0");
}

fn main() {
    enter(main_);
}
