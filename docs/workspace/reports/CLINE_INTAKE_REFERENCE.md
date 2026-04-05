# Cline Intake Reference (Isolated Import Lane)

## Source
- Repo: `https://github.com/cline/cline`
- Isolated clone: `local/workspace/vendor/cline`
- Checked revision: `e52a052c8` (short SHA)

## Master Ledger
- File-by-file status ledger: `docs/workspace/reports/CLINE_FILE_STATUS.tsv`
- Format: `status<TAB>path<TAB>notes`
- Scope excludes `.git/**` and `node_modules/**`.

## Status Vocabulary
- `pending`: not yet reviewed.
- `reviewed_no_import`: reviewed, no net-new capability worth porting.
- `reviewed_candidate`: reviewed, candidate capability identified.
- `imported`: capability ported into runtime.
- `skipped_non_runtime`: assets/docs/legal/non-runtime file skipped for capability ingestion.

## Checkoff Deletion Rule
- After a row is checked off (`reviewed_*` or `imported`), the source file is removed from the active intake tree and moved to:
  - `local/workspace/vendor/cline/.assimilation_deleted/<path>`
- Ledger notes include `deleted_to=.assimilation_deleted`.

## Current Imported Capability
- `CLINE-FILE-SEARCH-001` (from `src/services/search/file-search.ts`):
  - Added Rust-core `workspace-file-search` domain with:
    - ripgrep-backed file/folder discovery
    - fuzzy ranking (tight-match preference)
    - multi-root input support
    - workspace boundary gate (`workspace_outside_root` unless explicitly allowed)
- `CLINE-TERMINAL-TRUNCATION-001` (from `cli/src/acp/AcpTerminalManager.ts`):
  - Imported head+tail terminal output truncation semantics into core terminal broker:
    - explicit truncation marker (`... (output truncated) ...`)
    - preserves both early and recent output context (instead of tail-only clipping)
    - UTF-8 boundary-safe slicing and byte-budget enforcement
- `CLINE-RG-HINT-001` (from `cli/src/components/FileMentionMenu.tsx`):
  - Imported ripgrep-missing install-hint diagnostics into Rust `workspace-file-search`:
    - emits `rg_not_found` warning with platform install hint
    - exposes install hint in `workspace-file-search status`
    - covered by regression test `run_search_reports_ripgrep_install_hint_when_missing`

## Captured Candidates (Not Yet Imported)
- `CLINE-CANDIDATE-SESSION-GUARD-001` (from `cli/src/agent/ClineAgent.ts`):
  - Per-session "already processing" prompt gate to prevent overlapping request races.
- `CLINE-CANDIDATE-STREAM-DEDUPE-001` (from `cli/src/agent/ClineAgent.ts` + `messageTranslator.ts`):
  - Stable mapping between streaming message timestamps and tool-call IDs to avoid duplicate tool events.
- `CLINE-CANDIDATE-PERMISSION-AUTOALLOW-001` (from `cli/src/agent/permissionHandler.ts`):
  - Scoped auto-approval tracker for repeated command/tool/server approvals.
- `CLINE-CANDIDATE-ACTION-BAR-MAP-001` (from `cli/src/components/ActionButtons.tsx`):
  - Centralized ask-state to primary/secondary action-button map for deterministic recovery UX.
- `CLINE-CANDIDATE-ACCOUNT-CARD-001` (from `cli/src/components/AccountInfoView.tsx`):
  - Provider/account/credits summary panel pattern for quick operator readiness context.
- `CLINE-CANDIDATE-ASK-MODE-ROUTER-001` (from `cli/src/components/AskPrompt.tsx`):
  - Prompt-type classifier driving deterministic input modes (confirmation/text/options/completion).
- `CLINE-CANDIDATE-AUTH-ONBOARDING-001` (from `cli/src/components/AuthView.tsx`):
  - Staged provider onboarding flow with import-source detection and guided auth transitions.
- `CLINE-CANDIDATE-CHAT-RENDER-001` (from `cli/src/components/ChatMessage.tsx`):
  - Structured tool-call/result rendering with markdown-token rendering paths and parse fallbacks.
- `CLINE-CANDIDATE-STREAM-REGION-001` (from `cli/src/components/ChatView.tsx`):
  - Dynamic-region isolation pattern to keep live rendering stable under continuous streaming updates.
- `CLINE-CANDIDATE-BEDROCK-SETUP-001` (from `BedrockSetup.tsx` + `BedrockCustomModelFlow.tsx`):
  - Guided cloud-provider setup flow for custom model onboarding and provider-specific credential UX.
- `CLINE-CANDIDATE-CHECKPOINT-MENU-001` (from `cli/src/components/CheckpointMenu.tsx`):
  - Restore-mode chooser (`task`, `workspace`, `taskAndWorkspace`) for explicit recovery semantics.
- `CLINE-CANDIDATE-DIFF-COLLAPSE-001` (from `cli/src/components/DiffView.tsx`):
  - Collapsed-context diff rendering that preserves nearby change context and hides long unchanged runs.
- `CLINE-CANDIDATE-UI-ERROR-BOUNDARY-001` (from `cli/src/components/ErrorBoundary.tsx`):
  - User-facing crash containment with centralized exception capture and graceful shutdown path.
- `CLINE-CANDIDATE-FEATURED-MODELS-001` (from `cli/src/components/FeaturedModelPicker.tsx`):
  - Curated featured-model selection surface with labels/tags and browse-all handoff.
- `CLINE-CANDIDATE-CLI-IMPORT-WIZARD-001` (from `cli/src/components/ImportView.tsx`):
  - Source-detected key import wizard flow (selection, confirm, apply, error recovery).
- `CLINE-CANDIDATE-HIGHLIGHTED-COMPOSER-001` (from `cli/src/components/HighlightedInput.tsx`):
  - Cursor-stable segment parser for mention/slash highlighting in live input composer.
- `CLINE-CANDIDATE-FEATURE-TIP-001` (from `cli/src/components/FeatureTip.tsx`):
  - Delayed rotating feature-tip strip during long thinking/acting phases to improve discoverability without blocking flow.
- `CLINE-CANDIDATE-FOCUS-CHAIN-001` (from `cli/src/components/FocusChain.tsx`):
  - Checklist parser with current-step/progress visualization for deterministic task-progress transparency.
- `CLINE-CANDIDATE-HISTORY-PANEL-001` (from `cli/src/components/HistoryPanelContent.tsx`):
  - Inline searchable keyboard-driven history panel with centered selection window and scroll indicators.
- `CLINE-CANDIDATE-HISTORY-VIEW-001` (from `cli/src/components/HistoryView.tsx`):
  - Adaptive visible-window history rendering with pagination controls and bounded row usage.
