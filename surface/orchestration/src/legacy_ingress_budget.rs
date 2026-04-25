// Layer ownership: surface/orchestration (non-authoritative ingress-quality release budgeting only).
use crate::contracts::RequestClassification;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const LEGACY_INGRESS_BUDGET_ARTIFACT_JSON: &str =
    "core/local/artifacts/legacy_ingress_budget_current.json";
pub const LEGACY_INGRESS_BUDGET_ARTIFACT_MARKDOWN: &str =
    "local/workspace/reports/LEGACY_INGRESS_BUDGET_CURRENT.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegacyIngressSurface {
    Sdk,
    Gateway,
    Dashboard,
    TypedCli,
    ExplicitLegacy,
    UnknownNonLegacy,
}

impl LegacyIngressSurface {
    fn as_str(self) -> &'static str {
        match self {
            Self::Sdk => "sdk",
            Self::Gateway => "gateway",
            Self::Dashboard => "dashboard",
            Self::TypedCli => "typed_cli",
            Self::ExplicitLegacy => "explicit_legacy",
            Self::UnknownNonLegacy => "unknown_non_legacy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyIngressBudgetPolicy {
    pub surface: LegacyIngressSurface,
    pub max_legacy_shim_rate: f32,
    pub max_surface_adapter_fallback_rate: f32,
    pub next_release_legacy_shim_rate_target: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyIngressBudgetRow {
    pub surface: LegacyIngressSurface,
    pub sample_count: u32,
    pub legacy_shim_count: u32,
    pub surface_adapter_fallback_count: u32,
    pub typed_probe_contract_violation_count: u32,
    pub low_confidence_count: u32,
    pub ambiguity_reason_count: u32,
    pub legacy_shim_rate: f32,
    pub surface_adapter_fallback_rate: f32,
    pub max_legacy_shim_rate: f32,
    pub max_surface_adapter_fallback_rate: f32,
    pub next_release_legacy_shim_rate_target: f32,
    pub over_budget: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LegacyIngressBudgetReport {
    #[serde(rename = "type")]
    pub report_type: String,
    pub policy_version: u32,
    pub ok: bool,
    pub release_over_release_tightening_delta: f32,
    pub rows: Vec<LegacyIngressBudgetRow>,
    pub failures: Vec<String>,
}

#[derive(Debug, Default)]
struct LegacyIngressAccumulator {
    sample_count: u32,
    legacy_shim_count: u32,
    surface_adapter_fallback_count: u32,
    typed_probe_contract_violation_count: u32,
    low_confidence_count: u32,
    ambiguity_reason_count: u32,
}

pub fn default_legacy_ingress_budget_policies() -> Vec<LegacyIngressBudgetPolicy> {
    vec![
        strict_non_legacy_policy(LegacyIngressSurface::Sdk),
        strict_non_legacy_policy(LegacyIngressSurface::Gateway),
        strict_non_legacy_policy(LegacyIngressSurface::Dashboard),
        strict_non_legacy_policy(LegacyIngressSurface::TypedCli),
        LegacyIngressBudgetPolicy {
            surface: LegacyIngressSurface::ExplicitLegacy,
            max_legacy_shim_rate: 1.0,
            max_surface_adapter_fallback_rate: 0.0,
            next_release_legacy_shim_rate_target: 0.75,
        },
        strict_non_legacy_policy(LegacyIngressSurface::UnknownNonLegacy),
    ]
}

fn strict_non_legacy_policy(surface: LegacyIngressSurface) -> LegacyIngressBudgetPolicy {
    LegacyIngressBudgetPolicy {
        surface,
        max_legacy_shim_rate: 0.0,
        max_surface_adapter_fallback_rate: 0.0,
        next_release_legacy_shim_rate_target: 0.0,
    }
}

pub fn build_legacy_ingress_budget_report(
    classifications: &[RequestClassification],
) -> LegacyIngressBudgetReport {
    let policies = default_legacy_ingress_budget_policies();
    let mut accumulators = policies
        .iter()
        .map(|policy| (policy.surface, LegacyIngressAccumulator::default()))
        .collect::<BTreeMap<_, _>>();

    for classification in classifications {
        let surface = surface_for_classification(classification);
        let accumulator = accumulators.entry(surface).or_default();
        accumulator.sample_count += 1;
        let legacy_shim = has_reason(classification, "legacy_intent_compatibility_shim");
        if legacy_shim {
            accumulator.legacy_shim_count += 1;
        }
        if classification.surface_adapter_fallback {
            accumulator.surface_adapter_fallback_count += 1;
        }
        if classification
            .reasons
            .iter()
            .any(|reason| reason.starts_with("typed_probe_contract_missing"))
        {
            accumulator.typed_probe_contract_violation_count += 1;
        }
        if classification.confidence < 0.55
            || has_reason(classification, "parse_confidence_below_threshold")
        {
            accumulator.low_confidence_count += 1;
        }
        accumulator.ambiguity_reason_count += classification
            .reasons
            .iter()
            .filter(|reason| is_trended_ambiguity_reason(reason))
            .count() as u32;
    }

    let mut failures = Vec::new();
    let rows = policies
        .into_iter()
        .map(|policy| {
            let acc = accumulators.remove(&policy.surface).unwrap_or_default();
            let legacy_shim_rate = rate(acc.legacy_shim_count, acc.sample_count);
            let fallback_rate = rate(acc.surface_adapter_fallback_count, acc.sample_count);
            let over_budget = acc.sample_count > 0
                && (legacy_shim_rate > policy.max_legacy_shim_rate
                    || fallback_rate > policy.max_surface_adapter_fallback_rate);
            if over_budget {
                failures.push(format!(
                    "legacy_ingress_budget_exceeded:{}:shim_rate={legacy_shim_rate:.3}:fallback_rate={fallback_rate:.3}",
                    policy.surface.as_str()
                ));
            }
            LegacyIngressBudgetRow {
                surface: policy.surface,
                sample_count: acc.sample_count,
                legacy_shim_count: acc.legacy_shim_count,
                surface_adapter_fallback_count: acc.surface_adapter_fallback_count,
                typed_probe_contract_violation_count: acc.typed_probe_contract_violation_count,
                low_confidence_count: acc.low_confidence_count,
                ambiguity_reason_count: acc.ambiguity_reason_count,
                legacy_shim_rate,
                surface_adapter_fallback_rate: fallback_rate,
                max_legacy_shim_rate: policy.max_legacy_shim_rate,
                max_surface_adapter_fallback_rate: policy.max_surface_adapter_fallback_rate,
                next_release_legacy_shim_rate_target: policy.next_release_legacy_shim_rate_target,
                over_budget,
            }
        })
        .collect::<Vec<_>>();

    LegacyIngressBudgetReport {
        report_type: "legacy_ingress_budget".to_string(),
        policy_version: 1,
        ok: failures.is_empty(),
        release_over_release_tightening_delta: 0.10,
        rows,
        failures,
    }
}

pub fn sample_release_classifications() -> Vec<RequestClassification> {
    vec![
        classification_fixture(LegacyIngressSurface::ExplicitLegacy, true, false, 0.80, &[]),
        classification_fixture(LegacyIngressSurface::Sdk, false, false, 0.90, &[]),
        classification_fixture(LegacyIngressSurface::Gateway, false, false, 0.90, &[]),
        classification_fixture(LegacyIngressSurface::Dashboard, false, false, 0.90, &[]),
        classification_fixture(LegacyIngressSurface::TypedCli, false, false, 0.90, &[]),
    ]
}

pub fn write_legacy_ingress_budget_artifacts(
    root: &Path,
    json_path: &str,
    markdown_path: &str,
) -> Result<LegacyIngressBudgetReport, String> {
    let report = build_legacy_ingress_budget_report(sample_release_classifications().as_slice());
    let json_out = root.join(json_path);
    let markdown_out = root.join(markdown_path);
    if let Some(parent) = json_out.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create json dir failed:{err}"))?;
    }
    if let Some(parent) = markdown_out.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create markdown dir failed:{err}"))?;
    }
    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("serialize legacy ingress budget failed:{err}"))?;
    fs::write(json_out, json).map_err(|err| format!("write json failed:{err}"))?;
    fs::write(markdown_out, markdown_for_report(&report))
        .map_err(|err| format!("write markdown failed:{err}"))?;
    Ok(report)
}

