// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Rust-native operator tooling kernel.
// Split into semantic parts to keep each file maintainable and under cap.

include!("operator_tooling_kernel_parts/010_prelude.rs");
include!("operator_tooling_kernel_parts/020_routing_validation_state.rs");
include!("operator_tooling_kernel_parts/030_memory_trace.rs");
include!("operator_tooling_kernel_parts/040_spawn.rs");
include!("operator_tooling_kernel_parts/050_ops.rs");
include!("operator_tooling_kernel_parts/060_entry_tests.rs");
