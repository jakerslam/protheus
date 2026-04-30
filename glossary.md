# InfRing Glossary (InfRing 101)

## Core Architecture
- **Layer -1**: Hardware/host substrate boundary and low-level wrappers.
- **Layer 0**: Deterministic safety and policy authority (fail-closed control plane).
- **Layer 1**: Governance, contract schemas, policy composition, and guarantees.
- **Layer 2**: Execution intelligence and orchestration primitives.
- **Layer 3**: Kernel OS Personality Template for process, service, VFS, driver, syscall, namespace, networking, and userland-isolation growth without bypassing Layer 2 scheduling/admission authority.
- **Kernel**: Canonical authority/truth owner. `core/**` is the implementation path and `Core` is compatibility wording only.
- **Orchestration Control Plane**: Canonical coordination owner for decomposition, sequencing, recovery, clarification, and result packaging. `Tower` is rejected as an active architecture term.
- **Shell**: Canonical presentation owner. `client/**` is the implementation path and `Client` is compatibility wording only.
- **Gateways**: Canonical external membrane. `adapters/**` is the implementation path and `Adapters` is compatibility wording only.
- **Assurance**: Umbrella for Validation, Observability, and Governance. Validation judges controlled behavior; Observability watches live behavior; Governance derives gates, scorecards, verdicts, and issue-candidate thresholds.
- **Cognition Plane**: Historical broad metakernel plane label for non-authoritative coordination and presentation. Active ownership should be named Orchestration Control Plane or Shell.

## Runtime Primitives
- **Conduit**: The governed action bus; all authoritative operations route through it.
- **Receipt**: Deterministic action evidence artifact (what happened, when, why, and policy context).
- **Receipt Chain**: Hash-linked receipts used for replay, audit, and tamper evidence.
- **Attention Queue**: Prioritized event queue used for triage, escalation, and operator/system focus.
- **Importance Scoring**: Deterministic ranking of events/tasks (criticality, urgency, impact, relevance, confidence).
- **Initiative Primitive**: Layer 2 lane that turns importance into bounded escalation behavior.

## Safety and Governance
- **T0 Invariants**: Constitutional safety rules that always execute first and cannot be bypassed.
- **Fail-Closed**: Default deny on uncertainty or policy mismatch.
- **Safety Plane**: Runtime subset that enforces irreversible safety constraints and policy boundaries.
- **Constitution Hash**: Integrity marker for active constitutional policy set.

## Memory and Continuity
- **Memory Hierarchy**: Structured memory tiers (live, warm, archive) with governed retention and compaction.
- **Compaction**: Loss-aware pruning/summarization to keep context bounded and high-signal.
- **Snapshot History**: Time-sequenced runtime state captures used for continuity/audit.

## Swarm and Agents
- **Swarm Runtime**: Multi-agent collaboration execution lane with deterministic governance.
- **Handoff**: Structured transfer of context/work between agents.
- **Agent Contract**: Mission/expiry/termination rules for ephemeral agents.
- **Thorn Cell**: Sacrificial quarantine cell for compromised agents.

## Specialized Terms
- **Dream Sequencer**: Memory-context sequencing subsystem for retrieval and narrative continuity under policy constraints.
- **Plugin Registry**: Deterministic runtime index of registered WASM extensions and their health state.
- **Plugin Auto-Heal**: Bounded retry/quarantine loop that restores healthy plugins or fail-closes compromised ones.
- **Pure Mode**: Rust-only runtime/client authority path with no Node/TS runtime dependency.
- **Tiny-Max**: Extreme low-resource pure profile for constrained hardware/edge targets.
