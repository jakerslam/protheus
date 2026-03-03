# Org Code Format Standard

Applies to all first-party source and documentation assets.

## Baseline Rules

- Line endings: LF
- Charset: UTF-8
- Trailing whitespace: prohibited
- Tabs: prohibited in `.ts`, `.js`, `.rs`, `.md`, `.sh`
- Files end with a newline
- Keep commits scoped; one logical change per commit

## TypeScript / JavaScript

- Use `'use strict';` and explicit shebang in executable scripts.
- Prefer `const` by default; use `let` only when reassignment is required.
- Use deterministic JSON output (`JSON.stringify(..., null, 2)`).
- Keep CLI usage docs in each executable lane file.

## Rust

- `rustfmt`-compliant formatting.
- Public APIs use explicit types and predictable error surfaces.
- Keep crate-level docs concise and operational.

## Markdown

- Title first, then short purpose section.
- Keep sections stable to reduce diff noise.
- Use fenced code blocks with language hints where practical.

## Shell

- Start executable scripts with `#!/usr/bin/env bash` or `zsh` as needed.
- Prefer quoted vars and fail-fast mode when script semantics allow.
- Keep commands idempotent when used in CI/hooks.

## Guardrails

- CI gate: `npm run ops:format:check` and `npm run lint`
- Local pre-commit gate: `.githooks/pre-commit`
- Hook activation: `git config core.hooksPath .githooks`
- Verification engine: `systems/ops/org_code_format_guard.ts`
