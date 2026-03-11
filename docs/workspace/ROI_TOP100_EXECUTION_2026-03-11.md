# ROI Top 100 Execution Ledger (2026-03-11)

- Ordering basis: immediate policy/risk reduction first, then highest-score SRS regression-validated items.
- Execution rule: each move has concrete evidence path or regression result.

| Rank | Move | Type | Result | Evidence |
|---:|---|---|---|---|
| 1 | Move adaptive layer_store authority to core adaptive lane | implemented | done | `core/layer1/memory_runtime/adaptive/layer_store.ts` |
| 2 | Move adaptive habit_store authority to core adaptive lane | implemented | done | `core/layer1/memory_runtime/adaptive/habit_store.ts` |
| 3 | Move adaptive reflex_store authority to core adaptive lane | implemented | done | `core/layer1/memory_runtime/adaptive/reflex_store.ts` |
| 4 | Move adaptive strategy_store authority to core adaptive lane | implemented | done | `core/layer1/memory_runtime/adaptive/strategy_store.ts` |
| 5 | Move adaptive catalog_store authority to core adaptive lane | implemented | done | `core/layer1/memory_runtime/adaptive/catalog_store.ts` |
| 6 | Move adaptive focus_trigger_store authority to core adaptive lane | implemented | done | `core/layer1/memory_runtime/adaptive/focus_trigger_store.ts` |
| 7 | Backfill thin client wrappers for all moved adaptive stores | implemented | done | `client/runtime/systems/adaptive/**/{layer_store,habit_store,reflex_store,strategy_store,catalog_store,focus_trigger_store}.ts` |
| 8 | Extract shared adaptive UID primitive to reduce duplication | implemented | done | `core/layer1/memory_runtime/adaptive/uid.ts` |
| 9 | Tighten client layer boundary allowlist from 3 to 1 non-wrapper system file | implemented | done | `client/runtime/config/client_layer_boundary_policy.json` |
| 10 | Move ollama collector execution logic from client systems to adapters | implemented | done | `adapters/cognition/collectors/ollama_search.ts` |
| 11 | Leave thin client bridge for ollama collector | implemented | done | `client/runtime/systems/sensory/eyes_collectors/ollama_search.ts` |
| 12 | Tighten target contract exception cap from 8 to 1 | implemented | done | `client/runtime/config/client_target_contract_policy.json` |
| 13 | Update target contract disposition to mark ollama collector as adapter-owned | implemented | done | `client/runtime/config/client_target_contract_policy.json` |
| 14 | Move misplaced client vitest file to top-level tests surface | implemented | done | `tests/vitest/conduit_primitives_gap_closer.test.ts` |
| 15 | Update vitest include path to top-level tests only | implemented | done | `vitest.config.ts` |
| 16 | Patch conduit wrapper test contract to current wrapper architecture and pass suite | implemented | done | `tests/vitest/conduit_primitives_gap_closer.test.ts` |
| 17 | V6-COCKPIT-003: Resident ambient memory service snapshot (`REQ-32`) | regression-validated | existing-coverage-validated | score=440, non_backlog_evidence=1 |
| 18 | V6-COCKPIT-005: Subscribe bridge timeout elimination under host pressure | regression-validated | existing-coverage-validated | score=440, non_backlog_evidence=6 |
| 19 | V6-MEMORY-012: Full legacy daily-file XML hierarchy backfill | regression-validated | existing-coverage-validated | score=435, non_backlog_evidence=2 |
| 20 | V6-ARCH-032: Executable system map registry + generator (`REQ-34`) | regression-validated | existing-coverage-validated | score=412, non_backlog_evidence=1 |
| 21 | V6-COCKPIT-008: Rust/Node runtime unification contract (`REQ-37`) | regression-validated | existing-coverage-validated | score=380, non_backlog_evidence=5 |
| 22 | V6-COCKPIT-007: WebSocket-native conduit push stream (`REQ-37`) | regression-validated | existing-coverage-validated | score=375, non_backlog_evidence=5 |
| 23 | V2-027: Evolution arena (spawn-broker A/B variant trials) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=10 |
| 24 | V2-BRG-003: [V2] Client communication organ (light) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=4 |
| 25 | V3-004b: Dual-signature constitutional change gate | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=2 |
| 26 | V3-004c: Delayed activation + veto window | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=2 |
| 27 | V3-004d: Nursery red-team constitutional gauntlet | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=2 |
| 28 | V3-004e: Fractal inheritance lock enforcement | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=2 |
| 29 | V3-030: Tool assimilation pipeline (Research -> Forge -> Nursery -> Doctor graft) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=9 |
| 30 | V3-033: [V3] Sentinel protocol + confirmed-malice permanent quarantine lane | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=34 |
| 31 | V3-037: [V3] Verifiable Off-Device Memory Replication | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=4 |
| 32 | V3-058: [V3] Explanation Primitive (Human + Machine Verifiable) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=9 |
| 33 | V3-ASSIM-004: Memory Evolution Primitive (MemRL-inspired) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=20 |
| 34 | V3-ASSIM-025: Assimilation Policy Enforcement Closure (Wire or Retire Dead Knobs) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=7 |
| 35 | V3-BIN-001: [extension / V3] Master-vs-Child Instance Binary Bootstrap Policy | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=6 |
| 36 | V3-BLD-001: [primitive-upgrade / V3] Release Build Matrix Optimizer (LTO/PGO/musl/strip per target) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=17 |
| 37 | V3-DOC-004: [extension / V3] Interface Contract Registry + Schema Governance Pack Evidence: `client/runtime/systems/ops/backlog_runtime_anchors/v3_doc_004_anchor.ts`. | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=39 |
| 38 | V3-MEM-002: [primitive-upgrade / V3] Two-Stage Retrieval Gate (metadata -> selective body fetch) Evidence: `client/runtime/systems/ops/backlog_runtime_anchors/v3_mem_002_anchor.ts`. | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=22 |
| 39 | V3-MLC-001: [extension / V3] Default-On Master Training Conduit Lane | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=5 |
| 40 | V3-MLC-002: [hardening / V3] Hereditary + Master-Reviewed Federation Routing Evidence: `client/runtime/systems/ops/backlog_runtime_anchors/v3_mlc_002_anchor.ts`. | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=23 |
| 41 | V3-OPS-010: [extension / V3] Plugin/Capability Registry with Version Pinning | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=8 |
| 42 | V3-RACE-044: [extension / V3] Automated Compliance Mapping & Evidence Engine | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=59 |
| 43 | V3-RACE-141: [extension / V3] Content-Addressed Archival Plane (IPFS-Compatible) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=23 |
| 44 | V3-RACE-166: [extension / V3] A2A Delegation Plane (Agent-to-Agent Protocol Contracts) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=26 |
| 45 | V3-RACE-177: [primitive-upgrade / V3] System-3 Meta-Curriculum Bridge (`system3 -> strategy_learner/model_catalog_loop`) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=26 |
| 46 | V3-RACE-183: [extension / V3] Always-On Idle RSI Scheduler (Background Hands + Freshness Loop) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=22 |
| 47 | V3-RACE-194: [hardening / V3] Mobile Competitive Benchmark & CI Matrix (Battery/Thermal/72h Autonomy) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=17 |
| 48 | V3-RACE-208: [hardening / V3] Model Health Stabilizer (Adaptive Probe Timeouts + Temporary Suppression/Rehab) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=10 |
| 49 | V3-RACE-230: [extension / V3] Type-Derived Lane Docs Autogeneration (`typedoc` + `cargo-doc`) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=18 |
| 50 | V3-RACE-249: [extension / V3] visionOS Spatial Runtime + Vision Framework Adapter | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=44 |
| 51 | V3-RACE-261: [hardening / V3] Apple Release Security Gate (App Sandbox/TCC + Notarization/Gatekeeper + Privacy Manifest) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=34 |
| 52 | V3-RACE-264: [extension / V3] GKE Autopilot + Anthos Federation Adapter | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=26 |
| 53 | V3-RACE-271: [extension / V3] Jetpack Compose + Material You + ARCore/Starline Interaction Adapter | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=26 |
| 54 | V3-RACE-279: [hardening / V3] NVIDIA Enterprise Licensing/Compliance Support Lane | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=31 |
| 55 | V3-RACE-286: [extension / V3] Intra-Host Massive Multi-GPU Federation Lane (NVSwitch/NVLink) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=14 |
| 56 | V3-RACE-339: [hardening / V3] 15-Year LTS Maintenance Alignment Lane (Helix Lifecycle Contract) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=20 |
| 57 | V3-RACE-340: [hardening / V3] Compliance Hardening Pack (FIPS 140-3 + CC EAL4+ + DISA STIG Automation) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=20 |
| 58 | V3-RACE-353: [hardening / V3] Long-LTS Migration & Support Continuity Lane (Ubuntu/Enterprise Linux Focus) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=14 |
| 59 | V3-RACE-360: [hardening / V3] Socket Admission Proof Gate (Formal + Redteam + HostProfile Chaos Validation) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=14 |
| 60 | V3-RTE-001: [primitive-upgrade / V3] Warm-Start Snapshot Restore for Sub-Second Boot | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=5 |
| 61 | V3-USE-001: [extension / V3] First-Run Onboarding Doctor (`protheus init`) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=6 |
| 62 | V4-SUITE-002: [extension / V4] `protheus-mem` Long-Memory CLI Surface | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=14 |
| 63 | V4-SUITE-010: [launch-polish / V4] `protheus-soul` Public Export Mode | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=9 |
| 64 | V5-RUST-PROD-002: [hardening / V5] TS/Rust Boundary Contract + ABI/Schema Stability Gate | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=43 |
| 65 | V5-RUST-PROD-012: [scale-readiness / V5] Rust-at-Scale Capacity + Unit Economics Validation | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=23 |
| 66 | V6-F100-A-007: Executable A+ procurement readiness scorecard gate | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 67 | V6-HOST-BUILD-STALE-001: Host Rust build-stall guard stabilization | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 68 | V6-ORIGIN-002: Conduit-only + constitution hardening gate in origin checks | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 69 | V6-ORIGIN-003: Receipt binding to exact safety-plane state | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 70 | V6-ORIGIN-005: Seed bootstrap verification via `verify.sh` certificate | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 71 | V6-PRIM-004: [primitive-upgrade / V6] IPC Primitive Rust Completion | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=5 |
| 72 | V6-SEC-001: [hardening / V6] Audited Release + SBOM Bundle (`v0.2.0`) | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=22 |
| 73 | V6-SEC-004: [governance / V6] Independent Security Audit Publication | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=19 |
| 74 | V6-TECH-301: Formal three-plane spec surface + guard | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 75 | V6-TECH-302: Inter-plane contract source-of-truth scaffold | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 76 | V6-TECH-303: Architecture + verify integration for formal surfaces | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 77 | V6-TECH-304: Full formal proof runtime lane in CI | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 78 | V6-TECH-305: Reproducible benchmark/proof-pack threshold enforcement | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 79 | V6-VALIDATION-HOST-001: Full non-skipped host validation pack execution | regression-validated | existing-coverage-validated | score=370, non_backlog_evidence=1 |
| 80 | V6-COMP-005: Public multi-release cadence target (9+ tags) | regression-validated | existing-coverage-validated | score=350, non_backlog_evidence=11 |
| 81 | V6-SUBSTRATE-002.4: Bioethics/consent policy gate contract for biological interaction | regression-validated | existing-coverage-validated | score=336, non_backlog_evidence=3 |
| 82 | V7-META-016: Live QPU provider validation campaign | regression-validated | existing-coverage-validated | score=326, non_backlog_evidence=4 |
| 83 | V7-META-017: Ternary hardware lowering validation campaign | regression-validated | existing-coverage-validated | score=326, non_backlog_evidence=4 |
| 84 | V7-META-018: Neural I/O safety validation campaign | regression-validated | existing-coverage-validated | score=326, non_backlog_evidence=5 |
| 85 | V6-COCKPIT-009: Fully deferred identity hydration (`REQ-37`) | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=22 |
| 86 | V6-COCKPIT-009.1: OpenDev dual-agent architecture (Planner + Executor) | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=3 |
| 87 | V6-COCKPIT-009.2: OpenDev lazy tool discovery lane | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=3 |
| 88 | V6-COCKPIT-009.3: OpenDev adaptive context compaction engine | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=3 |
| 89 | V6-COCKPIT-009.4: OpenDev event-driven reminder injection lane | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=3 |
| 90 | V6-COCKPIT-009.5: OpenDev cross-session automated memory reconstruction | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=4 |
| 91 | V6-COCKPIT-009.6: OpenDev strict autonomous safety controls for coding execution | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=3 |
| 92 | V6-COCKPIT-010: Integrity reseal auto-clear lane for trusted update classes (`REQ-37`) | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=4 |
| 93 | V6-COCKPIT-011: Moltbook skill-path contract repair + heartbeat verification lane (`REQ-37`) | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=15 |
| 94 | V6-COCKPIT-012.1: Workspace bootstrap memory contract (`AGENTS/SOUL/USER/MEMORY/HEARTBEAT/TOOLS + daily logs`) | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=4 |
| 95 | V6-COCKPIT-012.2: Gateway daemon runtime with persistent connector sessions + WebSocket UI bridge | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=4 |
| 96 | V6-COCKPIT-012.3: `dmScope` isolation contract (per-agent / per-channel-peer context fences) | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=3 |
| 97 | V6-COCKPIT-012.4: Native cron + heartbeat automation envelope for detached autonomy | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=3 |
| 98 | V6-COCKPIT-012.5: Safe exec/browser tool-mode governance (`sandbox`/`gateway`/`full`) + conduit receipts | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=3 |
| 99 | V6-COCKPIT-012.6: Session-classified workspace memory visibility contract (`main` vs `shared`) | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=1 |
| 100 | V6-COCKPIT-012.7: Unified tool-mode matrix for `exec/browser/file/message` surfaces | regression-validated | existing-coverage-validated | score=320, non_backlog_evidence=1 |
