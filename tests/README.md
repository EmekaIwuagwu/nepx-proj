# Integration Tests

To run full integration tests, we recommend using `near-workspaces-rs`.

## Setup

1. Add `near-workspaces` to your root `Cargo.toml` dev-dependencies:
```toml
[dev-dependencies]
near-workspaces = "0.10"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
serde_json = "1.0"
```

2. Create a test file `tests/integration_test.rs`:

```rust
use near_workspaces::{Account, Contract, Worker};

#[tokio::test]
async fn test_bridge_flow() -> anyhow::Result<()> {
    let worker = near_workspaces::sandbox().await?;
    // ... deployment and interaction logic
    Ok(())
}
```

3. Run with `cargo test`.
