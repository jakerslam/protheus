# Observability Plan (OTLP + Dashboard)

## Scope

Complete OTLP metrics/traces export while preserving local-first, receipt-first guarantees.

## OTLP Endpoints

- Metrics: `OTEL_EXPORTER_OTLP_METRICS_ENDPOINT` (default `http://127.0.0.1:4318/v1/metrics`)
- Traces: `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` (default `http://127.0.0.1:4318/v1/traces`)
- Optional logs: `OTEL_EXPORTER_OTLP_LOGS_ENDPOINT`

## Required Signals

### Metrics
- `infring.queue.depth`
- `infring.conduit.signals.active`
- `infring.cockpit.blocks.active`
- `infring.cockpit.blocks.stale`
- `infring.spine.success_rate`
- `infring.receipt.latency.p95`
- `infring.receipt.latency.p99`

### Traces
- `conduit.dispatch`
- `attention.enqueue`
- `attention.drain`
- `agent.lifecycle.spawn|terminate|revive`
- `plugin.wasm.invoke`

## Dashboard Coupling

- Dashboard consumes runtime telemetry from the same canonical metrics surface.
- If OTLP export fails, dashboard remains live from local state and surfaces exporter health explicitly.
- Any SLO violation emits both dashboard alert state and deterministic receipt.

## SLO Gates

- queue depth target: `< 60`
- conduit active signals floor: `>= 6`
- stale cockpit blocks target: `< 10`
- spine success target: `>= 0.90` (near term), `>= 0.999` (scale target)

## Rollout Phases

1. Emit canonical metrics/traces locally.
2. Wire OTLP exporter with fail-closed validation.
3. Validate dashboard/OTLP parity under stress.
4. Add automated remediation receipts for SLO breach classes.
