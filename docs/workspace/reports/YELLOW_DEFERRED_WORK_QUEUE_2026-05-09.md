# Yellow Deferred Work Queue (2026-05-09)

This queue keeps important architecture/tooling and next-SRS work visible without allowing it to compete with the current red closure work.

## Rule

Do not begin these streams until the red section is materially clear or an operator explicitly overrides the queue.

## Deferred streams

### Architecture and tooling deltas

- Source TODO: `ARCH-TOOLING-NEXT`
- Status: deferred behind red closure work
- Reason: architecture/tooling work is valuable, but it can become entropy if it lands while Shell, Sentinel freshness, and compile blockers are unresolved.
- Release condition: red section reduced below blocking threshold or explicit operator override.

### Next SRS stream

- Source TODO: `SRS-NEXT`
- Status: deferred behind red closure work
- Reason: more SRS streams would fragment focus while the active board still contains high-priority red blockers.
- Release condition: red section reduced below blocking threshold or explicit operator override.

## Active priority

Current active priority remains:

1. Reliability blockers that stop validation or Sentinel freshness.
2. Shell authority extraction only in a dedicated Shell-safe thread.
3. Compile blocker clearance before destructive hygiene.
4. Workflow utility after Shell de-authority is proven.
