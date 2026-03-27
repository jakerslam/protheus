# Repo File Size Policy

- Scope: `core/**`, `client/**`
- Tests exempt: yes (`**/tests/**`, `*.test.*`, `**/__tests__/**`)
- Enforcement: `npm run -s ops:file-size:gate`

## Caps

| Category | Cap (LoC/file) |
| --- | ---: |
| client/runtime/systems/ui/**/*.(ts|tsx|js|jsx|css|html) | 500 |
| other ts/tsx/js/jsx | 1200 |
| core/**/*.rs | 500 |
| non-core .rs | 1000 |

## Exceptions

Exceptions require `path`, `owner`, `reason`, `expires` in `docs/workspace/repo_file_size_policy.json`.

| Path | Owner | Expires | Reason |
| --- | --- | --- | --- |
| _none_ | - | - | - |

Generated from policy snapshot: 2026-03-27T02:13:14.058Z
