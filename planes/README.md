# Three-Plane Metakernel

Protheus is structured as a substrate-independent metakernel with three explicit planes.

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
- Cognition plane implementation: `client/systems/*` user-facing and model orchestration surfaces.
- Substrate plane implementation: template adapters in `core/layer_minus_one/*` and capability descriptors under this directory.

Layer flow contract:

`Layer -1 -> Layer 0 -> Layer 1 -> Layer 2 -> Layer 3 -> Cognition`.

See `ARCHITECTURE.md` for the authoritative filesystem mapping and root-hygiene rationale.
