# Project Registry
# Last updated: 2026-02-13

## Active Projects
| node_id | name | start_date | notes |
|---------|------|------------|-------|
| moltstack-v1 | The Protheus Codex | 2026-02-13 | MoltStack publication + auto-publish cron |
| x-engagement | X/Twitter Engagement Bot | 2026-02-13 | Automated browsing + reply system |

## Paused Projects
| node_id | name | paused_date | reason | resume_trigger |
|---------|------|-------------|--------|----------------|

## Retired Projects
| node_id | name | retired_date | reason | lessons_learned |
|---------|------|--------------|--------|-----------------|

## Classification Rules
1. **Registry precedence**: project-registry.md overrides tag-based classification
2. **Mismatch handling**: Emit warning, keep registry classification
3. **Default category**: Ops (never auto-promote to Project)
4. **Uncertain classification**: Ask user to add to registry
5. **Deterministic only**: No heuristics, no silent promotions

## Node States
- **Active**: Currently running, has active cron or ongoing work
- **Paused**: Temporarily stopped, has resume trigger defined
- **Retired**: Completed or abandoned, lessons documented
