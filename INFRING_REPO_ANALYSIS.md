# InfRing — Repository Analysis

_Analyzed: 2026-04-26 · branch `main` · version `0.2.1-alpha.1`_

## What this project is

InfRing is a **deterministic, receipt-first autonomous runtime** organized as a **three-plane metakernel**. Its central design claim is that a Rust kernel is the single source of truth for "what is allowed and true," while everything probabilistic (LLMs, agent reasoning, UI) is pushed out to non-authoritative surfaces that can only cross the boundary through a single conduit. The product identity is built around a single root (`~/.infring`) and a binary family (`infring`, `infringctl`, `infringd`, `infring-top`), with a default operator dashboard at `http://127.0.0.1:4173/dashboard#chat`.

The repo is dual-licensed (`LICENSE_SCOPE.md` + `LICENSE_MATRIX.json`): the kernel layers and protocol adapters are Apache-2.0, and everything else falls under a custom non-commercial license (`LicenseRef-InfRing-NC-1.0`) unless an SPDX header overrides it.

## High-level architecture

The architecture is documented authoritatively in `ARCHITECTURE.md`, `planes/README.md`, and `docs/SYSTEM-ARCHITECTURE-SPECS.md`. The three planes are:

- **Safety plane** — deterministic authority, fail-closed. Implemented in `core/`.
- **Cognition plane** — coordination + presentation. Implemented in `surface/orchestration/` (Rust-first, ≥95% Rust by tracked SLOC) and `client/` (TypeScript shell).
- **Substrate plane** — descriptors and degradation contracts for CPU/MCU/GPU/NPU/QPU/neural backends; template adapters live in `core/layer_minus_one/`.

The **boundary axiom** is stated three ways across the docs: kernel decides what is true and allowed; orchestration decides what should happen next; shell decides how it is shown and collected. Every cross-boundary call must traverse the **conduit + scrambler** (`core/layer2/conduit`, `core/layer2/conduit-security`). There is no policy authority in TS shells — they're explicitly described as "thin" presentation/wrapper code.

The kernel itself is layered with a hard upward-only flow:

```
Layer -1  (exotic hardware template)         core/layer_minus_one/
   ↓
Layer  0  (immutable safety origin)          core/layer0/        ← 25 crates
   ↓
Layer  1  (policy + deterministic receipts)  core/layer1/        ← 17 crates
   ↓
Layer  2  (scheduling + execution)           core/layer2/        ← conduit, execution, nexus, ops, …
   ↓
Layer  3  (OS personality template)          core/layer3/
   ↓
Cognition (never root-of-correctness)        surface/orchestration/, client/
```

Layer 0 is sacred/immutable and proof-preserving; Layer -1 normalizes exotic substrates into a standard envelope; Layer 3 is where a more traditional OS surface (processes, VFS, drivers, syscalls, namespaces) is allowed to grow. Cognition is explicitly _outside_ the numbered kernel stack.

## Workspace layout

The repo is a polyglot Cargo + npm workspace. The Cargo workspace (root `Cargo.toml`) is configured with a glob over `core/layer_minus_one/*`, `core/layer0/*`, `core/layer1/*`, `core/layer2/*`, `core/layer2/tools/task_fabric`, and `core/layer3/*`, plus `xtask` — about **57 crates** in total. MSRV is pinned at 1.84.0 via `workspace.metadata.infring`, the toolchain file pins stable, and the release profile is size-optimized (`opt-level = "z"`, `panic = "abort"`, thin LTO, single codegen unit) with `release-speed` and `release-minimal` variants.

The npm side is a single root `package.json` (no `workspaces` field) with TypeScript, Vitest, and Svelte as primary dev dependencies and an unusually large surface of **1,069 npm scripts**. The script namespaces tell their own story about what the repo prioritizes operationally:

| Namespace | Scripts | What it covers |
|---|---|---|
| `ops:*` | 473 | Production operator commands — topology, status, evidence, release readiness |
| `test:*` | 163 | Layered test surface (unit/integration/CI/regression/parity) |
| `security:*` | 37 | Boundary guards, deny-lists, scope policy |
| `autonomy:*`, `sensory:*`, `memory:*`, `symbiosis:*`, `helix:*`, `xai:*`, `redteam:*` | 9–28 each | Agent/cognition lanes — clearly the part of the system being aggressively scaffolded |

Top-level source roots beyond `core/`:

