# Plugin and WASM Component Model Spec

## Objective

Enable third-party extensions while preserving Layer 0 fail-closed authority and deterministic receipts.

## Supported Plugin Classes

- `cognition_reflex`
- `substrate_adapter`
- `memory_backend`

## Registration Contract

Runtime registration is routed through conduit `install_extension` and stored in the plugin registry.

Required fields:
- `extension_id` (plugin id)
- `plugin_type`
- `version`
- `wasm_component_path`
- `wasm_sha256`
- `capabilities`

Optional fields:
- `signature`
- `provenance`
- `recovery_max_attempts`
- `recovery_backoff_ms`

Validation (fail-closed):
- `extension_id` must be non-empty.
- `plugin_type` must be one of the allowed classes.
- `wasm_component_path` must be present.
- `wasm_sha256` must be valid SHA-256 format.
- `capabilities` must be non-empty and normalized.

## Runtime Registry

Canonical state path:
- `client/runtime/local/state/extensions/plugin_registry.json`

Canonical receipt log:
- `client/runtime/local/state/extensions/plugin_runtime_receipts.jsonl`

Each registry entry tracks:
- identity (`plugin_id`, `plugin_type`, `version`)
- source (`wasm_component_path`, `wasm_sha256`, signature/provenance)
- runtime policy (`enabled`, max attempts, retry backoff)
- health state (`healthy`, `healing`, `quarantined`)
- failure/recovery telemetry (`failure_count`, `next_retry_ts_ms`, `last_error`)

## Sandbox Rules (Mandatory)

- Execute as WASM component under a capability-scoped host runtime (wasmtime class).
- No implicit filesystem or network access.
- Host calls are deny-by-default unless policy-approved.
- Enforce strict per-invocation resource limits (time/memory/ops).
- Emit deterministic timeout/violation receipts on forced termination.

## Auto-Heal Policy

Auto-heal runs on runtime status polls and registration events:
1. Verify component presence at `wasm_component_path`.
2. Verify component hash equals `wasm_sha256`.
3. On failure:
   - mark `healing`,
   - increment failure count,
   - schedule bounded exponential retry.
4. On repeated failure beyond cap:
   - mark `quarantined`,
   - disable plugin,
   - emit quarantine receipt.
5. On recovery:
   - mark `healthy`,
   - clear failure state,
   - emit recovery receipt.

Auto-heal goals:
- no hardcoded manual restart dependency,
- deterministic bounded retry behavior,
- explicit quarantine for untrusted or drifting artifacts.

## Host Interface (WIT)

- `invoke(input-json: string) -> result<string, string>`
- `health() -> string`
- `capabilities() -> string`

See `adapters/protocol/wasm_adapter_skeleton/wit/infring_plugin.wit`.

## Policy Integration

Layer 1 policy validates manifest and capability scope.

Layer 0 conduit enforces:
- registration invariants,
- command authorization,
- fail-closed runtime response on violations.
