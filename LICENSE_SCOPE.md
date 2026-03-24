# InfRing Dual-License Scope Map

This repository uses dual licensing.

- `Apache-2.0` for the Open Core Scope (commercial use allowed)
- `InfRing-NC-1.0` (Protheus Non-Commercial License v1.0) for default scope

## Scope Resolution

1. If a file contains an explicit SPDX identifier, that identifier is authoritative.
2. Otherwise, apply the path-based scope below.
3. If no rule matches, default to `InfRing-NC-1.0`.

## Apache-2.0 Open Core Scope

- `core/layer_minus_one/**`
- `core/layer0/**`
- `core/layer1/**`
- `core/layer2/**`
- `adapters/protocol/**`
- `proofs/layer0/**`

## InfRing-NC-1.0 Default Scope

Everything else in this repository, including but not limited to:

- `client/**`
- `apps/**`
- `docs/**`
- `adapters/**` paths not listed in the Apache scope
- top-level operational scripts/config not explicitly marked Apache

## Commercial Licensing and Support

For commercial licensing of non-Apache paths and enterprise support terms, contact Protheus Labs.
