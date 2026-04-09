# InfRing Licensing Scope (Canonical)

This repository is dual-licensed and uses SPDX-first scope resolution.

- `Apache-2.0` for the open-core scope (commercial use allowed)
- `LicenseRef-InfRing-NC-1.0` for the default scope

## Canonical Matrix

The authoritative licensing matrix is machine-readable at:

- `LICENSE_MATRIX.json`

Human-readable resolution in this file is a mirror of that matrix.

## Resolution Precedence

1. File-level SPDX header (`SPDX-License-Identifier`) is authoritative.
2. Otherwise, apply the longest matching `path_prefix` rule in `LICENSE_MATRIX.json`.
3. If no path rule matches, default to `LicenseRef-InfRing-NC-1.0`.

## Path Scope Table

| Path Prefix | Effective SPDX |
| --- | --- |
| `core/layer_minus_one/` | `Apache-2.0` |
| `core/layer0/` | `Apache-2.0` |
| `core/layer1/` | `Apache-2.0` |
| `core/layer2/` | `Apache-2.0` |
| `adapters/protocol/` | `Apache-2.0` |
| `proofs/layer0/` | `Apache-2.0` |
| `LICENSE-APACHE-2.0` | `Apache-2.0` |
| all other tracked paths | `LicenseRef-InfRing-NC-1.0` |

## Release Artifact Labeling

Release and container metadata use:

- `Apache-2.0 AND LicenseRef-InfRing-NC-1.0`

## Commercial Licensing and Support

For commercial licensing of non-Apache paths and enterprise support terms, contact the InfRing maintainers.
