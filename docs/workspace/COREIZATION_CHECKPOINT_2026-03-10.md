# Coreization Checkpoint (2026-03-10)

## Scope
- Hard Coreization Wave 1 static verification pass for:
  - `client/runtime/systems/security`
  - `client/runtime/systems/spine`
  - `client/runtime/systems/memory`
  - `client/runtime/systems/autonomy`
  - `client/runtime/systems/workflow`
  - `client/runtime/systems/ops/protheusd.ts`

## Static Audit Result
- Command:
  - `node scripts/ci/coreization_wave1_static_audit.mjs --out artifacts/coreization_wave1_static_audit_2026-03-10.json`
- Result:
  - `pass: true`
  - `violation_count: 0`
  - `warning_count: 0`
- Module summary:
  - `security`: wrappers `197`, non-wrappers `0`
  - `spine`: wrappers `14`, non-wrappers `0`
  - `memory`: wrappers `63`, non-wrappers `0`
  - `autonomy`: wrappers `122`, non-wrappers `0`
  - `workflow`: wrappers `35`, non-wrappers `0`
  - `ops-daemon`: wrappers `1`, non-wrappers `0`

## Cleanup Applied
- Added missing ownership marker in:
  - `client/runtime/systems/security/venom_containment_layer.ts`
- Added wrapper-policy marker token used by layer-placement guard:
  - `client/runtime/systems/security/venom_containment_layer.ts`

## Rust Share
- Command:
  - `npm run -s metrics:rust-share:gate`
- Result:
  - `rust_share_pct: 63.723`
  - `rs: 118795`, `ts: 39301`, `js: 28329`

## Runtime Regression Status
- Runtime execution remains blocked in this session:
  - Local compiled binaries (including minimal `/tmp` test binaries) hang before `main`.
- Host policy evidence:
  - `/tmp/hello_c_test` and `/tmp/hello_rust_test` hang with no stdout.
  - `spctl --assess --type execute /tmp/hello_c_test` -> rejected.
- Deferred fallback status:
  - `OPS-BLOCKER-002` completed (fail-closed defaults enabled).
  - Active bridge defaults now set `PROTHEUS_OPS_DEFER_ON_HOST_STALL=0`.
- Current command snapshot (fail-closed):
  - `./verify.sh` -> timed out (60s cap in blocker run)
  - `npm run -s typecheck:systems` -> `spawnSync .../target/debug/protheus-ops ETIMEDOUT`
  - `npm run -s ops:source-runtime:check` -> `spawnSync .../target/debug/protheus-ops ETIMEDOUT`
  - `npm run -s ops:subconscious-boundary:check` -> `spawnSync .../target/debug/protheus-ops ETIMEDOUT`
  - `npm run -s test:memory:context-budget` -> `spawnSync .../target/debug/protheus-ops ETIMEDOUT`
  - `npm run -s test:memory:matrix` -> `spawnSync .../target/debug/protheus-ops ETIMEDOUT`
  - `npm run -s test:memory:auto-recall` -> `spawnSync .../target/debug/protheus-ops ETIMEDOUT`
  - `npm run -s test:reflexes` -> `spawnSync .../target/debug/protheus-ops ETIMEDOUT`
  - `npm run -s ops:srs:top200:regression` -> pass (`fail:0 warn:0 pass:200`)
  - `npm run -s metrics:rust-share:gate` -> pass (`rust_share_pct: 63.723`)
  - `npm run -s ops:layer-placement:check` -> pass (`violations_count:0`)
- Full regression artifact:
  - `artifacts/blocker_regression_2026-03-10.json`
- Action when environment clears:
  - Re-run `./verify.sh`
  - Re-run system suite:
    - `npm run -s typecheck:systems`
    - `npm run -s test:ops:source-runtime-classifier`
    - `npm run -s test:ops:subconscious-boundary-guard`
    - `npm run -s test:memory:context-budget`
    - `npm run -s test:memory:matrix`
    - `npm run -s test:memory:auto-recall`
    - `npm run -s test:reflexes`
