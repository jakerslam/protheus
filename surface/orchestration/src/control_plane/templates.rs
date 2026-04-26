// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::WorkflowTemplate;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowSubtemplate {
    pub id: &'static str,
    pub description: &'static str,
    pub required_signals: &'static [&'static str],
    pub required_gates: &'static [&'static str],
    pub source_refs: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowTemplateDefinition {
    pub id: &'static str,
    pub description: &'static str,
    pub default_for_request_classes: &'static [&'static str],
    pub subtemplates: &'static [WorkflowSubtemplate],
}

#[derive(Debug, Clone, Deserialize)]
struct WorkflowTemplateSpec {
    #[serde(default)]
    name: String,
    #[serde(default)]
    id: String,
    description: String,
    #[serde(default)]
    stages: Vec<String>,
    #[serde(default)]
    default_for_request_classes: Vec<String>,
    #[serde(default)]
    subtemplates: Vec<WorkflowSubtemplateSpec>,
}

#[derive(Debug, Clone, Deserialize)]
struct WorkflowSubtemplateSpec {
    #[serde(default)]
    id: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    required_signals: Vec<String>,
    #[serde(default)]
    required_gates: Vec<String>,
    #[serde(default)]
    source_refs: Vec<String>,
}

const WORKFLOW_TEMPLATE_SPEC_CLARIFY_THEN_COORDINATE: &str =
    include_str!("workflows/clarify_then_coordinate.workflow.json");
const WORKFLOW_TEMPLATE_SPEC_RESEARCH_SYNTHESIZE_VERIFY: &str =
    include_str!("workflows/research_synthesize_verify.workflow.json");
const WORKFLOW_TEMPLATE_SPEC_PLAN_EXECUTE_REVIEW: &str =
    include_str!("workflows/plan_execute_review.workflow.json");
const WORKFLOW_TEMPLATE_SPEC_DIAGNOSE_RETRY_ESCALATE: &str =
    include_str!("workflows/diagnose_retry_escalate.workflow.json");
const WORKFLOW_TEMPLATE_SPEC_CODEX_TOOLING_SYNTHESIS: &str =
    include_str!("workflows/codex_tooling_synthesis.workflow.json");
const WORKFLOW_TEMPLATE_SPEC_FORGECODE_AGENT_COMPOSITION: &str =
    include_str!("workflows/forgecode_agent_composition.workflow.json");
const WORKFLOW_TEMPLATE_SPEC_FORGECODE_RAW_CAPABILITY_ASSIMILATION: &str =
    include_str!("workflows/forgecode_raw_capability_assimilation.workflow.json");
const WORKFLOW_TEMPLATE_SPEC_OPENHANDS_CONTROL_PLANE_ASSIMILATION: &str =
    include_str!("workflows/openhands_control_plane_assimilation.workflow.json");

const WORKFLOW_TEMPLATE_SPEC_SOURCES: &[(&str, &str)] = &[
    (
        "workflows/clarify_then_coordinate.workflow.json",
        WORKFLOW_TEMPLATE_SPEC_CLARIFY_THEN_COORDINATE,
    ),
    (
        "workflows/research_synthesize_verify.workflow.json",
        WORKFLOW_TEMPLATE_SPEC_RESEARCH_SYNTHESIZE_VERIFY,
    ),
    (
        "workflows/plan_execute_review.workflow.json",
        WORKFLOW_TEMPLATE_SPEC_PLAN_EXECUTE_REVIEW,
    ),
    (
        "workflows/diagnose_retry_escalate.workflow.json",
        WORKFLOW_TEMPLATE_SPEC_DIAGNOSE_RETRY_ESCALATE,
    ),
    (
        "workflows/codex_tooling_synthesis.workflow.json",
        WORKFLOW_TEMPLATE_SPEC_CODEX_TOOLING_SYNTHESIS,
    ),
    (
        "workflows/forgecode_agent_composition.workflow.json",
        WORKFLOW_TEMPLATE_SPEC_FORGECODE_AGENT_COMPOSITION,
    ),
    (
        "workflows/forgecode_raw_capability_assimilation.workflow.json",
        WORKFLOW_TEMPLATE_SPEC_FORGECODE_RAW_CAPABILITY_ASSIMILATION,
    ),
    (
        "workflows/openhands_control_plane_assimilation.workflow.json",
        WORKFLOW_TEMPLATE_SPEC_OPENHANDS_CONTROL_PLANE_ASSIMILATION,
    ),
];

static WORKFLOW_TEMPLATE_REGISTRY: OnceLock<HashMap<String, WorkflowTemplateDefinition>> =
    OnceLock::new();

const EMPTY_SUBTEMPLATES: &[WorkflowSubtemplate] = &[];

const CODEX_SUBTEMPLATES: &[WorkflowSubtemplate] = &[];

const FORGECODE_AGENT_SUBTEMPLATES: &[WorkflowSubtemplate] = &[];

