// SPDX-License-Identifier: Apache-2.0
// Plane ownership: Safety, Cognition, Substrate, Assimilation stack.

#[path = "directed_assimilation_protocol_support.rs"]
mod support;

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use support::{
    hash_json, matches_hard_selector, now_iso, parse_csv, parse_output_contract,
    parse_transfer_class, short_hash, step_receipt,
};

const VECTOR_PARSER_VERSION: &str = "assimilation_vector_parser_v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HardSelectorMode {
    Constrain,
    Exclude,
    Require,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HardSelector {
    pub mode: HardSelectorMode,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SoftSelector {
    pub key: String,
    pub value: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferClass {
    AnalysisOnly,
    CleanRoomSpec,
    BehavioralClone,
    EmitterRetarget,
    DirectLift,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputContract {
    ObservationBundle,
    CapabilitySpec,
    BehaviorModel,
    IrCapsule,
    TestHarness,
    Patchset,
    EmitterPackage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentSpec {
    pub intent_id: String,
    pub goal: String,
    pub hard_selectors: Vec<HardSelector>,
    pub soft_selectors: Vec<SoftSelector>,
    pub destination: Option<String>,
    pub equivalence_target: String,
    pub proof_min: f64,
    pub uncertainty_budget: f64,
    pub halo_policy: String,
    pub degradation_policy: String,
    pub budget: u64,
    pub transfer_class: TransferClass,
    pub output_contract: OutputContract,
    pub license_policy: String,
    pub forbidden_license_classes: Vec<String>,
    pub allowed_transfer_classes: Vec<TransferClass>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProtocolStepReceipt {
    pub receipt_id: String,
    pub parent_receipt_ids: Vec<String>,
    pub step_kind: String,
    pub artifact_hash: String,
    pub policy_version: String,
    pub lineage_chain: Vec<String>,
    pub uncertainty_delta: f64,
    pub emitted_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorCompilationReceipt {
    pub original_text: String,
    pub normalized_intent_spec_hash: String,
    pub parser_version: String,
    pub policy_version: String,
    pub ambiguity_flags: Vec<String>,
    pub protocol_step_receipt: ProtocolStepReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReconIndex {
    pub recon_id: String,
    pub source_artifact_hash: String,
    pub extractor_fingerprint: String,
    pub schema_version: String,
    pub target_descriptor_ref: Option<String>,
    pub feature_refs: Vec<String>,
    pub region_refs: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateSet {
    pub candidate_ids: Vec<String>,
    pub source_refs: Vec<String>,
    pub selector_hits: Vec<String>,
    pub scores: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateClosure {
    pub candidate_id: String,
    pub closure_class: String,
    pub dependencies: Vec<String>,
    pub omissions: Vec<String>,
    pub omitted_dependency_risk: f64,
    pub closure_confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BindingPlan {
    pub binding_id: String,
    pub intent_id: String,
    pub candidate_refs: Vec<String>,
    pub closure_summary: String,
    pub capability_bindings: Vec<String>,
    pub consumer_demands: Vec<String>,
    pub target_descriptor_ref: Option<String>,
    pub transfer_class: TransferClass,
    pub output_contract: OutputContract,
    pub omission_risks: Vec<String>,
    pub legal_posture: String,
    pub assumption_set: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProvisionalGapReport {
    pub report_id: String,
    pub binding_id: String,
    pub observed_license_state: String,
    pub capability_gaps: Vec<String>,
    pub adaptation_cost: f64,
    pub environment_coupling: f64,
    pub degradation_likelihood: f64,
    pub required_emulation_shims: Vec<String>,
    pub contamination_risk: f64,
    pub fallback_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionVerdict {
    Accepted,
    AcceptedWithDegradation,
    DowngradedToAnalysisOnly,
    Rejected,
    NeedsOperatorOverride,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdmissionVerdictArtifact {
    pub verdict: AdmissionVerdict,
    pub reason_codes: Vec<String>,
    pub policy_refs: Vec<String>,
    pub required_receipts: Vec<String>,
    pub assumption_deltas: Vec<String>,
    pub rationale: String,
    pub protocol_step_receipt: ProtocolStepReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdmittedAssimilationPlan {
    pub plan_id: String,
    pub verdict_ref: String,
    pub approved_steps: Vec<String>,
    pub required_receipts: Vec<String>,
    pub degraded: bool,
    pub operator_override_ref: Option<String>,
    pub execution_budget: u64,
    pub target_descriptor_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    Observation,
    Extraction,
    Transplant,
    Emulation,
    Retarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssimilationOutcomeReceipt {
    pub outcome_kind: OutcomeKind,
    pub transfer: Option<TransferClass>,
    pub provenance_lineage: Vec<String>,
    pub capability_gaps: Vec<String>,
    pub final_license_state: String,
    pub contamination_class: String,
    pub degradation_fallback_details: String,
    pub uncertainty_delta: f64,
    pub final_trust_state: String,
    pub protocol_step_receipt: ProtocolStepReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DegradationReceipt {
    pub reason_codes: Vec<String>,
    pub blast_radius: String,
    pub capability_gaps: Vec<String>,
    pub operator_ack_required: bool,
    pub fallback_mode: String,
    pub assumption_deltas: Vec<String>,
    pub protocol_step_receipt: ProtocolStepReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverrideRequest {
    pub reason_codes: Vec<String>,
    pub blast_radius: String,
    pub capability_gaps: Vec<String>,
    pub operator_ack_required: bool,
    pub fallback_mode: String,
    pub assumption_deltas: Vec<String>,
    pub protocol_step_receipt: ProtocolStepReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectedAssimilationInput {
    pub vector_text: String,
    pub source_artifact_hash: String,
    pub target_descriptor_ref: Option<String>,
    pub feature_refs: Vec<String>,
    pub region_refs: Vec<String>,
    pub substrate_descriptors: Vec<String>,
    pub substrate_adapters: Vec<String>,
    pub substrate_execution_surfaces: Vec<String>,
    pub consumer_demands: Vec<String>,
    pub observed_license_state: String,
    pub capability_gaps: Vec<String>,
    pub adaptation_cost: f64,
    pub environment_coupling: f64,
    pub degradation_likelihood: f64,
    pub required_emulation_shims: Vec<String>,
    pub contamination_risk: f64,
    pub fallback_options: Vec<String>,
    pub operator_override_requested: bool,
    pub policy_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DirectedAssimilationArtifacts {
    pub vector_compilation_receipt: VectorCompilationReceipt,
    pub intent_spec: IntentSpec,
    pub recon_index: ReconIndex,
    pub candidate_set: CandidateSet,
    pub candidate_closure: CandidateClosure,
    pub binding_plan: BindingPlan,
    pub provisional_gap_report: ProvisionalGapReport,
    pub admission_verdict: AdmissionVerdictArtifact,
    pub admitted_assimilation_plan: Option<AdmittedAssimilationPlan>,
    pub degradation_receipt: Option<DegradationReceipt>,
    pub override_request: Option<OverrideRequest>,
    pub assimilation_outcome_receipt: AssimilationOutcomeReceipt,
    pub protocol_step_receipts: Vec<ProtocolStepReceipt>,
}

pub fn run_directed_assimilation(input: &DirectedAssimilationInput) -> Result<DirectedAssimilationArtifacts, String> {
    let (intent_spec, vector_compilation_receipt) = compile_assimilation_vector(&input.vector_text, &input.policy_version)?;
    let mut receipts = vec![vector_compilation_receipt.protocol_step_receipt.clone()];
    let recon_index = build_recon_index(input);
    receipts.push(step_receipt("recon_index", &input.source_artifact_hash, &input.policy_version, &receipts, 0.04));
    let candidate_set = build_candidate_set(&intent_spec, input);
    receipts.push(step_receipt("candidate_set", &input.source_artifact_hash, &input.policy_version, &receipts, 0.08));
    let candidate_closure = build_candidate_closure(&intent_spec, &candidate_set, input);
    receipts.push(step_receipt("candidate_closure", &input.source_artifact_hash, &input.policy_version, &receipts, 0.12));
    let binding_plan = build_binding_plan(&intent_spec, &recon_index, &candidate_set, &candidate_closure, input);
    receipts.push(step_receipt("binding_plan", &input.source_artifact_hash, &input.policy_version, &receipts, 0.16));
    let provisional_gap_report = build_gap_report(&binding_plan, input);
    receipts.push(step_receipt("provisional_gap_report", &input.source_artifact_hash, &input.policy_version, &receipts, 0.22));
    let admission_verdict = build_admission_verdict(&intent_spec, &provisional_gap_report, input, &receipts);
    receipts.push(admission_verdict.protocol_step_receipt.clone());
    let admitted_assimilation_plan = build_admitted_plan(&admission_verdict, &intent_spec, &recon_index, &receipts);
    if admitted_assimilation_plan.is_some() {
        receipts.push(step_receipt("admitted_assimilation_plan", &input.source_artifact_hash, &input.policy_version, &receipts, 0.28));
    }
    let degradation_receipt = build_degradation_receipt(&admission_verdict, &provisional_gap_report, &receipts, input);
    if let Some(ref row) = degradation_receipt {
        receipts.push(row.protocol_step_receipt.clone());
    }
    let override_request = build_override_request(&admission_verdict, &provisional_gap_report, &receipts, input);
    if let Some(ref row) = override_request {
        receipts.push(row.protocol_step_receipt.clone());
    }
    let assimilation_outcome_receipt = build_outcome_receipt(&intent_spec, &admission_verdict, &provisional_gap_report, &receipts, input);
    receipts.push(assimilation_outcome_receipt.protocol_step_receipt.clone());
    Ok(DirectedAssimilationArtifacts { vector_compilation_receipt, intent_spec, recon_index, candidate_set, candidate_closure, binding_plan, provisional_gap_report, admission_verdict, admitted_assimilation_plan, degradation_receipt, override_request, assimilation_outcome_receipt, protocol_step_receipts: receipts })
}

pub fn compile_assimilation_vector(vector_text: &str, policy_version: &str) -> Result<(IntentSpec, VectorCompilationReceipt), String> {
    let mut intent = default_intent();
    let mut ambiguity_flags = Vec::new();
    for token in vector_text.split_whitespace() {
        if let Some((mode, assignment)) = token.strip_prefix("hard:").and_then(|tail| tail.split_once(':')) {
            if let Some((key, value)) = assignment.split_once('=') {
                let parsed_mode = match mode { "constrain" => Some(HardSelectorMode::Constrain), "exclude" => Some(HardSelectorMode::Exclude), "require" => Some(HardSelectorMode::Require), _ => None };
                if let Some(mode) = parsed_mode { intent.hard_selectors.push(HardSelector { mode, key: key.to_string(), value: value.to_string() }); } else { ambiguity_flags.push(format!("unknown_hard_selector_mode:{mode}")); }
            } else { ambiguity_flags.push(format!("hard_selector_parse_error:{token}")); }
            continue;
        }
        if let Some(assignment) = token.strip_prefix("soft:") {
            if let Some((lhs, rhs)) = assignment.split_once('=') {
                let (value, weight) = rhs.split_once('@').map(|(v, w)| (v.to_string(), w.parse::<f64>().unwrap_or(0.5))).unwrap_or((rhs.to_string(), 0.5));
                intent.soft_selectors.push(SoftSelector { key: lhs.to_string(), value, weight: weight.clamp(0.0, 1.0) });
            } else { ambiguity_flags.push(format!("soft_selector_parse_error:{token}")); }
            continue;
        }
        if let Some((key, value)) = token.split_once('=') {
            match key {
                "goal" => intent.goal = value.replace('_', " "),
                "destination" => intent.destination = Some(value.to_string()),
                "equivalence_target" => intent.equivalence_target = value.to_string(),
                "proof_min" => intent.proof_min = value.parse::<f64>().unwrap_or(intent.proof_min),
                "uncertainty_budget" => intent.uncertainty_budget = value.parse::<f64>().unwrap_or(intent.uncertainty_budget),
                "halo_policy" => intent.halo_policy = value.to_string(),
                "degradation_policy" => intent.degradation_policy = value.to_string(),
                "budget" => intent.budget = value.parse::<u64>().unwrap_or(intent.budget),
                "transfer_class" => intent.transfer_class = parse_transfer_class(value).unwrap_or(intent.transfer_class),
                "output_contract" => intent.output_contract = parse_output_contract(value).unwrap_or(intent.output_contract),
                "license_policy" => intent.license_policy = value.to_string(),
                "forbidden_license_classes" => intent.forbidden_license_classes = parse_csv(value),
                "allowed_transfer_classes" => intent.allowed_transfer_classes = parse_csv(value).into_iter().filter_map(|item| parse_transfer_class(&item)).collect(),
                _ => ambiguity_flags.push(format!("unknown_vector_assignment:{key}")),
            }
            continue;
        }
        if token != "assimilate" && token != "artifact" && token != "surfaces" {
            ambiguity_flags.push(format!("unparsed_token:{token}"));
        }
    }
    if intent.allowed_transfer_classes.is_empty() { intent.allowed_transfer_classes = vec![TransferClass::AnalysisOnly, TransferClass::CleanRoomSpec, TransferClass::BehavioralClone, TransferClass::EmitterRetarget, TransferClass::DirectLift]; }
    for hard in &intent.hard_selectors {
        let conflicting = intent.hard_selectors.iter().any(|other| other.key == hard.key && other.value == hard.value && other.mode != hard.mode && (matches!(hard.mode, HardSelectorMode::Exclude) || matches!(other.mode, HardSelectorMode::Exclude)));
        if conflicting { ambiguity_flags.push(format!("conflicting_hard_selector:{}={}", hard.key, hard.value)); }
    }
    let normalized_intent_spec_hash = hash_json(&intent)?;
    intent.intent_id = format!("intent:{}", &normalized_intent_spec_hash[..16]);
    let protocol_step_receipt = step_receipt("vector_compilation", &normalized_intent_spec_hash, policy_version, &[], 0.02);
    let receipt = VectorCompilationReceipt { original_text: vector_text.to_string(), normalized_intent_spec_hash, parser_version: VECTOR_PARSER_VERSION.to_string(), policy_version: policy_version.to_string(), ambiguity_flags, protocol_step_receipt };
    Ok((intent, receipt))
}

fn default_intent() -> IntentSpec {
    IntentSpec {
        intent_id: String::new(),
        goal: "assimilate artifact surfaces with directed precision".to_string(),
        hard_selectors: Vec::new(),
        soft_selectors: Vec::new(),
        destination: None,
        equivalence_target: "behavioral_parity".to_string(),
        proof_min: 0.7,
        uncertainty_budget: 0.3,
        halo_policy: "strict".to_string(),
        degradation_policy: "explicit_receipt_required".to_string(),
        budget: 100,
        transfer_class: TransferClass::CleanRoomSpec,
        output_contract: OutputContract::CapabilitySpec,
        license_policy: "respect_source_license".to_string(),
        forbidden_license_classes: Vec::new(),
        allowed_transfer_classes: Vec::new(),
    }
}

fn build_recon_index(input: &DirectedAssimilationInput) -> ReconIndex {
    ReconIndex { recon_id: format!("recon:{}", short_hash(&input.source_artifact_hash)), source_artifact_hash: input.source_artifact_hash.clone(), extractor_fingerprint: "recon_extractor_v1".to_string(), schema_version: "recon_index_v1".to_string(), target_descriptor_ref: input.target_descriptor_ref.clone(), feature_refs: input.feature_refs.clone(), region_refs: input.region_refs.clone(), created_at: now_iso() }
}

fn build_candidate_set(intent: &IntentSpec, input: &DirectedAssimilationInput) -> CandidateSet {
    let mut rows = if input.substrate_descriptors.is_empty() { vec![("candidate:default".to_string(), "artifact_surface:default".to_string())] } else { input.substrate_descriptors.iter().enumerate().map(|(idx, source)| (format!("candidate:{idx:02}"), source.clone())).collect() };
    rows.retain(|(_, source)| intent.hard_selectors.iter().all(|selector| matches_hard_selector(selector, source)));
    let mut scored: Vec<(String, String, String, f64)> = rows.into_iter().map(|(id, source)| {
        let mut score = 0.5;
        let mut hits = Vec::new();
        for selector in &intent.soft_selectors {
            if source.contains(&selector.value) || source.contains(&selector.key) {
                score += selector.weight;
                hits.push(format!("{}={}", selector.key, selector.value));
            }
        }
        (id, source, hits.join("|"), score.min(1.0))
    }).collect();
    scored.sort_by(|left, right| right.3.partial_cmp(&left.3).unwrap_or(Ordering::Equal));
    CandidateSet { candidate_ids: scored.iter().map(|row| row.0.clone()).collect(), source_refs: scored.iter().map(|row| row.1.clone()).collect(), selector_hits: scored.iter().map(|row| row.2.clone()).collect(), scores: scored.iter().map(|row| row.3).collect() }
}

fn build_candidate_closure(intent: &IntentSpec, candidates: &CandidateSet, input: &DirectedAssimilationInput) -> CandidateClosure {
    let omissions = intent.hard_selectors.iter().filter(|selector| matches!(selector.mode, HardSelectorMode::Require) && !candidates.source_refs.iter().any(|source| source.contains(&selector.value))).map(|selector| format!("missing_required:{}={}", selector.key, selector.value)).collect::<Vec<_>>();
    let omitted_dependency_risk = if intent.hard_selectors.is_empty() { 0.0 } else { (omissions.len() as f64 / intent.hard_selectors.len() as f64).clamp(0.0, 1.0) };
    CandidateClosure { candidate_id: candidates.candidate_ids.first().cloned().unwrap_or_else(|| "candidate:none".to_string()), closure_class: if omissions.is_empty() { "closed".to_string() } else { "partial".to_string() }, dependencies: input.substrate_adapters.iter().chain(input.substrate_execution_surfaces.iter()).cloned().collect(), omissions, omitted_dependency_risk, closure_confidence: candidates.scores.first().cloned().unwrap_or(0.0) }
}

fn build_binding_plan(intent: &IntentSpec, recon: &ReconIndex, candidates: &CandidateSet, closure: &CandidateClosure, input: &DirectedAssimilationInput) -> BindingPlan {
    let capability_bindings = if input.capability_gaps.is_empty() { vec!["capability:full".to_string()] } else { input.capability_gaps.iter().map(|gap| format!("capability:{gap}")).collect() };
    BindingPlan { binding_id: format!("binding:{}", short_hash(&format!("{}:{}", intent.intent_id, closure.candidate_id))), intent_id: intent.intent_id.clone(), candidate_refs: candidates.candidate_ids.clone(), closure_summary: format!("closure_class={} confidence={:.2}", closure.closure_class, closure.closure_confidence), capability_bindings, consumer_demands: input.consumer_demands.clone(), target_descriptor_ref: recon.target_descriptor_ref.clone(), transfer_class: intent.transfer_class.clone(), output_contract: intent.output_contract.clone(), omission_risks: closure.omissions.clone(), legal_posture: format!("observed_license={} policy={}", input.observed_license_state, intent.license_policy), assumption_set: format!("equivalence_target={} uncertainty_budget={}", intent.equivalence_target, intent.uncertainty_budget) }
}

fn build_gap_report(binding: &BindingPlan, input: &DirectedAssimilationInput) -> ProvisionalGapReport {
    ProvisionalGapReport { report_id: format!("gap:{}", short_hash(&binding.binding_id)), binding_id: binding.binding_id.clone(), observed_license_state: input.observed_license_state.clone(), capability_gaps: input.capability_gaps.clone(), adaptation_cost: input.adaptation_cost, environment_coupling: input.environment_coupling, degradation_likelihood: input.degradation_likelihood, required_emulation_shims: input.required_emulation_shims.clone(), contamination_risk: input.contamination_risk, fallback_options: input.fallback_options.clone() }
}

fn build_admission_verdict(intent: &IntentSpec, gap: &ProvisionalGapReport, input: &DirectedAssimilationInput, receipts: &[ProtocolStepReceipt]) -> AdmissionVerdictArtifact {
    let mut reason_codes = Vec::new();
    let mut verdict = AdmissionVerdict::Accepted;
    if intent.forbidden_license_classes.contains(&gap.observed_license_state) { verdict = AdmissionVerdict::Rejected; reason_codes.push("license_class_forbidden".to_string()); }
    else if !intent.allowed_transfer_classes.contains(&intent.transfer_class) { verdict = AdmissionVerdict::Rejected; reason_codes.push("transfer_class_not_allowed".to_string()); }
    else if gap.contamination_risk >= 0.9 { verdict = AdmissionVerdict::Rejected; reason_codes.push("contamination_risk_critical".to_string()); }
    else if input.operator_override_requested { verdict = AdmissionVerdict::NeedsOperatorOverride; reason_codes.push("operator_override_requested".to_string()); }
    else if intent.transfer_class == TransferClass::AnalysisOnly { verdict = AdmissionVerdict::DowngradedToAnalysisOnly; reason_codes.push("analysis_only_transfer_class".to_string()); }
    else if gap.degradation_likelihood >= 0.5 || !gap.capability_gaps.is_empty() { verdict = AdmissionVerdict::AcceptedWithDegradation; reason_codes.push("degradation_required".to_string()); }
    if reason_codes.is_empty() { reason_codes.push("policy_clean_accept".to_string()); }
    let required_receipts = receipts.iter().map(|row| row.receipt_id.clone()).collect::<Vec<_>>();
    let protocol_step_receipt = step_receipt("admission_verdict", &input.source_artifact_hash, &input.policy_version, receipts, gap.degradation_likelihood.clamp(0.0, 1.0));
    AdmissionVerdictArtifact { verdict: verdict.clone(), reason_codes, policy_refs: vec![format!("policy:{}", input.policy_version), format!("license_policy:{}", intent.license_policy)], required_receipts: required_receipts.clone(), assumption_deltas: vec![format!("equivalence_target={}", intent.equivalence_target), format!("proof_min={}", intent.proof_min)], rationale: format!("admission_verdict={verdict:?} under policy {}", input.policy_version), protocol_step_receipt }
}

fn build_admitted_plan(verdict: &AdmissionVerdictArtifact, intent: &IntentSpec, recon: &ReconIndex, receipts: &[ProtocolStepReceipt]) -> Option<AdmittedAssimilationPlan> {
    match verdict.verdict {
        AdmissionVerdict::Rejected | AdmissionVerdict::NeedsOperatorOverride => None,
        AdmissionVerdict::Accepted | AdmissionVerdict::AcceptedWithDegradation | AdmissionVerdict::DowngradedToAnalysisOnly => {
            let degraded = matches!(verdict.verdict, AdmissionVerdict::AcceptedWithDegradation | AdmissionVerdict::DowngradedToAnalysisOnly);
            let approved_steps = if verdict.verdict == AdmissionVerdict::DowngradedToAnalysisOnly { vec!["analysis_bundle_only".to_string()] } else { vec!["recon_index".to_string(), "candidate_closure".to_string(), "binding_plan".to_string(), "emit_output_contract".to_string()] };
            Some(AdmittedAssimilationPlan { plan_id: format!("plan:{}", short_hash(&verdict.protocol_step_receipt.receipt_id)), verdict_ref: verdict.protocol_step_receipt.receipt_id.clone(), approved_steps, required_receipts: receipts.iter().map(|row| row.receipt_id.clone()).collect(), degraded, operator_override_ref: None, execution_budget: intent.budget, target_descriptor_ref: recon.target_descriptor_ref.clone() })
        }
    }
}

fn build_degradation_receipt(verdict: &AdmissionVerdictArtifact, gap: &ProvisionalGapReport, receipts: &[ProtocolStepReceipt], input: &DirectedAssimilationInput) -> Option<DegradationReceipt> {
    if verdict.verdict != AdmissionVerdict::AcceptedWithDegradation { return None; }
    Some(DegradationReceipt { reason_codes: verdict.reason_codes.clone(), blast_radius: "targeted_surface_scope".to_string(), capability_gaps: gap.capability_gaps.clone(), operator_ack_required: false, fallback_mode: gap.fallback_options.first().cloned().unwrap_or_else(|| "analysis_fallback".to_string()), assumption_deltas: verdict.assumption_deltas.clone(), protocol_step_receipt: step_receipt("degradation_receipt", &input.source_artifact_hash, &input.policy_version, receipts, gap.degradation_likelihood) })
}

fn build_override_request(verdict: &AdmissionVerdictArtifact, gap: &ProvisionalGapReport, receipts: &[ProtocolStepReceipt], input: &DirectedAssimilationInput) -> Option<OverrideRequest> {
    if verdict.verdict != AdmissionVerdict::NeedsOperatorOverride { return None; }
    Some(OverrideRequest { reason_codes: verdict.reason_codes.clone(), blast_radius: "cross_surface_policy_boundary".to_string(), capability_gaps: gap.capability_gaps.clone(), operator_ack_required: true, fallback_mode: "analysis_only_until_ack".to_string(), assumption_deltas: verdict.assumption_deltas.clone(), protocol_step_receipt: step_receipt("override_request", &input.source_artifact_hash, &input.policy_version, receipts, gap.contamination_risk) })
}

fn build_outcome_receipt(intent: &IntentSpec, verdict: &AdmissionVerdictArtifact, gap: &ProvisionalGapReport, receipts: &[ProtocolStepReceipt], input: &DirectedAssimilationInput) -> AssimilationOutcomeReceipt {
    let outcome_kind = match verdict.verdict {
        AdmissionVerdict::DowngradedToAnalysisOnly => OutcomeKind::Observation,
        AdmissionVerdict::Rejected => OutcomeKind::Observation,
        AdmissionVerdict::NeedsOperatorOverride => OutcomeKind::Observation,
        AdmissionVerdict::Accepted | AdmissionVerdict::AcceptedWithDegradation => match intent.transfer_class {
            TransferClass::AnalysisOnly => OutcomeKind::Observation,
            TransferClass::CleanRoomSpec => OutcomeKind::Extraction,
            TransferClass::BehavioralClone => OutcomeKind::Emulation,
            TransferClass::EmitterRetarget => OutcomeKind::Retarget,
            TransferClass::DirectLift => OutcomeKind::Transplant,
        },
    };
    let final_trust_state = match verdict.verdict { AdmissionVerdict::Accepted => "admitted", AdmissionVerdict::AcceptedWithDegradation => "degraded", AdmissionVerdict::DowngradedToAnalysisOnly => "analysis_only", AdmissionVerdict::Rejected => "rejected", AdmissionVerdict::NeedsOperatorOverride => "override_required" }.to_string();
    let contamination_class = if gap.contamination_risk >= 0.8 { "high" } else if gap.contamination_risk >= 0.4 { "medium" } else { "low" }.to_string();
    let transfer = if matches!(verdict.verdict, AdmissionVerdict::Rejected | AdmissionVerdict::NeedsOperatorOverride) { None } else { Some(intent.transfer_class.clone()) };
    let details = format!("fallback_options={} degradation_likelihood={:.2}", gap.fallback_options.join(","), gap.degradation_likelihood);
    AssimilationOutcomeReceipt { outcome_kind, transfer, provenance_lineage: receipts.iter().map(|row| row.receipt_id.clone()).collect(), capability_gaps: gap.capability_gaps.clone(), final_license_state: gap.observed_license_state.clone(), contamination_class, degradation_fallback_details: details, uncertainty_delta: ((gap.degradation_likelihood + gap.contamination_risk) / 2.0).clamp(0.0, 1.0), final_trust_state, protocol_step_receipt: step_receipt("assimilation_outcome_receipt", &input.source_artifact_hash, &input.policy_version, receipts, gap.degradation_likelihood) }
}
