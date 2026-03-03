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

## CLI Entry

Use the suite wrapper:

```bash
protheus toolkit list
```

Tool routes:

```bash
protheus toolkit personas --list
protheus toolkit dictionary term "Binary Blobs"
protheus toolkit orchestration status
protheus toolkit blob-morphing status
protheus toolkit comment-mapper --persona=vikram_menon --query="Should we prioritize memory or security first?" --gap=1 --active=1
```

## Examples

- `examples/personas-demo/`
- `examples/dictionary-demo/`
- `examples/orchestration-demo/`
- `examples/blob-morphing-demo/`
- `examples/comment-mapper-demo/`

## Internal Positioning

This is an internal operators toolkit. It is optimized for:

- deterministic evidence paths
- quick auditability
- behavior-preserving workflows
- sovereignty gate compatibility

