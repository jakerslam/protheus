# Roadmap Gateway Graduation Plan (compat path: `adapters/**`)

## Objective

Graduate the roadmap gateway set under one explicit contract model and make closure visible in release evidence.

## Target Gateway Set

1. `ollama`
2. `llama.cpp`
3. `mcp_baseline`
4. `otlp_exporter`
5. `durable_memory_local` (compat alias: `local_durable_memory_backend`)

## Graduation Requirements

1. Manifest entry exists for each target gateway.
2. Each target gateway includes a gateway checklist in the shared graduation manifest:
- `health_checks`
- `fail_closed_behavior`
- `chaos_scenarios`
- `receipt_completeness`
- `fallback_degradation_declaration`
3. Each target gateway publishes support level (`experimental` / `candidate` / `graduated`) and owner/blocker metadata.
4. Required lifecycle hooks are implemented.
5. Chaos scenarios pass:
- `process_never_starts`
- `starts_then_hangs`
- `invalid_schema_response`
- `response_too_large`
- `repeated_flapping`
6. Fail-closed behavior is explicit and receipt-backed.
7. Gateway appears in release proof-pack summary with graduation status.

## Release Criteria

1. No target gateway may remain undocumented in graduation manifest.
2. All target gateways must remain on one shared readiness track in the graduation manifest (`gateway_production_v1`) and use the same checklist schema.
3. Release gating must consume the same graduation manifest used by chaos checks.
4. Any non-graduated roadmap gateway must carry explicit blocker reason and owner.
