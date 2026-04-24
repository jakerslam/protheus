# Layer 3 — OS Personality Template

Purpose:
- Host traditional operating-system personality contracts on top of the deterministic lower stack.

Scope examples:
- Process lifecycle and isolation model
- Execution-unit identity, lifecycle, budget, dependency, and receipt tracking
- VFS/filesystem contract surface
- Driver registration and syscall dispatch surfaces
- Namespace, networking, and userland abstraction contracts

Rules:
- Consume lower-layer guarantees; do not bypass Layer 0 invariants.
- Upward-only flow: Layer 2 -> Layer 3 -> Cognition/Conduit surfaces.
- New Layer 3 source files must be mapped in `tests/tooling/config/layer3_contract_policy.json`.
- CI enforcement is fail-closed through `ops:layer3:contract:guard`.

Primary implementation:
- `core/layer3/os_extension_wrapper` (Rust crate)

Canonical contract:
- `docs/workspace/process/layer3_contract.md`
