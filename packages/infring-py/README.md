# infring-cli-wrapper

Thin Python entrypoint for Infring.

This package does not re-implement kernel logic. It only forwards CLI arguments to the Rust `infring-ops` runtime.

## Install

```bash
pip install infring-cli-wrapper
```

From this repository:

```bash
pip install ./packages/infring-py
```

## Usage

```bash
infring --help
infring status --dashboard
```

## Runtime Resolution Order

1. `INFRING_OPS_BIN` (if set)
2. `infring-ops` on `PATH`
3. Local repo binaries:
   - `target/release/infring-ops`
   - `target/debug/infring-ops`
4. Cargo fallback:
   - `cargo run --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops -- ...`
