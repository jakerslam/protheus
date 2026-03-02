# `core/memory` Compatibility Alias

This directory is a compatibility surface for external instructions that expect a
`core/memory` crate layout.

Canonical runtime implementation remains:

- `systems/memory/rust/` (authoritative Rust memory core)
- `systems/memory/memory_recall.ts` (TS runtime integration)

Use:

```bash
node core/memory/compat_bridge.js status
```

to inspect alias -> canonical path mapping.
