# Singularity Seed Demo

Run one full sovereignty-guarded singularity seed cycle:

```bash
node apps/examples/singularity-seed-demo/run.js
```

Expected behavior:
- Loads 4 signed loop blobs.
- Evolves each loop one generation.
- Unfolds and verifies evolved loop blobs via signed manifest.
- Fail-closes automatically if drift exceeds 2%.

This example executes the public Rust seed binary (`singularity_seed_core demo`) rather than importing private runtime internals from `client/`.
