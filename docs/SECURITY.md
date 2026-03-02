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

## Defense Hardening Expansion (`V3-RACE-DEF-025` ... `V3-RACE-DEF-031C`)

- `V3-RACE-DEF-025`: Smart Knot crown-jewel obfuscation contract lane (`systems/security/smart_knot_crown_jewel_obfuscation.ts`) with `build/knot/knot_pipeline_manifest.json` + `config/knot_config.json`.
- `V3-RACE-DEF-026`: Lockweaver structural flux lane (`systems/security/lockweaver/eternal_flux_field.ts`) with origin-lock verify/reseed checks.
- `V3-RACE-DEF-027`: Project Jigsaw replay theater lane (`systems/security/jigsaw/attackcinema_replay_theater.ts`) for capture/edit/playback governance.
- `V3-RACE-DEF-028`: Phoenix auto-respawn continuity lane (`systems/security/phoenix_protocol_respawn_continuity.ts`).
- `V3-RACE-DEF-029`: MirrorReaper Tier-4 resource inversion lane (`systems/security/mirrorreaper_tier4_resource_inversion.ts`).
- `V3-RACE-DEF-031A/B/C`: Thorn Swarm, Crimson Wraith, and Irrevocable Geas hardening lanes for sacrificial defense scaling + lineage-ban enforcement.

## Sovereignty & Governance Security (`V3-RACE-033`, `V3-RACE-035`, `V3-RACE-036`, `V3-RACE-039`)

- Mind Fortress covenant anchor lane (`systems/security/mind_fortress_principle.ts`) plus canonical manifesto in [`docs/MIND_SOVEREIGNTY.md`](./MIND_SOVEREIGNTY.md).
- Formal machine-checkable sovereignty verification lane (`systems/security/formal_mind_sovereignty_verification.ts`).
- Multi-mind isolation boundary lane (`systems/security/multi_mind_isolation_boundary_plane.ts`).
- Merge-interface protection substrate (`systems/continuity/human_machine_merge_interface_security_substrate.ts`).

## Enterprise Security Readiness (`V3-RACE-042` ... `V3-RACE-056`)

- Formal threat-model generation lane (`systems/security/formal_threat_modeling_engine.ts`).
- Reproducible build + supply-chain attestation lane (`systems/security/supply_chain_reproducible_build_plane.ts`).
- Independent safety coprocessor veto lane (`systems/security/independent_safety_coprocessor_veto_plane.ts`).
- Hardware root-of-trust attestation mesh lane (`systems/security/hardware_root_of_trust_attestation_mesh.ts`).
- Adversarial goal-drift auditor lane (`systems/security/adversarial_goal_drift_auditor.ts`).
- Insider split-trust governance lane (`systems/security/insider_threat_split_trust_command_governance.ts`).
- Signed plugin trust marketplace lane (`systems/security/signed_plugin_trust_marketplace.ts`).

## Pinnacle Integration Security Contract (`V3-RACE-144`)

- Contract check: `systems/ops/pinnacle_integration_contract_check.ts`
- Scope boundaries are enforced jointly with:
  - `docs/PINNACLE_TECH.md`
  - `docs/DATA_SCOPE_BOUNDARIES.md`
- Integration lanes (`V3-RACE-137`..`143`) must keep user data in `memory/` + `adaptive/` and permanent runtime logic in `systems/` + `config/`.
