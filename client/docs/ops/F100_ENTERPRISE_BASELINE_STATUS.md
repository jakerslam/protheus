# F100 Enterprise Baseline Status

Generated: 2026-03-07T18:19:29.437Z

| Check | Type | Path | Status | Reason |
|---|---|---|---|---|
| `license_apache2` | `file_contains` | `LICENSE` | PASS | `ok` |
| `security_posture_doc` | `file_exists` | `client/docs/SECURITY_POSTURE.md` | PASS | `ok` |
| `dependabot_enabled` | `file_exists` | `.github/dependabot.yml` | PASS | `ok` |
| `codeql_enabled` | `file_exists` | `.github/workflows/codeql.yml` | PASS | `ok` |
| `sbom_release_workflow` | `file_exists` | `.github/workflows/release-security-artifacts.yml` | PASS | `ok` |
| `slsa_attestation_release_workflow` | `file_contains` | `.github/workflows/release-security-artifacts.yml` | PASS | `ok` |
| `coverage_workflow` | `file_exists` | `.github/workflows/coverage.yml` | PASS | `ok` |
| `helm_packaging_present` | `file_exists` | `client/deploy/helm/protheus/Chart.yaml` | PASS | `ok` |
| `terraform_packaging_present` | `file_exists` | `client/deploy/terraform/protheus_helm/main.tf` | PASS | `ok` |
| `k8s_secret_runtime_manifest_present` | `file_exists` | `client/deploy/k8s/secret.runtime.example.yaml` | PASS | `ok` |
| `helm_secret_wiring_enabled` | `file_contains` | `client/deploy/helm/protheus/templates/cronjob.yaml` | PASS | `ok` |
| `enterprise_support_template_present` | `file_exists` | `client/docs/ENTERPRISE_SUPPORT_ENVELOPE_TEMPLATE.md` | PASS | `ok` |
| `case_study_template_present` | `file_exists` | `client/docs/REFERENCE_CUSTOMER_CASE_STUDY_TEMPLATE.md` | PASS | `ok` |
| `legal_packet_checklist_present` | `file_exists` | `client/docs/LEGAL_ENTERPRISE_PACKET_CHECKLIST.md` | PASS | `ok` |
| `a_plus_gate_script_present` | `file_exists` | `client/systems/ops/f100_a_plus_readiness_gate.js` | PASS | `ok` |
| `human_split_compliance_certs` | `file_contains` | `client/docs/HUMAN_ONLY_ACTIONS.md` | PASS | `ok` |

## Summary

- Total checks: 16
- Passed checks: 16
- Failed checks: 0
- Contract status: PASS
- Receipt hash: `a635bc4796736a53a0b88340eecb30ef10f63a2b04a6096d2068a1acca578061`