const FORGECODE_RAW_CAPABILITY_SUBTEMPLATES: &[WorkflowSubtemplate] = &[];

fn template_id_for_enum(template: &WorkflowTemplate) -> &'static str {
    match template {
        WorkflowTemplate::ClarifyThenCoordinate => "clarify_then_coordinate",
        WorkflowTemplate::ResearchSynthesizeVerify => "research_synthesize_verify",
        WorkflowTemplate::PlanExecuteReview => "plan_execute_review",
        WorkflowTemplate::DiagnoseRetryEscalate => "diagnose_retry_escalate",
        WorkflowTemplate::CodexToolingSynthesis => "codex_tooling_synthesis",
        WorkflowTemplate::ForgeCodeAgentComposition => "forgecode_agent_composition",
        WorkflowTemplate::ForgeCodeRawCapabilityAssimilation => {
            "forgecode_raw_capability_assimilation"
        }
        WorkflowTemplate::OpenHandsControlPlaneAssimilation => {
            "openhands_control_plane_assimilation"
        }
    }
}

fn subtemplates_for_template_id(id: &str) -> &'static [WorkflowSubtemplate] {
    match id {
        "codex_tooling_synthesis" => CODEX_SUBTEMPLATES,
        "forgecode_agent_composition" => FORGECODE_AGENT_SUBTEMPLATES,
        "forgecode_raw_capability_assimilation" => FORGECODE_RAW_CAPABILITY_SUBTEMPLATES,
        _ => EMPTY_SUBTEMPLATES,
    }
}

fn leak_static_str(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn leak_static_str_slice(values: Vec<String>) -> &'static [&'static str] {
    let leaked = values
        .into_iter()
        .map(|row| row.trim().to_string())
        .filter(|row| !row.is_empty())
        .map(leak_static_str)
        .collect::<Vec<_>>();
    if leaked.is_empty() {
        return &[];
    }
    Box::leak(leaked.into_boxed_slice())
}

fn leak_static_subtemplate_slice(
    values: Vec<WorkflowSubtemplate>,
) -> &'static [WorkflowSubtemplate] {
    if values.is_empty() {
        return &[];
    }
    Box::leak(values.into_boxed_slice())
}

fn subtemplate_from_spec(spec: WorkflowSubtemplateSpec) -> Option<WorkflowSubtemplate> {
    let id = spec.id.trim().to_ascii_lowercase();
    if id.is_empty() {
        return None;
    }
    let id = leak_static_str(id);
    let description = if spec.description.trim().is_empty() {
        leak_static_str(format!("workflow subtemplate `{}`", id))
    } else {
        leak_static_str(spec.description.trim().to_string())
    };
    Some(WorkflowSubtemplate {
        id,
        description,
        required_signals: leak_static_str_slice(spec.required_signals),
        required_gates: leak_static_str_slice(spec.required_gates),
        source_refs: leak_static_str_slice(spec.source_refs),
    })
}

fn merge_json_subtemplates(
    base: &'static [WorkflowSubtemplate],
    extras: Vec<WorkflowSubtemplateSpec>,
) -> &'static [WorkflowSubtemplate] {
    if extras.is_empty() {
        return base;
    }
    let mut merged = base.to_vec();
    let mut seen: HashSet<&'static str> = merged.iter().map(|row| row.id).collect();
    let mut added = false;
    for raw in extras {
        if let Some(candidate) = subtemplate_from_spec(raw) {
            if seen.insert(candidate.id) {
                merged.push(candidate);
                added = true;
            }
        }
    }
    if !added {
        return base;
    }
    leak_static_subtemplate_slice(merged)
}

fn parse_template_spec(raw_spec: &str) -> Option<WorkflowTemplateSpec> {
    let parsed: WorkflowTemplateSpec = serde_json::from_str(raw_spec).ok()?;
    let id_source = if parsed.name.trim().is_empty() {
        parsed.id.trim()
    } else {
        parsed.name.trim()
    };
    let id = id_source.to_ascii_lowercase();
    if id.is_empty() {
        return None;
    }
    let has_stages = parsed.stages.iter().any(|row| !row.trim().is_empty());
    if !has_stages {
        return None;
    }
    Some(WorkflowTemplateSpec {
        name: id.clone(),
        id,
        description: parsed.description.trim().to_string(),
        stages: parsed.stages,
        default_for_request_classes: parsed.default_for_request_classes,
        subtemplates: parsed.subtemplates,
    })
}

fn definition_from_spec(spec: WorkflowTemplateSpec) -> WorkflowTemplateDefinition {
    let template_id = spec.id.clone();
    let id = leak_static_str(template_id.clone());
    let description = if spec.description.is_empty() {
        leak_static_str(format!("workflow template `{}`", template_id))
    } else {
        leak_static_str(spec.description)
    };
    let base_subtemplates = subtemplates_for_template_id(&template_id);
    let merged_subtemplates = merge_json_subtemplates(base_subtemplates, spec.subtemplates);
    WorkflowTemplateDefinition {
        id,
        description,
        default_for_request_classes: leak_static_str_slice(spec.default_for_request_classes),
        subtemplates: merged_subtemplates,
    }
}

