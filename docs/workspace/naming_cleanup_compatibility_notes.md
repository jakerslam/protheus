# Naming Cleanup Canonical Notes (V11-TODO-005)

Purpose: normalize non-functional naming to architecture-first terminology with canonical-only enforcement.

## Canonical Terms

| Canonical | Scope |
| --- | --- |
| `Kernel` | Authority plane and public docs language |
| `Gateways` | External boundary layer and integration bridge language |
| `Shell` | Presentation layer language and operator-facing docs |

## Canonical Policy

1. Canonical docs use `Kernel`, `Gateways`, and `Shell`.
2. Command aliases are retired; only canonical command IDs are supported.
3. Internal paths stay stable (`core/**`, `adapters/**`, `client/**`) until explicit path migration program is approved.
4. Naming guard enforcement mode is currently **yellow-flag/advisory** by default (`ops:*naming:guard`), with strict opt-in lanes available via `ops:*naming:guard:strict`.
5. Narrative language must be canonical-only.

## Non-Functional Cleanup Rule

When renaming legacy labels:

- do not change behavioral contracts
- keep canonical naming explicit
- document any temporary break-glass alias reintroduction with expiry
