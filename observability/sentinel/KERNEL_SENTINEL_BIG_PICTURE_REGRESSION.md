# Kernel Sentinel Big-Picture Regression Mode

Kernel Sentinel should not keep filing local symptom tickets when the evidence shows a
structural failure. This mode gives Sentinel a compact, deterministic way to decide
whether a finding stream should stay in local ticketing or pause into architecture-level
diagnosis.

## Purpose

The model is designed for failures like:

- Many symptoms appearing together across layers.
- Repeated local fixes that do not close the failure.
- Runtime behavior contradicting command/status output.
- Authority behavior reappearing after syntax or naming was removed.
- Policy text changing while the old behavior remains alive.

These are not ordinary bugs. They are signs that Sentinel should stop producing a pile
of small tickets and instead emit a structural diagnosis.

## Modes

- `local_ticketing`: isolated symptoms can become normal TODOs or issues.
- `structural_diagnosis`: local ticketing pauses while symptoms are clustered into one
  architecture-level report.
- `rebuild_realignment`: patching should stop until the authority model, runtime path,
  or ownership boundary is realigned.

## Required Inputs

- `symptom_ids`: stable names for the co-occurring symptoms.
- `affected_layers`: layers/domains touched by the symptoms.
- `repeated_local_fixes`: count of recent local repair attempts that failed to close the
  issue family.
- `command_runtime_contradiction`: true when CLI/status output disagrees with observed
  runtime behavior.
- `authority_shape_ghost`: true when removed authority behavior reappears in a new form.
- `policy_syntax_removed_but_behavior_remains`: true when policy text changed but runtime
  behavior kept the old shape.

## Operator Rule

When the assessment returns `stop_local_ticketing: true`, Sentinel output should not be
promoted directly into many small implementation tasks. It should first produce a
structural diagnosis with:

- the shared root-cause hypothesis,
- affected ownership boundaries,
- evidence references,
- why local patching is expected to fail,
- the recommended realignment or rebuild path.

This keeps Sentinel aligned with the system-understanding model: understand the whole
animal first, then zoom into the organs.
