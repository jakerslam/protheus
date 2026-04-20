# Error Codebook + Support Playbook (V11-TODO-004)

Purpose: one deterministic catalog for installer, CLI, gateway, and doctor failures.

Version: `v1`
Last updated: `2026-04-19`

## Error Catalog

| Code | Surface | Meaning | Deterministic Remediation |
| --- | --- | --- | --- |
| `asset_not_listed_in_release` | installer | Required prebuilt artifact not present in release metadata. | Verify release asset publish step and asset naming; republish release assets or run source fallback with full toolchain. |
| `source_build_output_missing` | installer | Source fallback ran but required binary output not produced. | Check cargo target name and build profile output path; run release build locally and confirm expected binary stem. |
| `msvc_tools_missing_no_reachable_prebuilt_asset` | installer (Windows) | No reachable prebuilt and MSVC toolchain unavailable. | Install Visual Studio C++ build tools + rerun install command from README. |
| `symbiosis_coherence_kernel_unknown_command` | CLI lane | Invalid subcommand passed to symbiosis coherence kernel. | Use `symbiosis-coherence-kernel help` and retry with supported command. |
| `core_domain_contract_guard_failed` | CLI route guard | Command/route/script contract mismatch detected. | Run command list registry + contract guard; repair route map before retry. |
| `dashboard_authority_freshness_missing` | gateway/dashboard | Runtime block freshness contract incomplete. | Ensure payload includes `source_sequence`, `age_seconds`, `stale` and rerun dashboard authority contract tests. |
| `web_search_duplicate_attempt_suppressed` | web tooling | Replay guard suppressed duplicate search attempt. | Wait for cooldown or change query signature; use retry guidance from payload. |
| `web_fetch_duplicate_attempt_suppressed` | web tooling | Replay guard suppressed duplicate fetch attempt. | Wait for cooldown or adjust request URL/shape to new signature. |
| `non_search_meta_query` | web tooling | Query blocked by meta-query guard. | Reformulate as explicit web search intent or use explicit override if policy allows. |
| `query_required` | web tooling | Search request was empty/invalid. | Provide non-empty search query string and retry. |

## Triage Flow

1. Identify surface (`installer`, `cli`, `gateway`, `doctor`).
2. Capture deterministic error code from receipt/log output.
3. Apply matching remediation from catalog.
4. If unresolved, attach:
   - error code
   - command used
   - platform/arch
   - release/ref
   - latest troubleshooting snapshot

## Escalation Rules

- Escalate immediately when:
  - install fails on multiple matrix rows with same code
  - gateway start fails with authority-contract errors
  - doctor fails after remediation was applied
- For escalations, include this codebook code and the exact remediation attempt history.
