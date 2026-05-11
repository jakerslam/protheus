# Rust Dead-Code Warning Baseline (2026-05-09)

- complete: false
- complete_reason: compile_blocked_before_full_warning_surface
- command: `cargo check --manifest-path core/layer0/ops/Cargo.toml`
- exit_code: 101
- warning_count_seen_before_blocker: 23
- error_count: 2

## Compile blocker

   --> core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/045-tool-recovery-and-turn-persistence.rs:627:3

error: this file contains an unclosed delimiter

## Policy

This is a partial baseline. It is useful for trend guard wiring, but it must not be treated as the full Rust dead-code surface until the outside-lane dashboard compatibility compile blocker is fixed.
