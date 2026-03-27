# Rust Core File Size Policy

- Scope: `core/**/*.rs`
- Tests exempt: yes (`**/tests/**`, `*.test.*`, `**/__tests__/**`)
- Hard cap: **500 LoC** per file
- Enforcement: `npm run -s ops:rust-core-file-size:gate`
- Exception source of truth: `docs/workspace/rust_core_file_size_policy.json`

## Rules

1. New Rust core files must be <= 500 LoC.
2. Existing files > 500 LoC require an explicit exception entry with owner, reason, and expiry.
3. Any touched exception file should be reduced toward <= 500 LoC in the same batch when feasible.
4. Expired exceptions fail the strict gate until renewed or split.

## Current Exceptions

| Path | Lines | Owner | Expires | Reason |
| --- | ---: | --- | --- | --- |
| _none_ | 0 | - | - | - |

Generated from policy snapshot: 2026-03-27T02:12:17.952Z
