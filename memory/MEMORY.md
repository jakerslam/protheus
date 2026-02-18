# MEMORY.md – Root Navigator

## Traversal Protocol
1. **Start with TAGS_INDEX.md** or MEMORY_INDEX.md
2. **Search by tag/keyword** → get node_id
3. **Read only matching node** from daily file
4. **Read max 1-2 nodes** unless task explicitly requires more
5. **Never load full daily files** unless rebuilding index

## Quick Lookup
| File | Purpose | When |
|------|---------|------|
| TAGS_INDEX.md | Inverted tag → node_id | Tag-based recall |
| MEMORY_INDEX.md | Node metadata + edges | Keyword search |
| [YYYY-MM-DD].md | Node content | After index lookup |

## High-Priority Nodes
- moltstack-v1
- feedback-binary
- multi-agent-pivot

## Learnings & Insights
- **LEARNINGS_INDEX.md** — Curated insights from Moltbook/X community
  - Check before tasks: relevant patterns from other agents
  - Proactive suggestions: improvements ready to present
  - Tags: [architecture], [security], [optimization], [workflows]

## Regen Schedule
**When:** Sundays 6PM MST or after 5+ new nodes
**What:** Rebuild MEMORY_INDEX.md + TAGS_INDEX.md

## Taxonomy Governance Rules

**Registry Classification:**
- Category assignment is deterministic based on content analysis
- Human override requires explicit rule documentation below

**Rule: No Silent Category Moves**
- Registry auto-classification is advisory only
- Category changes require:
  1. Explicit human approval, OR
  2. Documented deterministic classifier rule in this section
- Auto-moves must be reviewed and confirmed

**Current Taxonomy:**
| Category | Criteria | Override Rule |
|----------|----------|---------------|
| Projects | Active builds with deliverables | Human assigns |
| Concepts | Specs, definitions, frameworks | Human assigns |
| Ops/Logs | Execution logs, cron outputs | Auto-classify OK |
| Rules | Policies, governance | Human assigns |
| System | Infrastructure, tooling | Human assigns |

**moltstack-v1 Classification:**
- Content: Spec/definition for publishing workflow
- Human-assigned: **Concepts** (not auto-movable)
- Rationale: MoltStack is a defined framework, not execution logs
