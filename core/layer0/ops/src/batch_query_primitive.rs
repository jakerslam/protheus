// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
// V6-TOOL-001: Batch Query Primitive (MVP)

include!("batch_query_primitive_parts/010-core.rs");
include!("batch_query_primitive_parts/020-pipeline.rs");
include!("batch_query_primitive_parts/030-run.rs");
#[cfg(test)]
include!("batch_query_primitive_parts/040-tests.rs");
