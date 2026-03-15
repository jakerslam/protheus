# Internal Alias Map (V7-OBFUSCATION-001-SELECTIVE)

Status: internal-only mapping for crown-jewel symbol obfuscation.
Scope: binary blob vault, organism dream/homeostasis loops, RSI ignition core loop, tiny no_std + embedded-max internals.

## core/layer0/ops/src/binary_blob_runtime.rs

| Before | After |
| --- | --- |
| `prime_blob_vault_path` | `circulatory_vault_core_path` |
| `default_prime_blob_vault` | `default_circulatory_vault_core` |
| `load_prime_blob_vault` | `load_circulatory_vault_core` |
| `store_prime_blob_vault` | `store_circulatory_vault_core` |
| `blob_vault_secret` | `circulatory_signing_secret` |
| `blob_vault_signing_keys` | `circulatory_signing_keys` |
| `normalize_prime_blob_vault` | `normalize_circulatory_vault_core` |
| `validate_prime_blob_vault` | `validate_circulatory_vault_core` |
| `append_prime_blob_vault_entry` | `append_circulatory_vault_entry` |
| `repair_prime_blob_vault` | `repair_circulatory_vault_core` |

## core/layer0/ops/src/organism_layer_phase1.rs

| Before | After |
| --- | --- |
| `command_dream` body implementation | `substrate_dream_engine` |
| `command_homeostasis` body implementation | `substrate_homeostasis_loop` |

Notes:
- Public command handlers (`command_dream`, `command_homeostasis`) remain unchanged as thin wrappers.
- CLI and command surface are unchanged.

## core/layer0/ops/src/rsi_ignition.rs

| Before | After |
| --- | --- |
| `loop_state_path` | `recursive_core_state_path` |
| `recursive_loop_path` | `recursive_ignition_log_path` |
| `default_loop_state` | `default_recursive_core_state` |
| `load_loop_state` | `load_recursive_core_state` |
| `store_loop_state` | `store_recursive_core_state` |
| `loop_obj_mut` | `recursive_state_obj_mut` |
| `estimate_recent_failure_rate` | `estimate_recursive_failure_pressure` |
| `simulate_regression` | `simulate_recursive_regression` |

## core/layer0/tiny_runtime/src/lib.rs

| Before | After |
| --- | --- |
| `TINY_PROFILE` backing constant literal | `SUBSTRATE_TINY_CORE_PROFILE` |
| `tiny_profile` direct constant return path | `substrate_tiny_core_profile` helper |

Notes:
- Public `TINY_PROFILE` and `tiny_profile()` API remain intact.

## core/layer0/ops/src/protheusd.rs (tiny/embedded internals)

| Before | After |
| --- | --- |
| `tiny_status` | `substrate_no_std_status_payload` |
| `tiny_max_status` | `substrate_embedded_max_status_payload` |

Notes:
- Public daemon command names remain unchanged (`tiny-status`, `tiny-max-status`).