fn markdown_for_report(report: &LegacyIngressBudgetReport) -> String {
    let mut out = String::new();
    out.push_str("# Legacy Ingress Budget\n\n");
    out.push_str(format!("- ok: {}\n", report.ok).as_str());
    out.push_str(format!("- policy_version: {}\n", report.policy_version).as_str());
    out.push_str(
        format!(
            "- release_over_release_tightening_delta: {:.2}\n\n",
            report.release_over_release_tightening_delta
        )
        .as_str(),
    );
    out.push_str("| surface | samples | shim_rate | fallback_rate | next_target | over_budget |\n");
    out.push_str("| --- | ---: | ---: | ---: | ---: | --- |\n");
    for row in &report.rows {
        out.push_str(
            format!(
                "| {} | {} | {:.3} | {:.3} | {:.3} | {} |\n",
                row.surface.as_str(),
                row.sample_count,
                row.legacy_shim_rate,
                row.surface_adapter_fallback_rate,
                row.next_release_legacy_shim_rate_target,
                row.over_budget
            )
            .as_str(),
        );
    }
    if !report.failures.is_empty() {
        out.push_str("\n## Failures\n");
        for failure in &report.failures {
            out.push_str(format!("- `{failure}`\n").as_str());
        }
    }
    out
}

fn classification_fixture(
    surface: LegacyIngressSurface,
    legacy_shim: bool,
    fallback: bool,
    confidence: f32,
    extra_reasons: &[&str],
) -> RequestClassification {
    use crate::contracts::{Capability, ClarificationReason, RequestClass};
    let mut reasons = Vec::new();
    if legacy_shim {
        reasons.push("legacy_intent_compatibility_shim".to_string());
    }
    if !matches!(
        surface,
        LegacyIngressSurface::ExplicitLegacy | LegacyIngressSurface::UnknownNonLegacy
    ) {
        let prefix = if fallback {
            "surface_adapter_fallback"
        } else {
            "surface_adapter"
        };
        reasons.push(format!(
            "{prefix}:{}",
            surface.as_str().replace("typed_cli", "cli")
        ));
    }
    reasons.extend(extra_reasons.iter().map(|row| row.to_string()));
    RequestClassification {
        request_class: RequestClass::ReadOnly,
        confidence,
        reasons,
        required_capabilities: vec![Capability::ReadMemory],
        clarification_reasons: Vec::<ClarificationReason>::new(),
        needs_clarification: false,
        surface_adapter_used: !fallback && !legacy_shim,
        surface_adapter_fallback: fallback,
    }
}

