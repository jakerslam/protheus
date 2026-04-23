# Cognitive Toolkit Suite

The Cognitive Toolkit Suite bundles internal operator tools used for red-teaming, alignment checks, and deterministic governance reviews.

This suite is intentionally practical and sober: each tool maps to an executable command, a demo example, and test-backed behavior.

## Included Tools

1. Personas  
Purpose: Multi-lens red-teaming and alignment pressure testing.

2. Dictionary  
Purpose: Fast lookup for novel internal concepts and definitions.

3. Orchestration  
Purpose: Deterministic meeting/project control-plane with audited artifacts.

4. Blob Morphing  
Purpose: Validate binary blob assets used by fold/unfold logic.

5. Comment Mapper  
Purpose: Stream-of-thought mapping with optional intercept controls.

6. Assimilate  
Purpose: Ingest local/web sources into a deterministic sprint prompt with Kernel-5 review and safety gates.
Also available as a programmatic API (`client/runtime/systems/tools/assimilate_api.ts`) for loop/shadow self-use.

7. Research  
Purpose: Run natural-language research queries through hybrid evidence grading + Kernel-5 arbitration.
Also available as a programmatic API (`client/runtime/systems/tools/research_api.ts`) for loop/shadow self-use.
Includes proactive assimilation suggestions when tool/path/URL mentions are detected in query text.

8. Tutorial Suggestions
Purpose: Context-aware command nudges in the main CLI loop (external tool, drift, planning signals) with light Kernel-5 safety review.
Control via `infring tutorial status|on|off`.

## CLI Entry

Use the suite wrapper:

```bash
infring toolkit list
```

Tool routes:

```bash
infring toolkit personas --list
infring toolkit dictionary term "Binary Blobs"
infring toolkit orchestration status
infring toolkit blob-morphing status
infring toolkit comment-mapper --persona=vikram_menon --query="Should we prioritize memory or security first?" --gap=1 --active=1
infring toolkit assimilate ./docs/client/cognitive_toolkit.md --dry-run=1
infring toolkit research "creating a quant trading software" --dry-run=1
```

## Examples

- `apps/examples/personas-demo/`
- `apps/examples/dictionary-demo/`
- `apps/examples/orchestration-demo/`
- `apps/examples/blob-morphing-demo/`
- `apps/examples/comment-mapper-demo/`
- `apps/examples/assimilate-demo/`
- `apps/examples/research-demo/`

## Internal Positioning

This is an internal operators toolkit. It is optimized for:

- deterministic evidence paths
- quick auditability
- behavior-preserving workflows
- sovereignty gate compatibility
