# Validation release gates

This subdomain owns release-blocking controlled checks and promotion gates.

Canonical release-gate definitions now live here instead of under test harness paths:

- `config/release_gates.yaml` contains profile thresholds for runtime proof, boundedness, recovery, gateway chaos, and quality telemetry.
- `contracts/release_proof_pack_manifest.json` contains the required proof-pack artifact contract and category completeness/freshness budgets.
- `policies/release_blocker_rubric.json` contains release-blocker classification, status, ownership, and budget policy.
- `proof_packs/` contains generated release proof-pack snapshots and historical proof-pack evidence owned by Validation release gates.
- Temporary compatibility mirrors should be declared only if release-gate migration debt is reintroduced; there are no active release-gate mirror registries right now.

Harnesses may live under `tests/tooling/**`, but release-gate truth should be read from this subdomain.
