# InfRing Public Roadmap

This roadmap is intentionally execution-oriented and receipt-first. Milestones ship only when lane tests and runtime receipts pass.

## Q2 2026 (Stability + Adoption)

### 1. Layer 2 completion and extension ergonomics
- Finalize Layer 2 execution lane parity and remove remaining WIP gaps.
- Publish initiative primitive extension docs and reference examples.
- Gate new primitives with deterministic contract-runtime receipts.

### 2. First five production adapters
- Ship and stabilize first-class adapters for:
  - `ollama` (pure mode)
  - `llama.cpp` (pure mode)
  - MCP client/server interoperability baseline
  - OTLP exporter bridge
  - Local durable memory backend profile
- Enforce conduit-only execution and fail-closed adapter policy checks.

### 3. Runtime self-maintenance hardening
- Keep memory/storage bounded under sustained load with pressure-triggered cleanup.
- Add stronger state compaction and stale-surface sweep cadence.
- Publish 72h boundedness evidence artifacts.

### 4. Dashboard reliability polish
- Eliminate stale cockpit block drift under normal load.
- Strengthen conduit auto-heal and queue lane backpressure automation.
- Keep dashboard telemetry authoritative to runtime contracts.

### 5. Public onboarding baseline (InfRing 101)
- Keep glossary and architecture guides current with runtime contracts.
- Maintain runnable reference apps for sovereign memory, local research, and tiny-max MCU monitoring.
- Keep quick-start docs aligned with dual-license scope and support pathways.

## Q3 2026 (Scale + Edge)

### 1. 10-second assimilate lane
- Reduce p95 assimilate latency to target envelope with deterministic receipts.
- Add bounded fallback when substrate/network paths degrade.

### 2. Tiny-max and MCU proofs
- Validate tiny-max deployment profile on constrained edge targets.
- Publish MCU-oriented proof artifacts for memory, throughput, and failure recovery.
- Keep no_std authority path aligned with Layer 0 invariants.

### 3. Plugin/WASM ecosystem alpha
- Launch sandboxed WASM component plugin model for reflexes/adapters/backends.
- Add signed registration, runtime registry state, and auto-heal/quarantine enforcement.
- Publish third-party extension authoring guide.

### 4. Scale-readiness gates
- Raise spine reliability and latency SLOs with continuous regression proofing.
- Expand queue/conduit/cockpit self-healing automation for unattended runs.
- Run multi-day autonomous soak tests with bounded resource guarantees.

## Ongoing
- Safety-first constitutional enforcement (T0 + fail-closed policy routing)
- Receipt-first audit guarantees across all runtime actions
- Rust-authority migration where it materially improves reliability, performance, and trust
