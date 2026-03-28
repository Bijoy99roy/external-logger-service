# External Logger Service

A Rust service for receiving external log messages and forwarding them to storage. A simple ingest pathway, WebSocket handling, and storage adapters.

## Prerequisites

- Rust toolchain (stable)
- Redis

## Build

```bash
cargo build --release
```

## Run

Start the service locally:

```bash
cargo run
```

Set configuration via environment variables or edit the configuration in `src/config.rs` as needed.

## Testing

```bash
cargo test
```
