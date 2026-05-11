# Combined Rust Reference Map (2026-05-09)

This text-scan map classifies whether tracked combined artifacts appear to be referenced by live modules, validation/tests, docs/policy, other text, or no textual reference. It is intentionally conservative: unreferenced rows are candidates for case-by-case cleanup, not automatic deletion.

- Total artifacts mapped: 472
- JSON artifact: `validation/reports/combined_rust_reference_map_2026-05-09.json`

## Reference classes

| Class | Count |
| --- | ---: |
| live_rust_module_reference | 264 |
| unreferenced_by_text_scan | 150 |
| docs_or_policy_reference | 46 |
| validation_or_test_reference | 9 |
| other_text_reference | 3 |

## Cleanup rule

Do not delete any combined artifact solely because this scan marks it unreferenced. Use this map to create small rollback-safe cleanup commits by domain, with generated-artifact policy checked after each wave.
