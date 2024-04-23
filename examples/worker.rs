use futures::FutureExt;
use std::time::Duration;
use tokio::time::sleep;

async fn do_work(num: u64) -> usize {
    sleep(Duration::from_millis(num * 300)).await;
    1
}

async fn worker(name: u64) {
    let guard = elegant_departure::get_shutdown_guard();

    let mut counter = 0;
    loop {
        // Or use `tokio::select!`
        let result = futures::select! {
            _ = guard.wait().fuse() => None,
            r = do_work(name).fuse() => Some(r),
        };

        let result = match result {
            None => break,
            Some(x) => x,
        };

        counter += result;
        println!("[worker {}] Did some hard work", name);
    }

    println!(
        "[worker {}] Created {} work units, cleaning up",
        name, counter
    );
    sleep(Duration::from_secs(1)).await;
    println!("[worker {}] Done", name);
}

#[tokio::main]
async fn main() {
    tokio::spawn(worker(1));
    tokio::spawn(worker(2));

    elegant_departure::tokio::depart().on_termination().await
}
