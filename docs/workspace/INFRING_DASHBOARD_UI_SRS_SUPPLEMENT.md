# InfRing Dashboard UI SRS Supplement

Updated: 2026-03-27 17:36 America/Denver
Owner: Dashboard Reliability + UX
Status: Active (supplemental until merged into `docs/workspace/SRS.md`)

## Purpose
This document captures detailed dashboard/chat UI requirements that are currently under-specified in canonical SRS lanes and have historically produced regressions.

## Scope
- In scope: dashboard UI behavior, rendering contracts, interaction contracts, and runtime sync UX.
- Out of scope: backend authority semantics (captured in `CLIENT_RUNTIME_SRS_SUPPLEMENT.md`).

## UI global invariants
- `UI-INV-001`: UI must be non-authoritative; all mutations are core-backed actions.
- `UI-INV-002`: UI must render deterministic placeholders during loading/reconnect phases.
- `UI-INV-003`: UI regressions must be guarded by lane tests for each major surface.

## A. Boot/startup UX
- `UI-BOOT-001`: InfRing splash must be first visible surface on startup.
- `UI-BOOT-002`: Theme handshake must avoid white flash / theme flicker during first paint.
- `UI-BOOT-003`: Dashboard boot failures must show actionable retry state (not blank screen).
- `UI-BOOT-004`: Startup must emit connection phase states: `booting`, `connecting`, `ready`.

## B. Navigation + information architecture
- `UI-NAV-001`: Agent sidebar hierarchy must keep `Conversations`, `Manage`, `Sessions`, `Approvals` at correct sibling levels.
- `UI-NAV-002`: Redundant overlays duplicating existing navigation must be removed.
- `UI-NAV-003`: Tabs and menus must keep stable ordering across reloads.
- `UI-NAV-004`: Selected tab background must be semi-transparent and theme-consistent.

## C. Chat column layout contracts
- `UI-CHAT-001`: Top and bottom fade shadows must span full chat column width.
- `UI-CHAT-002`: Top fade anchor must be fixed to top bar lower boundary.
- `UI-CHAT-003`: Bottom fade anchor must be fixed to input zone boundary (not to moving bubbles).
- `UI-CHAT-004`: Loading spinner for thread/chat fetch must be exactly vertically centered in chat column viewport.
- `UI-CHAT-005`: User bubbles must include transparent title label `Me` for export/copy disambiguation.

## D. Message rendering + isolation
- `UI-MSG-001`: Message streams are isolated by agent conversation id.
- `UI-MSG-002`: One agent's init/system messages must never render in another agent thread.
- `UI-MSG-003`: Tool call cards and terminal cards render as cards, not inline role text.
- `UI-MSG-004`: Thinking traces and internal reasoning must not render as assistant message body.

## E. Prompt queue rendering/interaction
- `UI-QUEUE-001`: Queued prompts are centered and max width is 90%.
- `UI-QUEUE-002`: Queue stack corner radii: top item top-corners only, other items square.
- `UI-QUEUE-003`: Queue 3-dot menu appears on right and supports `Edit message`.
- `UI-QUEUE-004`: Edit mode keeps queue item width stable and preserves order rules.
- `UI-QUEUE-005`: Queue items do not render/send before turn or explicit steer.

## F. Prompt suggestion UX
- `UI-SUG-001`: Suggestions render maximum 3 cards.
- `UI-SUG-002`: Suggestion card width is stable regardless of count (1-3).
- `UI-SUG-003`: Expand behavior is upward-only, width invariant.
- `UI-SUG-004`: Blur animation triggers only on actual height change.
- `UI-SUG-005`: Transition timing is fast enough to feel responsive and non-jarring.

## G. Input composer + fades
- `UI-INP-001`: Input-area fade must remain attached to input anchor, not transient chat elements.
- `UI-INP-002`: No duplicate/redundant shadow layers around composer.
- `UI-INP-003`: Prompt queue must not shift input fade anchor.