- **`surface/orchestration/`** — Rust-authoritative orchestration control plane (decomposition, sequencing, recovery, packaging). The doc tracks an explicit Rust SLOC ratio for this folder.
- **`client/`** — the presentation shell: `cli/` (operator entrypoints), `runtime/` (systems, lib, config, observability), `cognition/` (reflexes, sensors, adaptive surfaces), plus `instinct/`, `memory/`, `skills/`, `pure-workspace/`, `types/`.
- **`adapters/`** — gateway layer (`cognition/`, `economy/`, `importers/`, `polyglot/`, `protocol/`, `runtime/`, `skills/`).
- **`apps/`** — 22 thin "first-party" surfaces over Rust app-planes: `chat_starter`, `chat_ui`, `code_engineer`, `intelligence-nexus`, `local-rag`, `local-research-agent`, `personas`, `snowball_engine`, `sovereign-memory-os`, `video-ad-factory`, etc. These are explicitly described as thin shells; the substantive logic lives in core.
- **`packages/`** — distribution packages: `infring-core`, `infring-edge`, `infring-npm`, `infring-py`, `infring-sdk`, plus a parallel `protheus-*` family (a sibling/forked product line) and `lensmap`.
- **`planes/`** — living architectural contracts (`safety/`, `cognition/`, `substrate/`, `contracts/`, `spec/`).
- **`tools/`, `xtask/`, `setup/`, `tests/`, `benchmarks/`, `proofs/`, `docs/`** — supporting surfaces. `proofs/layer0/` is treated as authoritative evidence and is explicitly Apache-2.0 like the kernel.

Repo root also carries a notable amount of governance metadata (`AGENTS.md`, `GOVERNANCE.md`, `IDENTITY.md`, `SOUL.md`, `HEARTBEAT.md`, `DECISIONS_INDEX.md`, `CITATION.cff`) — this is a project that treats its own contributor process as a first-class artifact.

## Operator + runtime model

The user-facing entrypoint is the `infring` bash wrapper in `client/cli/bin/infring`, with `infringctl` and `infringd` siblings. It resolves a Rust binary by trying, in order: `INFRING_NPM_BINARY`, a vendored npm-shipped binary at `client/cli/npm/vendor/infring-ops`, then `target/debug/infring-ops`, then `target/release/infring-ops`. There is also a family of TypeScript subcommand entrypoints (`infring-graph.ts`, `infring-mem.ts`, `infring-soul.ts`, `infring-swarm.ts`, `infring-vault.ts`, `infring-pinnacle.ts`, etc.) — these correspond to the agent/memory/swarm subsystems exposed in `core/layer0`.

The Dockerfile is a clean four-stage build:

1. `deps-dev` and `deps-prod` install npm deps off `node:22-alpine`.
2. `dist-builder` runs `npm run -s runtime:dist:dashboard:build`.
3. `rust-builder` (rust:1.89-alpine + musl) builds `infring-ops` and `infringd` from `core/layer0/ops/Cargo.toml`.
4. The final image bundles both binaries plus the dashboard JS, drops to a non-root `infring` user, and exposes 4173 with a `/healthz`-based HEALTHCHECK. Default CMD serves the dashboard.

The README's "Current State (April 2026)" section sets the production support contract explicitly: rich profile is canonical, `--pure` and `--tiny-max` are constrained Rust-only profiles (they intentionally do _not_ expose the rich gateway UI), `assimilate` is experimental opt-in, and **resident-IPC is the only supported production topology** — process-transport fallbacks are blocked at production with deny-codes (`process_transport_forbidden_in_production`, `process_fallback_forbidden_in_production`). The legacy process runner is quarantined under `adapters/runtime/dev_only/**` with a dated removal target (`v0.3.11-stable` / `2026-05-15`).

## Notable design choices

A few things stand out as defining decisions, beyond the obvious "Rust kernel + thin shell":

**Receipt-first execution.** Layer 1 is named "Policy + Deterministic Receipts," the release pipeline now produces split synthetic-canary vs. empirical-live runtime-proof artifacts (`runtime_proof_synthetic_canary_current.json`, `runtime_proof_empirical_release_evidence_current.json`, `runtime_proof_empirical_trends_current.json`), and release proof packs are assembled as grouped checksummed artifacts under `releases/proof-packs/<version>/`. This is an evidence-heavy release model.

**Cognition is gated behind "REQ-27" attention/initiative authority.** Importance scoring lives in `core/layer0/ops/src/importance.rs`, priority queues in `core/layer0/ops/src/attention_queue.rs`, and there is an explicit regression guard (`client/runtime/systems/ops/subconscious_boundary_guard.ts`) whose job is to prevent the shell from accumulating subconscious authority. The architecture goes out of its way to mechanize the rule that the shell can never become root-of-correctness.

**Memory is a first-class subsystem, not a database wrapper.** `client/runtime/systems/memory/` contains a memory-matrix, dream sequencer, and auto-recall lane; nodes are tagged with a level taxonomy (`node1` > `tag2` > `jot3`) and weekly admission is quota-bounded. The `conversation_eye` collector/synthesizer is described as a default cognition-plane sensor that auto-provisions on `local:init` and writes JSONL nodes. Memory recall queries enforce a hard context-budget contract (`--context-budget-tokens` default 8000, floor 256, with `trim` or `reject` modes).

