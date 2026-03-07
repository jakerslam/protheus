# Cognition Plane

The cognition plane is probabilistic userland. It can propose and optimize, but does not own correctness.

## Scope

- Model routing and execution surfaces.
- Retrieval, planning, summarization, and tool mediation.
- Persona overlays and adaptive strategy surfaces.
- Epistemic memory objects and confidence-aware reasoning.

## Model Hierarchy

- Reflex models: tiny, local, low-latency routing/safety classifiers.
- Executive models: local planning and tool-use orchestration.
- External models: large-scale reasoning and knowledge expansion.

## Epistemic Memory

Cognition memory items are first-class epistemic objects with:

- value
- confidence
- provenance
- expiry
- policy label
- allowed effects

Schema: `planes/cognition/epistemic_memory.schema.json`.

Implementation mapping: `client/` source surfaces; mutable artifacts in `client/local/`.
