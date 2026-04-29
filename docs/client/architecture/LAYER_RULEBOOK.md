# InfRing Layer Rulebook — Strict Enforcement Policy
**Version 1.3** — April 2026  
**This is the source of truth for file placement, language boundaries, and layer ownership. No deviations without explicit user approval.**

### 1. Directory Split (Enforced)
The repository has seven top-level product/code roots:
- `/core` — deterministic core stack (`layer_minus_one`, `layer0`, `layer1`, `layer2`, `layer3`) and trusted low-level logic.
- `/surface` — orchestration coordination surfaces (request shaping, sequencing, clarification, recovery, packaging) that do not canonize truth.
- `/client` — shell surface path (developer-facing platform, SDKs, CLI, dashboards, and thin wrappers).
- `/packages` — public SDK/package distribution surfaces, starter bundles, and installable developer-facing wrappers.
- `/apps` — end-user applications built on top of the shell/platform surface.
- `/adapters` — Gateway layer implementation path (compat alias: Adapters) for external integration shims.
- `/tests` — integration, end-to-end, regression, and system verification surfaces.

All product code should live in one of these roots.

Allowed root-level exceptions (metadata/infrastructure): `.github/`, `.githooks/`, policy/docs, `scripts/`, `examples/`, `benchmarks/`, lockfiles, build manifests, deploy manifests, generated artifacts, and runtime state directories.

### 1.1 Placement Rule (Authority First)
Placement is decided by authority before language.

- If a surface decides, enforces, records, budgets, schedules, or guards system truth, it belongs in `core`.
- If a surface performs non-canonical orchestration coordination (classification, clarification, sequencing, progress/recovery packaging), it belongs in `surface/orchestration`.
- If a surface exists to help developers call, inspect, visualize, package, or extend the system, it belongs in the shell path `client`.
- If a surface exists to ship the public SDK/package layer to developers, it belongs in `packages`.
- If a surface is an opinionated workflow/product on top of the platform, it belongs in `apps`.
- If a surface exists to connect InfRing to something external, legacy, or third-party, it belongs in the Gateway layer (`adapters/**` path during compatibility transition).
- If a surface exists only to verify behavior, it belongs in `tests` or adjacent unit-test locations.

### 2. Layer Definitions (Strict)
- **Layer -1 (Exotic Hardware Template)** — `/core/layer_minus_one/`  
  Thin adapter contract for exotic substrates; capability + fallback declarations only.

- **Layer 0 (Safety Plane / Immutable Origin)** — `/core/layer0/`  
  Constitution, deterministic receipts, invariant enforcement, security gates, and root safety authority.

- **Layer 1 (Policy + Deterministic Receipts)** — `/core/layer1/`  
  Deterministic policy interpretation and receipt shaping.

- **Layer 2 (Scheduling + Execution)** — `/core/layer2/`  
  Execution orchestration, deterministic scheduling, queue/runtime coordination.

- **Layer 3 (OS Personality Template)** — `/core/layer3/`  
  Traditional OS growth surface (process/VFS/drivers/syscalls/namespaces/network/userland isolation).

- **Cognition Plane (Unnumbered)** — `/surface/orchestration/` + `/client/`  
  Orchestration Surface in `surface/orchestration/` for non-canonical execution coordination; Presentation Shell (compat alias: Client) in `client/` for UX and interaction shells.

### 2.1 Orchestration Surface Contract
Orchestration Surface code must be limited to:
1. Request normalization/classification and clarification policy.
2. Execution posture, sequencing, progress, retry/fallback, and result packaging.
3. Contract-bound calls into core authority paths (Tool Broker, Unified Memory, Task Fabric, Assimilation).
4. Transient sweepable orchestration context only.

Orchestration Surface must not canonize truth, persist private durable workflow state, or bypass core ingress contracts.

### 2.2 Shell Scope Contract (Developer-Only Surface, repo path `client/`)
Shell code must be limited to:
1. SDK/wrapper surfaces that call orchestration/core through conduit/lanes.
2. Developer experience tooling (CLI, templates, local orchestrators, diagnostics).
3. Developer-visible interfaces (observability UI, dashboards, docs, runbooks).
4. App construction and app hosting surfaces (`/apps` and client app glue).

