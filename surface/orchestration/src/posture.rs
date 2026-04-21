use crate::contracts::{ExecutionPosture, RequestClass};

pub fn choose_execution_posture(
    request_class: RequestClass,
    needs_clarification: bool,
) -> ExecutionPosture {
    if needs_clarification {
        return ExecutionPosture::Ask;
    }
    match request_class {
        RequestClass::ReadOnly => ExecutionPosture::Act,
        RequestClass::ToolCall | RequestClass::TaskProposal | RequestClass::Assimilation => {
            ExecutionPosture::Verify
        }
        RequestClass::Mutation => ExecutionPosture::Verify,
    }
}

// Compatibility alias during control-plane naming transition.
pub fn choose_posture(request_class: RequestClass, needs_clarification: bool) -> ExecutionPosture {
    choose_execution_posture(request_class, needs_clarification)
}
