# InfRing Layer Rulebook — Strict Enforcement Policy
**Version 1.1** — March 2026  
**This is the source of truth for file placement, language boundaries, and layer ownership. No deviations without explicit user approval.**

### 1. Directory Split (Enforced)
The repository has two top-level source code roots:
- `/core` — deterministic core stack (`layer_minus_one`, `layer0`, `layer1`, `layer2`, `layer3`) and trusted low-level logic.
- `/client` — cognition/user-facing surfaces, SDKs, scripts, and extensions.

All source code must live in one of these roots.

Allowed root-level exceptions (metadata/infrastructure): `.github/`, `.githooks/`, policy docs, lockfiles, build manifests, deploy manifests.

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

- **Cognition Plane (Unnumbered)** — `/client/`  
  TS/JS/Python/Shell/PowerShell/HTML/CSS surfaces for user-facing and probabilistic workflows.

### 3. Language Rules
- `/core/`: Rust by default; C/C++ only for approved low-level performance-critical modules.
- `/client/`: TS/JS/Python/Shell/PowerShell/HTML/CSS only.
- No Rust/C/C++ in `/client/`.
- No TS/JS/Python/Shell in `/core/`.
- No JS/TS duplicate feature pairs. If both exist, TS is canonical and JS must be removed unless installer/deploy legacy is explicitly documented.

### 4. Boundary Rules (Enforced)
- Client <-> core communication is conduit + scrambler only.
- No direct client-side policy authority over core decisions.
- No direct back-channels, raw state bypasses, or legacy bridges around conduit.
- Layer flow is upward-only:
  `Layer -1 -> Layer 0 -> Layer 1 -> Layer 2 -> Layer 3 -> Cognition`.

### 5. Runtime Data Placement
- Client runtime/user/device/instance data: `client/local/`.
- Core runtime/user/device/instance data: `core/local/`.
- Source trees remain stable and reviewable; runtime churn never defines architecture authority.

### 6. Enforcement Rules
- No layer ownership changes without explicit user approval and audit note.
- CI/guards must fail on boundary violations.
- Architecture docs (`ARCHITECTURE.md`, `docs/SYSTEM-ARCHITECTURE-SPECS.md`, this rulebook) must remain synchronized.

This rulebook is a live constitution artifact and must be kept aligned with the layered stack contract.
