# Observability traces

This subdomain is part of the physical Observability domain. It contains the universal trace substrate, runtime trace source maps, trace schemas, and producer-to-Sentinel stream contracts.

Canonical contract:

- `trace_envelope.schema.json`
- `domain_trace_extension_registry.json`
- `sentinel_trace_source_map.json`

Canonical policy:

- `docs/workspace/universal_trace_substrate_policy.md`

## Fragmented Observability Anti-Pattern

Fragmented observability is a negative architecture state: each subsystem emits useful local facts, but no universal trace substrate ties them into one causal graph.

Observability owns the root trace envelope and correlation fabric. Kernel, Orchestration, Gateway, Shell, Validation, Governance, Conduit, and external assimilation probes may attach typed payload extensions, but they must remain projectable into the universal envelope.

## Single Trace ID Invariant

One `trace_id` is minted at the initial user request and flows unchanged through every workflow, orchestration decision, tool call, Gateway/Conduit boundary, Kernel receipt, Validation run, Shell projection, Sentinel observation, and final response for that causal story.

Domains may create their own `span_id` values. They may not create competing root trace IDs, drop the trace ID, fork it, or replace it. No exceptions.

Citation: `Machine Learning Systems, Volume 1`, Chapter 5 `AI Workflow`, motivates this stance through workflow feedback loops, distributed monitoring, degradation prevention, and system-level behavior: https://mlsysbook.ai/vol1/assets/downloads/Machine-Learning-Systems-Vol1.pdf
