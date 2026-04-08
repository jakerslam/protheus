# Packages

Public SDK and package distribution surfaces live here.

Rules:
- Packages are developer-facing, not authority-bearing.
- Packages may be polyglot.
- Packages may call stable `client` contracts and wrappers.
- Packages must not own policy, receipts, or canonical state.
- If a package starts deciding system truth, it belongs in `core`.

Current primary public surface:
- `@infring/sdk` (`packages/infring-sdk`) — stable task/receipt/memory/evidence/assimilation/policy contract.
