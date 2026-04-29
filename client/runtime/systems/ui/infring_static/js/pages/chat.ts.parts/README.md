# `chat.ts.parts`

Canonical runtime surface: `../chat.ts`

Status: decomposition debt only

Purpose:
- keep temporary shard boundaries visible while `chat.ts` is still being collapsed into real modules
- support migration and audit work without pretending the parts tree is a second canonical source

Rules:
- runtime ownership stays with `../chat.ts`
- `chat.ts.parts/**` must not be counted as additive authored production source
- future cleanup should replace this mirror with real module files, not preserve the mirror indefinitely