## H. Visual effects (grid + neon trail)
- `UI-FX-001`: Grid background must persist through full chat/document height (no cutoff).
- `UI-FX-002`: Grid anchoring follows page scroll space (world-space), not viewport lock.
- `UI-FX-003`: Grid mask around cursor remains visible when mouse is idle and pulses smoothly.
- `UI-FX-004`: Grid and trail effects apply consistently across dark-mode pages (where enabled).
- `UI-FX-005`: Neon trail particle size/spacing are consistent across pages.
- `UI-FX-006`: Trail and orb must track actual cursor coordinates precisely (no offset drift).

## I. LLM menu UI
- `UI-LLM-001`: Metadata row appears below model title.
- `UI-LLM-002`: Cost/power icon uses lightning iconography, not fire emoji.
- `UI-LLM-003`: Metadata includes context size, params, provider-locality indicator, specialty tags.
- `UI-LLM-004`: Download in progress shows spinner on button + progress bar with percentage.
- `UI-LLM-005`: On completion, UI emits standard in-chat notice event.

## J. Agent init/manage UI
- `UI-AG-001`: Create-agent button shows stable spinner ring while init view opens.
- `UI-AG-002`: Agent init flow must not exit prematurely before confirmation.
- `UI-AG-003`: Required sections visibly marked `required`.
- `UI-AG-004`: Lifespan section appears above optional personality/vibe sections.
- `UI-AG-005`: `Other` role card supports freeform purpose capture and card summary replacement.
- `UI-AG-006`: Init supports avatar upload + emoji picker; remove freeform emoji textbox.
- `UI-AG-007`: Agent countdown display uses days/hours/minutes only; immortals show infinity symbol.
- `UI-AG-008`: Pre-expiry fade/blink animation occurs before archival transition.

## K. Notifications + refresh controls
- `UI-NOTIF-001`: Bell icon uses subtle ring animation for unread attention.
- `UI-NOTIF-002`: Idle-agent alerts must have explicit rationale and suppress rules.
- `UI-RF-001`: Refresh icon hover rotation is deterministic and clockwise.
- `UI-RF-002`: Click refresh triggers actual refresh action and stable animation state.

## L. Channels/eyes/extensions pages
- `UI-CH-001`: Channel tab shows runtime-registered adapters, not hardcoded subset.
- `UI-EYES-001`: Eyes tab displays active eyes state + manual add/edit controls.
- `UI-EYES-002`: Channel-specific onboarding UX (including QR flows when relevant) must have clear state machine.

## M. Connectivity + resilience UX
- `UI-RES-001`: On disconnect, keep cached conversations visible with reconnect banner.
- `UI-RES-002`: Agent list must not collapse to zero during transient reconnect when runtime still has active agents.
- `UI-RES-003`: `connecting` indicator must reconcile with actual agent/session data state.
- `UI-RES-004`: Sidebar preview loading and chat loading spinners must recover after reconnect.

## N. Visual style constraints
- `UI-VIS-001`: Cards/tool boxes remain semi-transparent in default and hover states.
- `UI-VIS-002`: Notice colors use off-white in dark mode and gray in light mode.
- `UI-VIS-003`: Chat bubble opacity remains slightly transparent, theme-safe.

## O. Validation matrix (minimum)
- `UI-VAL-001`: Boot/reload no-blank-screen smoke test.
- `UI-VAL-002`: Agent init create flow endurance (100 sequential create/open/close loops).
- `UI-VAL-003`: Conversation isolation regression (cross-agent contamination test).
- `UI-VAL-004`: Fade-anchor snapshot tests at multiple scroll offsets.
- `UI-VAL-005`: Cursor FX coordinate/scroll integration test.
- `UI-VAL-006`: Reconnect resilience test with injected WS interruptions.

## Merge note
- These clauses are migration-era guardrails and must be converted into canonical `SRS.md` lane entries as slices are completed.
