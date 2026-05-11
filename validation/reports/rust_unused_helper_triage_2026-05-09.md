# Rust Unused Helper Triage (2026-05-09)

Source: `validation/reports/rust_deadcode_warning_baseline_2026-05-09.json`.

This triage is intentionally limited because `cargo check --manifest-path core/layer0/ops/Cargo.toml` currently stops on an outside-lane dashboard compatibility syntax conflict before a complete warning surface is available.

## Observed helper families before blocker

### Memory retention helpers

- `RETENTION_WARN_BELOW`
- `RETENTION_BLOCK_BELOW`
- `evaluate_retention_guard`
- `retention_health_label`
- Crate: `infring-memory-core-v6`
- Recommendation: reconnect to a memory health/reporting path or delete in a memory-owned batch after full compile validation is available.

### Layer 1 security normalization helpers

- `MAX_SECURITY_PLANES`
- `strip_invisible_unicode`
- `normalize_security_plane_name`
- `normalize_security_plane_list`
- `normalize_security_plane_list_with_contract`
- `security_planes_fail_closed`
- `MAX_SECURITY_SUBJECTS`
- `normalize_security_subject`
- `normalize_subject_list`
- `normalize_subject_list_with_contract`
- `fail_closed_subject_gate`
- Crate: `infring-layer1-security`
- Recommendation: likely valuable safety helpers. Prefer reconnecting to security policy validation or making intentional public contract exports before deletion.

### Execution autoscale parsing helpers

- `extract_mode_literals`
- `extract_bridge_modes`
- `extract_dispatch_modes`
- `read_optional_autonomy_surface`
- Crate: `execution_core`
- Recommendation: keep as review candidates until autoscale mode-surface validation is understood; deletion should wait for a complete execution-core check.

## Not observed in partial baseline

No unused-import warnings were observed before the compile blocker. `HYGIENE-RUST-UNUSED-IMPORTS` remains open until cargo can produce a complete warning surface.

## Compile blocker

```text
core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/045-tool-recovery-and-turn-persistence.rs
error: this file contains an unclosed delimiter
```
