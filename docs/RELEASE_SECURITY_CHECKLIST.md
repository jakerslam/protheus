# Release Security Checklist

Use this checklist for every tagged release (starting at `v0.2.0`).

## Required

1. Tag release from a clean `main` commit.
2. Run merge/security gates:
   - `cargo run --quiet --manifest-path crates/ops/Cargo.toml --bin protheus-ops -- enterprise-hardening run --strict=1`
   - `NODE_PATH=$PWD/node_modules npm run -s formal:invariants:run`
3. Generate SBOM artifact (CycloneDX JSON).
4. Generate checksum (`sha256`) for SBOM artifact.
5. Publish signed release notes with:
   - security-impact summary
   - migration notes
   - vulnerability/advisory references (if any)
6. Attach SBOM + checksum + notes to the GitHub release.

## Optional But Recommended

- Run `protheus-ops benchmark-matrix run --refresh-runtime=1` and attach report snapshot.
- Include reproducibility notes (toolchain versions, commit hash, date).
