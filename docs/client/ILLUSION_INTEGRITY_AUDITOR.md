# Illusion Integrity Auditor

`V4-SELF-001` establishes a self-audit lane that continuously checks for presentation, governance, and execution-surface leaks that weaken enterprise credibility.

## Entrypoints

- Manual:
  - `node client/runtime/systems/self_audit/illusion_integrity_lane.js run --trigger=manual`
  - `protheusctl audit illusion`
- Startup hook:
  - Triggered by `protheus start` / `protheus restart` in `protheus_control_plane`
- Promotion hook:
  - Triggered during release/environment promotion flows

## Engine

- Rust scanner core:
  - `client/runtime/systems/self_audit/illusion_integrity_auditor.rs`
  - `client/runtime/systems/self_audit/rust/Cargo.toml`
- TS orchestration lane:
  - `client/runtime/systems/self_audit/illusion_integrity_lane.ts`

The lane prefers Rust, then falls back to TS when policy allows.

## What It Checks

- Root-level naming and personal-marker leaks
- Required artifact presence (README/changelog/docs/client/templates)
- Backlog drift (`client/runtime/systems/ops/backlog_registry.ts check`)
- UI/documentation consistency score
- Scientific reasoning surface completeness score
- Git metadata concentration/burst heuristics

## Receipts

- Latest: `state/self_audit/illusion_integrity/latest.json`
- Receipts: `state/self_audit/illusion_integrity/receipts.jsonl`
- History: `state/self_audit/illusion_integrity/history.jsonl`
- Reports: `state/self_audit/illusion_integrity/client/reports/`
- Patch suggestions: `state/self_audit/illusion_integrity/client/runtime/patches/`

Receipts are signed (HMAC) using `ILLUSION_AUDIT_SIGNING_SECRET` (or policy fallback).

## Human Consent Gate

Autofix is blocked unless all conditions are met:

- `autofix.allow_apply=true` in policy
- `--apply=1`
- approval note length meets policy minimum
- consent token prefix matches policy requirement

Default policy keeps autofix disabled.

