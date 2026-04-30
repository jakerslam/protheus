# Chat Shell Purge Map

Updated: 2026-04-28

## Purpose

Recent `chat.ts` work was decomposition, not the purge itself.

That decomposition still matters because it turned one large unreadable Shell file into named seams we can classify honestly:

- keep in Shell because the logic is presentation-local
- shrink in Shell because the logic is still UI-adjacent but too stateful
- move out of Shell because the logic is deciding, shaping, or reconstructing more than a thin UI should

This document is the bridge between `SHELL-CLEANUP-007` and `SHELL-CLEANUP-014`.

## Boundary Rule

Use this split for the remaining chat surface:

- Shell keeps presentation, interaction, lightweight view state, and bounded local persistence
- Orchestration owns non-canonical coordination, decomposition, sequencing, routing recommendations, and workflow shaping
- Kernel owns authoritative truth, durable contracts, and canonical runtime decisions (`core/**` is the implementation path only)

If a `chat.ts` cluster is deciding what should happen next, rebuilding truth from raw payloads, or inferring workflow/tool behavior, it is not thin Shell and should be purged.

## Current Extracted Helper Classification

| Helper | Current role | Classification | Next action |
| --- | --- | --- | --- |
| `client/runtime/systems/ui/infring_static/js/pages/chat_scroll_helpers.ts` | scroll follow, bottom clamp, markdown export helpers | keep in Shell | keep local and eventually fold into a smaller chat-render/view module |
| `client/runtime/systems/ui/infring_static/js/pages/chat_message_display_helpers.ts` | message display windowing and search-visible row math | keep in Shell | keep local and reduce further only if Svelte extraction makes it obsolete |
| `client/runtime/systems/ui/infring_static/js/pages/chat_paste_helpers.ts` | large-paste detection and attachment conversion thresholds | keep in Shell | keep local unless paste policy becomes authoritative elsewhere |
| `client/runtime/systems/ui/infring_static/js/pages/chat_input_history_helpers.ts` | input history normalization, cursoring, per-agent local draft state | shrink in Shell | keep UI-local behavior but collapse into a smaller input-state module later |
| `client/runtime/systems/ui/infring_static/js/pages/chat_session_notice_helpers.ts` | dismissal keys and local session notice memory | shrink in Shell | keep only bounded local notice memory and avoid growing policy here |
| `client/runtime/systems/ui/infring_static/js/pages/chat_conversation_cache_helpers.ts` | draft/session cache restore and persistence helpers | move or heavily shrink | keep only bounded local draft cache; move any session reconstruction/shaping out of Shell |
| `client/runtime/systems/ui/infring_static/js/pages/chat_model_catalog_helpers.ts` | provider/model payload shaping and fallback option resolution | move or heavily shrink | Shell may format display rows, but model/fallback shaping should be emitted upstream as a contract |

## Remaining `chat.ts` Authority Clusters

These are the main non-thin seams still living in `client/runtime/systems/ui/infring_static/js/pages/chat.ts`.

### 1. Slash command execution

Representative symbols:

- `executeSlashCommand`
- `runSlashAlerts`
- `runSlashNextActions`
- `runSlashMemoryHygiene`
- `runSlashContinuity`
- `runSlashOptimizeWorkers`
- `runSlashApiKeyDiscovery`
- `runSlashMemprobe`

Why this is not thin Shell:

- It is choosing operations, not merely rendering controls
- It blends UX input handling with execution/routing behavior
- It accumulates command authority in a UI file

Target:

- move command routing and non-visual slash behavior into an explicit runtime/orchestration surface
- leave Shell with:
  - slash menu visibility
  - slash filter UX
  - command selection submission

Recommended destination:

- orchestration/runtime command adapter if behavior is coordination-only
- kernel/runtime command service if behavior is authoritative or mutating

### 2. Auto-route and context-window heuristics

Representative symbols:

- `applyAutoRouteTelemetry`
- `fetchAutoRoutePreflight`
- `inferContextWindowFromModelId`

Why this is not thin Shell:

