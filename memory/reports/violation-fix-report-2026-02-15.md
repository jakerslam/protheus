# Weekly System Report — Feb 15, 2026

## Violation Fixes Summary

### ✅ 1. Frontmatter Violation Fixed (2026-02-14.md)
**Issue:** Multiple `<!-- SNIP -->` blocks without YAML frontmatter  
**Fix:** Converted all SNIP blocks to proper nodes with frontmatter

**Changes:**
- `rebuild-validate-1771127215487` → Added frontmatter
- `rebuild-validate-1771128020203` → Added frontmatter  
- `rebuild-validate-1771128401716` → Added frontmatter
- `habit-state-run_memory_rebuild_and_validate-1771128418560` → Added frontmatter
- `habit-state-test_task-*` (5 duplicates) → Moved to archive file
- `habit-state-heavy_task-1771130144553` → Moved to archive file
- Added `habit-state-archive-ref` node to link to archive

### ✅ 2. Bloat Violation Fixed (weekly-compound-2026-07)
**Issue:** 765 tokens (515 over 250 limit)  
**Fix:** Split into 4 child nodes

**New Child Nodes:**
| Node | File | Tokens |
|------|------|--------|
| weekly-07-atoms | weekly-07-atoms.md | ~73 |
| weekly-07-moltstack | weekly-07-moltstack.md | ~120 |
| weekly-07-experiment | weekly-07-experiment.md | ~90 |
| weekly-07-backlog | weekly-07-backlog.md | ~85 |

**Updated Parent:** weekly-compound-2026-07 → 56 tokens (index node with `edges_to`)

### ✅ 3. Taxonomy Governance Rule Added
**Location:** MEMORY.md  
**Rule:** Registry classification changes require:
1. Explicit human approval, OR
2. Documented deterministic classifier rule

**moltstack-v1 Classification:**
- Content: Spec/definition for publishing workflow
- Assigned: **Concepts** (human-locked)
- Rationale: Framework definition, not execution logs

### ✅ 4. Regeneration Complete
```
Files scanned: 97
Nodes found: 59 (was 45)
Tags found: 79 (was 63)
Skipped: 0 (was 1)
```

---

## 🔐 Security Action Required

**CRITICAL:** Moltbook API key was exposed in tool outputs.

**Status:** 
- ✅ Redaction guard implemented in spawn-safe, spawn-run, and redact-secrets utility
- ⚠️ **Key rotation required** — User must revoke/regenerate at moltbook.com

**Next Steps:**
1. Log into moltbook.com
2. Revoke existing API key (moltbook_sk_6g...)
3. Generate new key
4. Update `~/.config/moltbook/credentials.json`

---

**All violations resolved. System clean.**
