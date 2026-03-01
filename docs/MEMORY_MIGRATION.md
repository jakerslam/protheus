# Memory Migration Status

## Overview

This document tracks the staged Rust memory migration for `V3-RACE-023`.

## Stage Status

- Stage 1: COMPLETE (2026-03-01)
- Stage 2: Pending
- Stage 3: Pending
- Stage 4: Pending

## Stage 1 Scope (Completed)

- Rust crate moved to `systems/memory/rust` and renamed to `protheus-memory-core`.
- SQLite is now the authoritative runtime index store for Rust memory commands.
- SQLite schema includes:
  - `embeddings`
  - `temporal_graph_nodes`
  - `temporal_graph_edges`
  - `hot_state`
  - `memory_index`
- Runtime commands (`query-index`, `get-node`, `build-index`) use SQLite-first behavior.
- Markdown index files remain compatibility fallback artifacts.
- TypeScript integration (`systems/memory/memory_recall.ts`) defaults to Rust first with deterministic JS fallback.
- Memory write receipts are published via `systems/ops/event_sourced_control_plane.js`.
- Stage 1 benchmark report is generated at `benchmarks/memory-stage1.md` from `systems/memory/rust_memory_transition_lane.ts`.

## Stage 1 Safety Notes

- Fallback path remains active for rollback safety.
- Benchmark-gated selector logic remains intact in transition lane.
- No Stage 2 export-only cutover was applied in this stage.
