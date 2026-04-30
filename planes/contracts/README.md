# Planes Contract Registry

This directory is the single-source contract registry for inter-plane interfaces.

## Canonical contract
- `conduit_envelope.schema.json`: required shape for any client<->core conduit message.

Bindings can be generated from this schema for Rust/TS/Python clients.

## Assurance-shaped contract relocation

Eval-loop and live Observability contracts are owned by the Assurance physical domains. New eval contracts belong under `validation/evals/**`; new live Observability contracts belong under `observability/**`. The remaining files here are non-Assurance plane/runtime contracts consumed by Kernel lanes.
