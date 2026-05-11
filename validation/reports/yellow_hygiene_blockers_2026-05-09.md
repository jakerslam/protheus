# Yellow Hygiene Blockers (2026-05-09)

The following yellow cleanup items should not be mutated further until the outside-lane dashboard compatibility compile blocker is fixed:

- `HYGIENE-RUST-UNUSED-IMPORTS`: no unused-import warnings were observed before compile stopped; needs complete warning surface.
- `HYGIENE-COMBINED-DEAD-DELETE`: deletion candidates exist, but deletion requires targeted compile/check validation.
- `HYGIENE-COMBINED-DECOMPOSE-LIVE`: live split debt exists, but decomposition requires compile/check validation and owner-scoped batches.

Current blocker:

```text
core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/045-tool-recovery-and-turn-persistence.rs
error: this file contains an unclosed delimiter
```

Policy: do not delete or decompose combined Rust artifacts while the owning crate cannot be checked.