fn workflow_template_definition_fallback(
    template: &WorkflowTemplate,
) -> WorkflowTemplateDefinition {
    match template {
        WorkflowTemplate::ClarifyThenCoordinate => WorkflowTemplateDefinition {
            id: "clarify_then_coordinate",
            description:
                "Clarification-first control-plane template used when request intent is ambiguous or clarification is mandatory before execution.",
            default_for_request_classes: &["read_only", "tool_call"],
            subtemplates: EMPTY_SUBTEMPLATES,
        },
        WorkflowTemplate::ResearchSynthesizeVerify => WorkflowTemplateDefinition {
            id: "research_synthesize_verify",
            description:
                "Research and synthesis template used for evidence-heavy retrieval turns and mixed tooling/web/workspace analysis.",
            default_for_request_classes: &["read_only", "tool_call"],
            subtemplates: EMPTY_SUBTEMPLATES,
        },
        WorkflowTemplate::PlanExecuteReview => WorkflowTemplateDefinition {
            id: "plan_execute_review",
            description:
                "Plan and execution template used for mutation/task proposal turns that require explicit sequencing and closure review.",
            default_for_request_classes: &["mutation", "task_proposal"],
            subtemplates: EMPTY_SUBTEMPLATES,
        },
        WorkflowTemplate::DiagnoseRetryEscalate => WorkflowTemplateDefinition {
            id: "diagnose_retry_escalate",
            description:
                "Recovery template used for blocked/failed states to route retry, reroute, or escalation decisions.",
            default_for_request_classes: &["tool_call", "mutation", "task_proposal"],
            subtemplates: EMPTY_SUBTEMPLATES,
        },
        WorkflowTemplate::CodexToolingSynthesis => WorkflowTemplateDefinition {
            id: "codex_tooling_synthesis",
            description:
                "Codex assimilation template used for tooling-heavy synthesis and deterministic multi-step assimilation waves.",
            default_for_request_classes: &["assimilation"],
            subtemplates: CODEX_SUBTEMPLATES,
        },
        WorkflowTemplate::ForgeCodeAgentComposition => WorkflowTemplateDefinition {
            id: "forgecode_agent_composition",
            description:
                "ForgeCode assimilation template that composes three specialized agent lanes (research, planning, implementation) into one single-agent master workflow.",
            default_for_request_classes: &["assimilation"],
            subtemplates: FORGECODE_AGENT_SUBTEMPLATES,
        },
        WorkflowTemplate::ForgeCodeRawCapabilityAssimilation => WorkflowTemplateDefinition {
            id: "forgecode_raw_capability_assimilation",
            description:
                "ForgeCode assimilation template that focuses on raw capability/mechanics extraction and direct runtime mapping without composed lane wrapper constraints.",
            default_for_request_classes: &["assimilation"],
            subtemplates: FORGECODE_RAW_CAPABILITY_SUBTEMPLATES,
        },
        WorkflowTemplate::OpenHandsControlPlaneAssimilation => WorkflowTemplateDefinition {
            id: "openhands_control_plane_assimilation",
            description:
                "OpenHands assimilation template for control-plane event-loop, replay, agent-registry, and limit-control mechanics.",
            default_for_request_classes: &["assimilation"],
            subtemplates: EMPTY_SUBTEMPLATES,
        },
    }
}

fn load_workflow_template_registry() -> HashMap<String, WorkflowTemplateDefinition> {
    let mut registry = HashMap::new();
    for (_source_path, raw_spec) in WORKFLOW_TEMPLATE_SPEC_SOURCES {
        if let Some(spec) = parse_template_spec(raw_spec) {
            let key = spec.id.clone();
            registry.insert(key, definition_from_spec(spec));
        }
    }
    for template in [
        WorkflowTemplate::ClarifyThenCoordinate,
        WorkflowTemplate::ResearchSynthesizeVerify,
        WorkflowTemplate::PlanExecuteReview,
        WorkflowTemplate::DiagnoseRetryEscalate,
        WorkflowTemplate::CodexToolingSynthesis,
        WorkflowTemplate::ForgeCodeAgentComposition,
        WorkflowTemplate::ForgeCodeRawCapabilityAssimilation,
        WorkflowTemplate::OpenHandsControlPlaneAssimilation,
    ] {
        let key = template_id_for_enum(&template).to_string();
        registry
            .entry(key)
            .or_insert_with(|| workflow_template_definition_fallback(&template));
    }
    registry
}

pub fn workflow_template_definition(template: &WorkflowTemplate) -> WorkflowTemplateDefinition {
    let registry = WORKFLOW_TEMPLATE_REGISTRY.get_or_init(load_workflow_template_registry);
    registry
        .get(template_id_for_enum(template))
        .cloned()
        .unwrap_or_else(|| workflow_template_definition_fallback(template))
}