fn surface_for_classification(classification: &RequestClassification) -> LegacyIngressSurface {
    classification
        .reasons
        .iter()
        .find_map(|reason| {
            reason
                .strip_prefix("surface_adapter_fallback:")
                .or_else(|| reason.strip_prefix("surface_adapter:"))
                .and_then(surface_from_token)
        })
        .unwrap_or_else(|| {
            if classification.surface_adapter_fallback {
                LegacyIngressSurface::UnknownNonLegacy
            } else {
                LegacyIngressSurface::ExplicitLegacy
            }
        })
}

fn surface_from_token(token: &str) -> Option<LegacyIngressSurface> {
    match token.trim().to_ascii_lowercase().as_str() {
        "sdk" => Some(LegacyIngressSurface::Sdk),
        "gateway" => Some(LegacyIngressSurface::Gateway),
        "dashboard" => Some(LegacyIngressSurface::Dashboard),
        "cli" | "typed_cli" => Some(LegacyIngressSurface::TypedCli),
        "legacy" | "explicit_legacy" => Some(LegacyIngressSurface::ExplicitLegacy),
        _ => None,
    }
}

fn has_reason(classification: &RequestClassification, reason: &str) -> bool {
    classification.reasons.iter().any(|row| row == reason)
}

fn is_trended_ambiguity_reason(reason: &str) -> bool {
    reason.starts_with("operation_candidates:")
        || reason.starts_with("resource_candidates:")
        || reason.starts_with("operation_kind:unknown")
        || reason.starts_with("surface_adapter_fallback:")
        || reason.starts_with("typed_probe_contract_missing")
        || reason == "parse_confidence_below_threshold"
}

