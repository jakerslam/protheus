// Layer ownership: orchestration (non-canonical orchestration coordination only).
use crate::contracts::WorkflowTemplate;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
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

static WORKFLOW_TEMPLATE_REGISTRY: OnceLock<HashMap<String, WorkflowTemplateDefinition>> =
    OnceLock::new();

const EMPTY_SUBTEMPLATES: &[WorkflowSubtemplate] = &[];

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
    let merged_subtemplates = merge_json_subtemplates(EMPTY_SUBTEMPLATES, spec.subtemplates);
    WorkflowTemplateDefinition {
        id,
        description,
        default_for_request_classes: leak_static_str_slice(spec.default_for_request_classes),
        subtemplates: merged_subtemplates,
    }
}

fn workflow_template_definition_missing(template: &WorkflowTemplate) -> WorkflowTemplateDefinition {
    WorkflowTemplateDefinition {
        id: template_id_for_enum(template),
        description: "",
        default_for_request_classes: &[],
        subtemplates: EMPTY_SUBTEMPLATES,
    }
}

fn workflow_template_directory_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(dir) = std::env::var("INFRING_ORCHESTRATION_WORKFLOW_DIR") {
        candidates.push(PathBuf::from(dir));
    }
    candidates.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/control_plane/workflows"));
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("orchestration/src/control_plane/workflows"));
    }
    candidates
}

fn collect_workflow_template_paths(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            paths.extend(collect_workflow_template_paths(&path));
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".workflow.json"))
            .unwrap_or(false)
        {
            paths.push(path);
        }
    }
    paths.sort();
    paths
}

fn workflow_template_specs_from_disk() -> Vec<String> {
    for dir in workflow_template_directory_candidates() {
        let specs = collect_workflow_template_paths(&dir)
            .into_iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .collect::<Vec<_>>();
        if !specs.is_empty() {
            return specs;
        }
    }
    Vec::new()
}

fn load_workflow_template_registry() -> HashMap<String, WorkflowTemplateDefinition> {
    let mut registry = HashMap::new();
    for raw_spec in workflow_template_specs_from_disk() {
        if let Some(spec) = parse_template_spec(&raw_spec) {
            let key = spec.id.clone();
            registry.insert(key, definition_from_spec(spec));
        }
    }
    registry
}

pub fn workflow_template_definition(template: &WorkflowTemplate) -> WorkflowTemplateDefinition {
    let registry = WORKFLOW_TEMPLATE_REGISTRY.get_or_init(load_workflow_template_registry);
    registry
        .get(template_id_for_enum(template))
        .cloned()
        .unwrap_or_else(|| workflow_template_definition_missing(template))
}
