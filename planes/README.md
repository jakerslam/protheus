# Three-Plane Metakernel

InfRing is structured as a substrate-independent metakernel with three explicit planes.

## Plane Contracts

- `planes/safety/`: deterministic authority plane.
- `planes/cognition/`: probabilistic cognition/userland plane.
- `planes/substrate/`: execution backend descriptors and degradation contracts.

## Hard Rules

1. Safety authority remains deterministic and fail-closed.
2. AI/probabilistic logic is never root-of-correctness.
3. Client to core communication only crosses the conduit + scrambler boundary.
4. Every substrate must declare degradation/fallback behavior.

## Current Mapping

- Safety plane implementation stack: `core/layer_minus_one`, `core/layer0`, `core/layer1`, `core/layer2`, `core/layer3`.
- Cognition plane implementation: `surface/orchestration/*` (Orchestration Surface coordination) and `client/runtime/systems/*` (Presentation Client surfaces).
- Substrate plane implementation: template adapters in `core/layer_minus_one/*` and capability descriptors under this directory.
- Layered Nexus federation: `core/layer0/nexus/*` (Core, Orchestration Surface, and Client central-domain routing with lease-bound inter-domain delivery).

Layer flow contract:

`Layer -1 -> Layer 0 -> Layer 1 -> Layer 2 -> Layer 3 -> Cognition`.

## Product Identity Contract

- Filesystem root: `~/.infring` (single canonical runtime root).
- Binary namespace: `infring*` (`infring`, `infringctl`, `infringd`, `infring-top`).
- Service naming: `infring` (docker service/container/image prefixes use `infring`).

See `ARCHITECTURE.md` for the authoritative filesystem mapping and root-hygiene rationale.
