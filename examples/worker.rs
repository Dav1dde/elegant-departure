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
            _ = guard.wait() => None,
            r = do_work(name) => Some(r),
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

async fn important_worker() {
    let _guard = elegant_departure::get_shutdown_guard().shutdown_on_drop();

    // Do some important work.
    for i in 0.. {
        sleep(Duration::from_secs(1)).await;
        if i == 5 {
            panic!("Oh no an unexpected crash in the important worker!");
        }
        println!("[important_worker] Did some important work");
    }
}

#[tokio::main]
async fn main() {
    tokio::spawn(worker(1));
    tokio::spawn(worker(2));
    tokio::spawn(important_worker());

    elegant_departure::tokio::depart().on_termination().await
}