Safety, policy, receipts, and system-truth authority remain in core.

Canonical Shell projection policy: `docs/workspace/shell_ui_projection_policy.md`.

Canonical Shell message/detail contract: `docs/workspace/shell_ui_message_detail_contract.md`.

Canonical Layered Nexus Federation Resolution policy: `docs/workspace/layered_nexus_federation_resolution_policy.md`.

Canonical Cross-Domain Nexus Route Inventory: `docs/workspace/cross_domain_nexus_route_inventory.md`.

Canonical Gateway ingress/egress interface policy: `docs/workspace/gateway_ingress_egress_policy.md`.

Canonical Interface Payload Budget policy: `docs/workspace/interface_payload_budget_policy.md`.

Canonical Shell-Independent Operation policy: `docs/workspace/shell_independent_operation_policy.md`.

Canonical Conduit/Scrambler Posture policy: `docs/workspace/conduit_scrambler_posture_policy.md`.

Shell UI/default API payloads must be bounded projections. Full runtime envelopes, raw tool payloads, traces, execution observations, workflow truth, policy decisions, and full-state durable caches are prohibited in default Shell rows.

Core, Orchestration Surface, CLI, and Gateway status must not require browser Shell assets such as dashboard HTML/CSS, Svelte bundles, browser static assets, DOM state, localStorage, or UI cache hydration.

### 2.3 Packages Scope Contract
Packages are the public distribution layer for InfRing-facing SDKs and starter surfaces.

- Packages may be polyglot.
- Packages may depend on stable shell SDK/CLI/runtime-entry surfaces (repo path `client`).
- Packages must not own policy, receipts, or canonical state.
- If a package starts making authority decisions, it is misplaced and must move into `core`.

### 2.4 Apps Scope Contract
Apps are not part of the platform core and are allowed to be more opinionated.

- Apps may be polyglot.
- Apps may depend on shell SDK/CLI/UI surfaces (repo path `client`).
- Apps must not become the canonical owner of policy, receipts, or core state.
- Apps should consume public platform contracts, not private core internals.

### 2.5 Gateways Scope Contract (compat alias: Adapters)
Gateways exist to connect InfRing to non-native systems.

- Gateways may be polyglot.
- Gateways may wrap third-party APIs, local tools, legacy services, or external applications.
- Gateways must remain capability-scoped and must not bypass conduit/policy/receipt contracts.
- If a gateway starts owning system truth, it is misplaced and must move into `core`.

### 2.6 Tests Scope Contract
Tests are a separate verification surface, with one exception:

- Unit tests may remain close to the code they verify.
- Integration, regression, system, chaos, and end-to-end tests should prefer `/tests`.

### 3. Language Rules
- `/core/`: Rust by default; C/C++ allowed only for approved low-level performance-critical or hardware-adjacent modules; shell allowed only for tightly-scoped build/install/packaging wrappers and never as safety authority.
- `/surface/`: Rust-first orchestration coordination contracts; orchestration surface tracked source should remain at least `95%` Rust, with TS/TSX wrappers allowed only for thin ingress/packaging interfaces.
- `/client/`: target state is TS/TSX + HTML/CSS frontend surfaces. JS/Python/Shell/PowerShell are tolerated only for explicitly-audited legacy shims, packaging helpers, or migration debt.
- `/packages/`: public SDK/package layer; polyglot is allowed, but packages stay thin and developer-facing.
- `/apps/`: polyglot by design.
- `/adapters/`: polyglot by design.
- `/tests/`: polyglot by design.
- No Rust/C/C++ in `/client/`.
- No TS/JS/Python/Shell in `/core/`.
- No JS/TS duplicate feature pairs. If both exist, TS is canonical and JS must be removed unless installer/deploy legacy is explicitly documented.
- No runnable app code under `client/cli/apps/`; runnable apps and demos must live under top-level `/apps`.
- No tracked runnable app code under `examples/apps/`; that path is legacy scratch only and must be migrated into `/apps`.

### 3.1 Public Platform Contract
The extension boundary for apps and adapters is:

