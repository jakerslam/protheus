# Protheus Architecture

Protheus is built as a Rust-first kernel (trusted core) with a narrow conduit to TypeScript surfaces.

## InfRing Direction

InfRing is the target operating model: a portable autonomous substrate that runs unchanged across desktop, server, embedded, and high-assurance profiles.

- Rust kernel remains the single source of truth.
- Conduit is the only TS <-> Rust bridge.
- TS is reserved for flexible surfaces (UI, marketplace, extensions, experimentation).

## Three-Plane Metakernel

Protheus is explicitly modeled as a substrate-independent metakernel with three planes:

1. Safety plane (`planes/safety`, implemented in `core/layer0..2`): deterministic authority for policy, isolation, scheduling, receipts, and fail-closed execution.
2. Cognition plane (`planes/cognition`, implemented in `client/`): probabilistic model orchestration, retrieval, planning, persona overlays, and user-facing cognition surfaces.
3. Substrate plane (`planes/substrate`): runtime/backend descriptors for CPU/MCU/GPU/NPU/QPU/neural channels with explicit degradation contracts.

Hard boundary:
- AI can propose; kernel authority decides.
- Client <-> core communication is conduit + scrambler only.
- Every substrate must declare fallback/degradation behavior.

## System Map

```mermaid
flowchart LR
    UI["Cognition Plane (Client Surface)"]
    CLI["Operator Surface (protheus/protheusctl/protheusd)"]
    CONDUIT["Conduit + Scrambler"]
    SAFETY["Safety Plane (Deterministic Rust Authority)"]
    SUBSTRATE["Substrate Plane (Node + Adapter Contracts)"]
    RECEIPTS["Deterministic Receipts + State Artifacts"]

    UI --> CONDUIT
    CLI --> CONDUIT
    CONDUIT --> SAFETY
    SAFETY --> SUBSTRATE
    SAFETY --> RECEIPTS
    SUBSTRATE --> RECEIPTS
```

## Runtime Flow

1. A command enters from CLI or a TS surface.
2. Conduit normalizes the command into a typed envelope.
3. Safety plane policy/constitution checks evaluate fail-closed.
4. Safety authority schedules deterministic execution against substrate adapters.
5. Cognition outputs are treated as probabilistic inputs unless authorized by safety policy.
6. Crossing + validation receipts are emitted for auditability.

## Portability Contract

- With TS present: conduit-backed orchestration and rich operator surfaces.
- Without TS: Rust core still runs with no kernel behavior drift.

## Related Docs

- [Getting Started](client/docs/GETTING_STARTED.md)
- [Conduit Requirement](client/docs/requirements/REQ-05-protheus-conduit-bridge.md)
- [Rust Primitive Requirement](client/docs/requirements/REQ-08-rust-core-primitives.md)
- [Security Posture](client/docs/SECURITY_POSTURE.md)
- [Three-Plane Model](planes/README.md)