**Three runtime profiles, real differences.** `rich` (full Node + dashboard), `pure` (Rust-only), `tiny-max` (Rust-only minimal) — these aren't just feature flags; the constrained profiles deliberately do _not_ ship the gateway UI surface, and the Rust toolchain targets `x86_64-unknown-linux-musl` to keep tiny-max clean.

**Graduation manifests for the gateway.** Release readiness is gated by manifest-backed graduation checks for hooks and chaos scenarios, and Layer 2 parity guard requires every listed lane to be explicitly marked `complete` — provisional lanes block release.

**Identity-as-files.** `IDENTITY.md`, `SOUL.md`, `HEARTBEAT.md`, `USER.md` and the `AGENTS.md` workspace contract are part of the repo itself, not a wiki. The agent contract instructs assistants to read these on every session and to write durable memory back into `local/workspace/memory/YYYY-MM-DD.md` and `local/workspace/assistant/MEMORY.md`. There's a strict "root output hygiene" policy that bans shadow agents from creating churn directories at root.

## Current trajectory (signal from recent commits)

The last 15 commits on `main` are all hardening/retirement work — gateway startup idempotency, agent attention ingress policy, moving shell authority config back to core, runtime/eval/planner governance hardening, removing a retired client bridge, removing the Svelte dashboard sveltekit surface (replaced by Svelte islands), and ongoing "alpine retirement" of the legacy shell ownership model. There are zero net-new feature commits in the recent window. The project is in a **boundary-tightening and consolidation phase**, not a build-out phase.

## Strengths

- Architectural intent is unusually crisp and consistently restated across `ARCHITECTURE.md`, `planes/README.md`, the README, and inline docs. The three-plane model is enforced by file-system layout, not just convention.
- Hard-coded boundary regressions guards (`subconscious_boundary_guard.ts`, deny-codes for production transport, layer-2 parity guard) make the architecture self-policing.
- Release evidence is treated as a first-class artifact stream rather than as CI ephemera.
- The dual-license matrix is machine-readable and SPDX-first, which is the right way to do this.

## Risks / things that warrant scrutiny

- **Surface area is enormous.** 57 Rust crates, 22 apps, 11 distribution packages, 1,069 npm scripts. That much script sprawl is a maintenance liability on its own; many of the `ops:*` lanes likely overlap.
- **Workspace fragmentation on the npm side.** The root `package.json` has no `workspaces` field despite the repo containing `packages/`, `apps/`, and `client/` — TypeScript composition is being managed by `tsconfig.runtime.json` / `tsconfig.sdk.references.json` instead. Worth checking whether the apps and packages are actually being built coherently or are quietly drifting.
- **Two product lineages co-resident.** `packages/infring-*` and `packages/protheus-*` (and a top-level `protheus-sim/` directory) suggest a forked or adjacent product is being maintained in the same tree; there's no documentation at the root explaining the relationship.
- **A lot of "thin shell over core" claims.** Worth spot-checking that the apps under `apps/` actually are thin and aren't accumulating their own logic or local state.
- **Legacy paths still live.** `adapters/runtime/dev_only/legacy_process_runner.ts` has a 2026-05-15 deletion target — close to today's date (2026-04-26). Worth verifying the burn-down is on track.
- **`.git` is tracked at ~22k+ artifacts in `artifacts/`** (the directory has 711 immediate entries). That's a lot of generated state living in the repo; the README acknowledges `artifacts/` as a "managed support zone" but it's worth knowing that root listings are noisy because of it.

## Where to look next, by question

- "How does a request actually flow through?" → `core/layer2/conduit/src/` and `core/layer2/conduit-security/`, plus the client-side caller in `client/runtime/lib/runtime_path_registry.ts`.
- "What's the kernel actually authoritative about?" → `core/layer0/ops/`, `core/layer1/primitives/`, `core/layer1/provenance/`.
- "How does the agent/cognition stack work?" → `core/layer0/swarm*`, `core/layer0/persona_dispatch_security_gate`, `client/cognition/`, `apps/intelligence-nexus`.
- "What's the production gate?" → `npm run -s ops:status:production` is named in the README as the single operator truth command, plus `releases/proof-packs/<version>/` for the evidence artifacts.
- "What's about to be deleted?" → `adapters/runtime/dev_only/`, the alpine retirement work in `client/`, anything tagged `provisional` in the Layer 2 parity manifest.

## Bottom line

InfRing is a serious, opinionated systems project with a coherent architectural thesis (deterministic Rust kernel + thin probabilistic surfaces, with every boundary mechanized as either a contract, a guard, or a receipt). The code reflects the docs, the docs reflect the code, and the recent commit history shows the team is currently in a tightening/retirement cycle rather than a feature-expansion cycle. The main risks are operational sprawl — too many scripts, too many apps, two product lines in one tree — rather than architectural confusion.