- Conduit / lane-based runtime contract
- Shell SDK/CLI/UI surfaces derived from that contract
- Explicit manifests and schemas

Apps and gateways should build against the contract, not against private implementation files.

### 4. Boundary Rules (Enforced)
- Primary path is Shell -> Orchestration Surface -> Kernel for user-driven execution flows.
- Shell <-> core communication is conduit + scrambler only when explicitly approved ingress requires it.
- Orchestration Surface <-> core communication is conduit + scrambler + lease/policy validation.
- Nexus-Conduit-Checkpoint policy is canonical at `docs/workspace/nexus_conduit_checkpoint_policy.md`.
- Layered Nexus Federation is resolved by `docs/workspace/layered_nexus_federation_resolution_policy.md`; the old exact three-domain federation runtime shape is retired in favor of Layer 2 Nexus primitives plus checkpoint/Conduit/lease/lifecycle/receipt guards.
- Cross-domain route inventory is canonical at `docs/workspace/cross_domain_nexus_route_inventory.md`; new domain routes must be inventoried with source checkpoint, target checkpoint, Conduit path, posture, lease/capability, lifecycle gate, receipt requirement, and owner-of-truth declaration.
- Conduit/Scrambler Posture policy is canonical at `docs/workspace/conduit_scrambler_posture_policy.md`; sensitive authority-bearing routes must declare strong Scrambler posture and must not silently downgrade.
- Shell UI Projection policy is canonical at `docs/workspace/shell_ui_projection_policy.md`.
- Any module/domain boundary crossing must enter and exit through explicit Nexus checkpoint surfaces.
- Packages <-> core communication flows through public client/package contracts, never private authority backdoors.
- No direct client-side policy authority over core decisions.
- Apps/gateways must reach authority through platform contracts, not by importing private core internals.
- No direct back-channels, raw state bypasses, or legacy bridges around conduit.
- No direct code-file-to-code-file cross-module path may bypass its Nexus checkpoint surface unless tracked as a dated migration exemption with a replacement Nexus plan.
- Layer flow is upward-only:
  `Layer -1 -> Layer 0 -> Layer 1 -> Layer 2 -> Layer 3 -> Cognition`.

### 5. Runtime Data Placement
- Shell runtime/user/device/instance data: `client/runtime/local/`.
- Kernel runtime/user/device/instance data: `core/local/`.
- Source trees remain stable and reviewable; runtime churn never defines architecture authority.

### 6. Enforcement Rules
- No layer ownership changes without explicit user approval and audit note.
- CI/guards must fail on boundary violations.
- Architecture docs (`ARCHITECTURE.md`, `docs/SYSTEM-ARCHITECTURE-SPECS.md`, this rulebook) must remain synchronized.
- Shell boundary audit:
  - `npm run -s ops:shell-layer:boundary`
  - policy: `client/runtime/config/shell_layer_boundary_policy.json`
- Repo surface audit:
  - `npm run -s ops:repo-surface:audit`
  - policy: `client/runtime/config/repo_surface_policy.json`
- Public platform contract audit:
  - `npm run -s ops:public-platform:contract`
  - policy: `client/runtime/config/public_platform_contract_policy.json`
- Shell legacy debt inventory:
  - `npm run -s ops:client-legacy-debt:report`
  - emits a path-classified migration ledger for non-TS client debt
- Default verification path:
  - `./verify.sh` must run boundary + repo-surface + public-platform contract gates before origin-integrity checks

This rulebook is a live constitution artifact and must be kept aligned with the layered stack contract.

### 7. Module Cohesion and Split Policy (Enforced)
- The canonical policy is `docs/client/MODULE_COHESION_POLICY.md`.
- Kernel authority modules should split by domain boundary when they exceed safe reviewability.
- Shell surfaces must remain thin adapters and intentionally small/explicit.
- Size policy is guidance backed by CI:
  - hard cap envelope: ~400-600 lines (client thin cap: 400),
  - warning attention threshold: >800 lines,
  - allowed exception class: generated output and simple/stable adapter glue.
- Enforcement command:
  - `npm run -s ops:module-cohesion:audit`
- Verification path:
  - `./verify.sh` must run the module-cohesion gate before origin-integrity checks.
