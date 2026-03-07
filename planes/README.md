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

- Safety plane implementation: `core/layer0`, `core/layer1`, `core/layer2`.
- Cognition plane implementation: `client/systems/*` user-facing and model orchestration surfaces.
- Substrate plane implementation: backend adapters in `core/layer0/*` and capability descriptors under this directory.
