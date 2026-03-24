# WASM Adapter Skeleton

Reference skeleton for a sandboxed InfRing WASM component adapter.

## Layout

- `wit/infring_plugin.wit` - component interface contract
- `src/lib.rs` - minimal Rust implementation skeleton
- `Cargo.toml` - crate metadata

## Build (example)

```bash
cargo build --target wasm32-wasip1
```

This skeleton is intentionally minimal; registration and policy wiring happen through runtime plugin lanes.
