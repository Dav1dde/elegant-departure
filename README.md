Elegant Departure
=================

[![Crates.io][crates-badge]][crates-url]
[![License][mit-badge]][mit-url]
[![Build Status][actions-badge]][actions-url]
[![docs.rs][docsrs-badge]][docsrs-url]

[crates-badge]: https://img.shields.io/crates/v/elegant-departure.svg
[crates-url]: https://crates.io/crates/elegant-departure
[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/tokio-rs/tokio/blob/master/LICENSE
[actions-badge]: https://github.com/Dav1dde/elegant-departure/workflows/CI/badge.svg
[actions-url]: https://github.com/Dav1dde/elegant-departure/actions?query=workflow%3ACI+branch%3Amaster
[docsrs-badge]: https://img.shields.io/docsrs/elegant-departure
[docsrs-url]: https://docs.rs/elegant-departure

Rust crate to simplify graceful async shutdowns:

- **Easy** to use with a minimal API
- **Runtime independent** (works with tokio, async-std, smol, …)
- **Additional integrations** for tokio (shutdown on ctrl-c, signals etc.)

## Usage

This crate is [on crates.io](https://crates.io/crates/elegant-departure) and can be
used by adding it to your dependencies in your project's `Cargo.toml`.

```toml
[dependencies]
elegant-departure = "0.2"
```

For a optional tokio integration, you need to enable the tokio feature:

```toml
[dependencies]
elegant-departure = { version = "0.2", features = "tokio" }
```

## Example

Examples can be found in the [example](./examples/) directory:

- [Simple](./examples/simple.rs): simple example without tokio integration
- [Axum](./examples/axum.rs): Axum integration example
- [Tokio](./examples/tokio.rs): Tokio integration example
- [Hyper](./examples/hyper.rs): a shutdown example using the Hyper webserver
- [Worker](./examples/worker.rs): example implementation of a worker using `select!`
- [Smol](./examples/smol.rs): example using the smol runtime
- [Async Std](./examples/async_std.rs): example using the async_std runtime

Minimal example using the tokio integration:

```rust
use std::time::Duration;

async fn worker(name: &'static str) {
    let guard = elegant_departure::get_shutdown_guard();

    println!("[{}] working", name);

    guard.wait().await;
    println!("[{}] shutting down", name);

    tokio::time::sleep(Duration::from_secs(1)).await;
    println!("[{}] done", name);
}

#[tokio::main]
async fn main() {
    tokio::spawn(worker("worker 1"));
    tokio::spawn(worker("worker 2"));

    elegant_departure::tokio::depart()
        // Shutdown on Ctrl+C and SIGTERM
        .on_termination()
        .await
}
```