fn rate(count: u32, total: u32) -> f32 {
    if total == 0 {
        0.0
    } else {
        count as f32 / total as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_ingress_budget_separates_surfaces_and_allows_explicit_legacy() {
        let report = build_legacy_ingress_budget_report(
            vec![
                classification_fixture(
                    LegacyIngressSurface::ExplicitLegacy,
                    true,
                    false,
                    0.80,
                    &[],
                ),
                classification_fixture(LegacyIngressSurface::Sdk, false, false, 0.90, &[]),
                classification_fixture(LegacyIngressSurface::Gateway, false, false, 0.90, &[]),
                classification_fixture(LegacyIngressSurface::Dashboard, false, false, 0.90, &[]),
                classification_fixture(LegacyIngressSurface::TypedCli, false, false, 0.90, &[]),
            ]
            .as_slice(),
        );
        assert!(report.ok);
        assert_eq!(report.rows.len(), 6);
        let explicit = report
            .rows
            .iter()
            .find(|row| row.surface == LegacyIngressSurface::ExplicitLegacy)
            .expect("explicit legacy row");
        assert_eq!(explicit.legacy_shim_count, 1);
        assert_eq!(explicit.max_legacy_shim_rate, 1.0);
        let sdk = report
            .rows
            .iter()
            .find(|row| row.surface == LegacyIngressSurface::Sdk)
            .expect("sdk row");
        assert_eq!(sdk.max_legacy_shim_rate, 0.0);
        assert_eq!(sdk.next_release_legacy_shim_rate_target, 0.0);
    }

    #[test]
    fn legacy_ingress_budget_fails_non_legacy_legacy_shim_and_fallback() {
        let report = build_legacy_ingress_budget_report(
            vec![classification_fixture(
                LegacyIngressSurface::Sdk,
                true,
                true,
                0.40,
                &["parse_confidence_below_threshold"],
            )]
            .as_slice(),
        );
        assert!(!report.ok);
        assert!(report
            .failures
            .iter()
            .any(|row| row.starts_with("legacy_ingress_budget_exceeded:sdk")));
        let sdk = report
            .rows
            .iter()
            .find(|row| row.surface == LegacyIngressSurface::Sdk)
            .expect("sdk row");
        assert_eq!(sdk.legacy_shim_count, 1);
        assert_eq!(sdk.surface_adapter_fallback_count, 1);
        assert_eq!(sdk.low_confidence_count, 1);
    }

    #[test]
    fn legacy_ingress_budget_tracks_probe_and_ambiguity_trends() {
        let report = build_legacy_ingress_budget_report(
            vec![classification_fixture(
                LegacyIngressSurface::Dashboard,
                false,
                false,
                0.90,
                &[
                    "typed_probe_contract_missing:core_probe_envelope",
                    "operation_candidates:2",
                    "resource_candidates:2",
                ],
            )]
            .as_slice(),
        );
        let dashboard = report
            .rows
            .iter()
            .find(|row| row.surface == LegacyIngressSurface::Dashboard)
            .expect("dashboard row");
        assert_eq!(dashboard.typed_probe_contract_violation_count, 1);
        assert_eq!(dashboard.ambiguity_reason_count, 3);
        assert!(report.release_over_release_tightening_delta > 0.0);
    }
}