- It decides routing and model/tool behavior
- It mixes runtime heuristics with a presentation surface
- It makes Shell responsible for workflow-adjacent judgment

Target:

- move route preflight and context-window inference behind a Rust-owned or orchestration-owned contract
- keep Shell only as the consumer of returned telemetry/projection

Recommended destination:

- `orchestration/**` for recommendation and preflight coordination
- `core/**` only if the decision becomes authoritative runtime truth

### 3. Session/message reconstruction and cache truth rebuilding

Representative symbols:

- `restoreAgentConversation`
- `loadConversationCache`
- `persistConversationCache`
- `normalizeSessionMessages`

Why this is not thin Shell:

- It is partially rebuilding a message/session model instead of only projecting it
- It risks Shell becoming the last-resort source of truth for history
- It blurs the line between local draft cache and semantic session reconstruction

Target:

- keep only bounded local draft/session convenience cache in Shell
- move normalization, reconstruction, and replay shaping into an upstream adapter or detail contract

Recommended destination:

- adapter/runtime projection layer for shaping server payloads into Shell-safe rows
- kernel/runtime contract emitter if canonical normalization is needed system-wide

### 4. Prompt suggestion fallback generation

Representative symbols:

- `derivePromptSuggestionFallback`
- `buildPromptSuggestionContextSnapshot`

Why this is not thin Shell:

- It generates assistant guidance/suggestion behavior in the UI surface
- It is logic-heavy and context-bearing rather than a simple view transform

Target:

- move suggestion fallback generation to orchestration or a dedicated backend/runtime suggestion contract
- keep Shell as a consumer of:
  - returned suggestion rows
  - display state
  - selection callbacks

Recommended destination:

- `orchestration/**`

### 5. Workspace panel payload shaping

Representative symbol:

- `workspacePanelPayload`

Why this may not be thin Shell:

- If it only formats current UI state for rendering, it can stay
- If it reconstructs or infers workspace truth, it should move

Target:

- audit this function after the earlier purge waves
- keep it only if it is a pure projection helper
- otherwise replace it with a bounded upstream detail payload

## Purge Order

Recommended order for the real Shell purge:

1. Slash command authority
2. Auto-route/context-window heuristics
3. Session/message normalization and cache reconstruction
4. Prompt suggestion fallback generation
5. Workspace panel payload audit and either retain-as-projection or move

This order gives the best burn reduction because it removes Shell-side decision logic before polishing leftover UI helpers.

## What Stays In Shell

Even after the purge, these categories should remain in Shell:

- scroll and viewport behavior
- local input focus/history UX
- bounded local draft persistence
- message list display/windowing
- paste/attachment affordance behavior
- menu open/close, picker filtering, selection UI
- projection-only formatting with no authority

## What Must Leave Shell

These categories should not keep living in `chat.ts` or nearby Shell runtime files:

- workflow/tool routing judgment
- model/context-window inference
- slash command execution authority
- prompt suggestion generation
- session/message truth reconstruction
- payload shaping that acts as hidden source-of-truth repair

## Execution Waves

### Wave A: purge map and contract prep

- classify extracted helpers
- identify remaining authority clusters
- define target destination per cluster

### Wave B: route/slash purge

- move slash command execution out of `chat.ts`
- move route preflight/context-window heuristics out of `chat.ts`
- keep only Shell submission and telemetry display

### Wave C: cache/session purge

- split local draft persistence from session/message reconstruction
- move normalization/replay shaping upstream
- reduce Shell cache helpers to bounded convenience state only

### Wave D: suggestion/workspace purge

- move prompt suggestion fallback generation out of Shell
- audit `workspacePanelPayload`
- convert any remaining non-projection shaping into upstream contracts

## Success Criteria

The purge is successful when:

- Shell chat code only renders, collects interaction, and manages bounded local UX state
- no route/tool/workflow heuristics remain in `chat.ts`
- no slash command execution authority remains in `chat.ts`
- no session/message truth reconstruction remains in `chat.ts`
- any remaining helper file can be described as presentation-local without hand-waving
