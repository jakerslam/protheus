# Roadmap Adapter Graduation Plan

## Objective

Graduate the roadmap adapter set under one explicit contract model and make closure visible in release evidence.

## Target Adapter Set

1. `ollama`
2. `llama.cpp`
3. `mcp_baseline`
4. `otlp_exporter`
5. `local_durable_memory_backend`

## Graduation Requirements

1. Manifest entry exists for each target adapter.
2. Required lifecycle hooks are implemented.
3. Chaos scenarios pass:
- `process_never_starts`
- `starts_then_hangs`
- `invalid_schema_response`
- `response_too_large`
- `repeated_flapping`
4. Fail-closed behavior is explicit and receipt-backed.
5. Adapter appears in release proof-pack summary with graduation status.

## Release Criteria

1. No target adapter may remain undocumented in graduation manifest.
2. Release gating must consume the same graduation manifest used by chaos checks.
3. Any non-graduated roadmap adapter must carry explicit blocker reason and owner.

