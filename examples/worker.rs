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
        let result = tokio::select! {
            r = do_work(name) => r,
            _ = guard.wait() => break,
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

async fn important_worker() {
    let guard = elegant_departure::get_shutdown_guard().shutdown_on_drop();

    for i in 0.. {
        // Do some important work and wait for the shutdown.
        tokio::select! {
            _ = sleep(Duration::from_secs(1)) => {},
            _ = guard.wait() => break,
        };

        if i == 5 {
            panic!("Oh no an unexpected crash in the important worker!");
        }
        println!("[important_worker] Did some important work");
    }

    println!("[important_worker] Important worker is shutting down");
}

#[tokio::main]
async fn main() {
    tokio::spawn(worker(1));
    tokio::spawn(worker(2));
    tokio::spawn(important_worker());

    elegant_departure::tokio::depart().on_termination().await
}
