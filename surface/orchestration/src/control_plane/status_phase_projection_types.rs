// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusEventKind {
    WorkflowPhase,
    AgentActivity,
    ThinkingBubble,
    ContextWarning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StatusSourceAuthority {
    CoreRuntime,
    Orchestration,
    ShellOptimistic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActivityState {
    Idle,
    Working,
    Typing,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusProjectionAction {
    ProjectPhase {
        display_label: String,
        source: StatusSourceAuthority,
    },
    ProjectActivity {
        activity: AgentActivityState,
        display_label: String,
        source: StatusSourceAuthority,
    },
    ProjectThinkingBubble {
        display_label: String,
        source: StatusSourceAuthority,
    },
    ProjectContextWarning {
        display_label: String,
        source: StatusSourceAuthority,
    },
    RejectShellAuthoredInference {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEventEnvelope {
    pub kind: StatusEventKind,
    pub display_label: String,
    pub source: StatusSourceAuthority,
    pub activity: Option<AgentActivityState>,
    pub backend_event_id: Option<String>,
    pub optimistic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusProjectionPlan {
    pub action: StatusProjectionAction,
    pub telemetry_note: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusLabelCandidate {
    pub display_label: Option<String>,
    pub status_text: Option<String>,
    pub thinking_status: Option<String>,
    pub workflow_stage: Option<String>,
    pub stage: Option<String>,
    pub phase: Option<String>,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ShellStatusProjection {
    pub projection_type: &'static str,
    pub display_label: String,
    pub status_text: String,
    pub source: StatusSourceAuthority,
    pub activity: Option<AgentActivityState>,
    pub backend_event_id: Option<String>,
    pub optimistic: bool,
    pub telemetry_note: String,
}
