# Naming Cleanup Compatibility Notes (V11-TODO-005)

Purpose: normalize non-functional naming to architecture-first terminology without breaking compatibility.

## Canonical Terms

| Canonical | Compatibility Alias | Scope |
| --- | --- | --- |
| `Kernel` | `compatibility alias: Core` | Authority plane and public docs language |
| `Gateways` | `compatibility alias: Adapters` | External boundary layer and integration bridge language |
| `Shell` | `Client` | Presentation layer language and operator-facing docs |

## Compatibility Policy

1. Canonical docs use `Kernel`, `Gateways`, and `Shell`.
2. Existing command aliases remain supported until planned removal milestone.
3. Internal paths stay stable (`core/**`, `adapters/**`, `client/**`) until explicit path migration program is approved.
4. Naming guard enforcement mode is currently **yellow-flag/advisory** by default (`ops:*naming:guard`), with strict opt-in lanes available via `ops:*naming:guard:strict`.
5. Narrative language must be canonical-first on first mention:
   - `Kernel (compat alias: Core)`
   - `Gateways (compat alias: Adapters)`
   - `Shell (compat alias: Client)`

## Redirect Alias Notes

| Alias | Canonical Target | State |
| --- | --- | --- |
| `ops:core-naming:guard` | `ops:kernel-naming:guard` | supported compatibility alias |
| `core_*` artifact/report labels | `kernel_*` labels | mapped via compatibility policy |
| `adapter_*` labels | `gateway_*` labels | mapped via compatibility policy |

## Non-Functional Cleanup Rule

When renaming legacy competitor-style labels:

- do not change behavioral contracts
- keep compatibility aliases explicit
- document alias removal target in release checklist before deleting aliases
