# Client Reflexes

Low-burn reflex helpers for frequent operator actions.

## Reflex IDs
- `read_snippet`
- `write_quick`
- `summarize_brief`
- `git_status`
- `memory_lookup`

Each reflex is hard-capped at `<=150` estimated tokens.

## Usage
```bash
node client/cognition/reflexes/index.js list
node client/cognition/reflexes/index.js run --id=memory_lookup --input="ambient mode regression"
```
