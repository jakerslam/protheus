# Security Lanes

## PsycheForge (`V3-RACE-DEF-024`)

`systems/security/psycheforge/` provides adaptive attacker profiling and governed countermeasure selection.

Key properties:

- Behavioral classification (`impatient`, `methodical`, `aggressive`, `cautious`, `overconfident`, `script_kiddie`, `nation_state`)
- Encrypted temporal profile persistence
- Rust memory hot-state mirror (`set-hot-state`) for replay-safe profile continuity
- Tier 3+ actions require second-gate promotion (`shadow` -> `live`)
- Integration hints emitted for guard/redteam/venom/fractal loops

Commands:

```bash
node systems/security/psycheforge/psycheforge_organ.js evaluate --actor=probe --telemetry_json='{"probe_density":0.9,"escalation_attempts":12}' --apply=1
node systems/security/psycheforge/psycheforge_organ.js promote --decision_id=<id> --two_gate_approved=1 --apply=1
node systems/security/psycheforge/psycheforge_organ.js status
```
