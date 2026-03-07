# TODO

## Backlog Follow-Up (Layer Ownership Guard)

- [ ] `V6-ADAPT-CORE-001` Port adaptation primitives from temporary client bootstrap to core authority.
  - Layer target: `core/layer2` (authoritative runtime primitive for `REQ-19-001`, `REQ-19-002`, `REQ-19-003`).
  - Client role: Layer 3 conduit-only wrappers, operator CLI surface, and tests.
  - Completion criteria:
    - Rust core owns cadence/resource/continuity policy and receipts.
    - Client runtime adaptation code is compatibility-only (no policy authority).
    - All client↔core communication for adaptation flows only through conduit + scrambler.
  - Progress:
    - Core lane scaffold shipped: `protheus-ops adaptive-runtime <tick|status>` with deterministic receipts.
    - Client thin conduit shell shipped: `systems/adaptive/adaptive_runtime.{ts,js}`.

- [ ] `V6-CONDUIT-RUNTIME-STALL-001` Resolve local Rust binary startup stall impacting conduit-lane execution.
  - Layer target: `core/layer2/conduit` + `core/layer0/ops` runtime startup path.
  - Symptoms:
    - `conduit_stdio_timeout` on spine/status and mech benchmark preflight.
    - Rust binaries remain non-responsive in this environment until forcibly killed.
  - Completion criteria:
    - `conduit_daemon` responds to `start_agent` within timeout budget.
    - `ops:mech-suit:benchmark` completes without preflight host fault.
    - `formal:invariants:run` + conduit bridge smoke tests pass with live Rust lane.

- [x] `LOCAL-PARTITION-001` Migrate mutable runtime paths into unified local partitions.
  - Partition standard:
    - `client/local/` for user/device/instance client runtime artifacts.
    - `core/local/` for node-local core runtime artifacts.
  - Scope:
    - Migrate generated state/logs/secrets/private-lens/runtime adaptive outputs from legacy paths.
    - Keep source/test/docs artifacts in their canonical source directories.
  - Completion criteria:
    - Runtime writes default to `client/local/*` and `core/local/*`.
    - Legacy path reads remain as compatibility fallback during transition.
    - Reset command can wipe local partitions without touching source code.
  - Completed deliverables:
    - `protheusctl migrate-to-planes` (`plan|run|status`) shipped.
    - `client/tools/migrate_to_planes_runtime.sh` shipped for one-command migration bootstrap.
    - `local_runtime_partitioner` init/status/reset shipped + tested.
    - Benchmark/harness defaults moved to `client/local/state/*` for generated artifacts.
