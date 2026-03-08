# REQ-31 — Layered Templates + OS Personality Expansion (Doc `1DAcIgyea82TMno09C8NIHbjJvB6CWA9XUeTmPPsMcf0`)

Status: in_progress  
Owner: Architecture + Runtime Foundation  
Updated: 2026-03-08

## Source
- Intake document: `https://docs.google.com/document/d/1DAcIgyea82TMno09C8NIHbjJvB6CWA9XUeTmPPsMcf0/edit`
- Canonical contract: `docs/SYSTEM-ARCHITECTURE-SPECS.md`

## Objective
Adopt the final layered core stack:
- add Layer -1 as the exotic hardware template boundary,
- preserve Layer 0 as immutable safety origin,
- formalize Layer 3 as the OS personality growth layer,
- and enforce strict upward-only information flow to cognition surfaces.

## Derived Executable Requirements

### REQ-31-001 Upward-only layer flow contract
- Enforce layer flow direction:
  `Layer -1 -> Layer 0 -> Layer 1 -> Layer 2 -> Layer 3 -> Cognition`.
- No downward calls or bypass channels.

### REQ-31-002 Layer -1 exotic substrate template contract
- Maintain `core/layer_minus_one/` as first-class template ownership path.
- Expose an exotic substrate adapter contract with:
  - envelope execution,
  - capability declaration,
  - degradation fallback declaration.

### REQ-31-003 Layer 0 immutability and invariant preservation
- Layer 0 public contract cannot be weakened.
- Receipts remain bound to Layer 0 state.
- Constitution + RSI + self-audit authority remain in Layer 0.

### REQ-31-004 Layer 3 OS personality contract
- Maintain `core/layer3/` as dedicated OS personality growth path.
- Scope includes process model, VFS, drivers, syscall surface, namespaces, memory/userland isolation, and networking/windowing contracts.

### REQ-31-005 Backward-compatible migration sequence
- Layered migration must preserve existing runtime behavior and proofs while stack ownership is expanded.
- Migration artifacts must remain deterministic and audit-friendly.

### REQ-31-006 Architecture document synchronization
- `ARCHITECTURE.md`, `docs/SYSTEM-ARCHITECTURE-SPECS.md`, `planes/README.md`, and `docs/client/architecture/LAYER_RULEBOOK.md` must stay aligned with the same layer map.

## Human-Owned Requirements (Non-Automatable)

### REQ-31-H001 Invariant change approval authority
- Any proposed Layer 0 contract change requires explicit human approval with audit rationale.

### REQ-31-H002 External architecture claim publication
- Public positioning around “full OS personality” and future substrate claims requires human legal/brand approval.

## Acceptance Criteria
1. The architecture spec explicitly defines Layer -1 and Layer 3 with responsibilities and flow rules.
2. Architecture docs no longer classify `/client` as “Layer 3.”
3. `core/layer_minus_one/` and `core/layer3/` exist as explicit ownership anchors.
4. Flow rule text is consistent across architecture docs.
5. Requirement linkage is present in `SRS.md`.

## Implementation Notes
- This requirement establishes architecture contracts and scaffolding.  
- Full runtime trait wiring and conformance tests remain tracked as follow-on execution under the same requirement family.
