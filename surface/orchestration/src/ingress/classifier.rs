// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    AmbiguityReason, Mutability, OperationKind, ParseResult, PolicyScope, RequestKind,
    RequestSurface, ResourceKind, TargetDescriptor, TypedOrchestrationRequest,
};

pub fn select_operation_kind(candidates: &[OperationKind]) -> OperationKind {
    match candidates {
        [] => OperationKind::Unknown,
        [only] => only.clone(),
        many if many.iter().any(|row| row == &OperationKind::Mutate) => OperationKind::Mutate,
        many if many.iter().any(|row| row == &OperationKind::Search) => OperationKind::Search,
        many if many.iter().any(|row| row == &OperationKind::Fetch) => OperationKind::Fetch,
        many if many.iter().any(|row| row == &OperationKind::Plan) => OperationKind::Plan,
        [first, ..] => first.clone(),
    }
}

pub fn select_resource_kind(candidates: &[ResourceKind]) -> ResourceKind {
    match candidates {
        [] => ResourceKind::Unspecified,
        [only] => only.clone(),
        _ => ResourceKind::Mixed,
    }
}

pub fn infer_request_kind(
    operation_candidates: &[OperationKind],
    operation_kind: &OperationKind,
) -> RequestKind {
    if operation_candidates.len() > 1 {
        return RequestKind::Ambiguous;
    }
    match operation_kind {
        OperationKind::Compare => RequestKind::Comparative,
        OperationKind::Assimilate | OperationKind::Plan => RequestKind::Workflow,
        OperationKind::Unknown => RequestKind::Ambiguous,
        _ => RequestKind::Direct,
    }
}

pub fn infer_policy_scope(resource_kind: &ResourceKind, mutability: &Mutability) -> PolicyScope {
    match (resource_kind, mutability) {
        (ResourceKind::Web, _) => PolicyScope::WebOnly,
        (ResourceKind::Workspace, _) => PolicyScope::WorkspaceOnly,
        (_, Mutability::Proposal | Mutability::Mutation) => PolicyScope::CoreProposal,
        (ResourceKind::Mixed, _) => PolicyScope::CrossBoundary,
        _ => PolicyScope::Default,
    }
}

pub fn parse_diagnostics(
    typed_request: TypedOrchestrationRequest,
    operation_candidates: &[OperationKind],
    resource_candidates: &[ResourceKind],
    adapter_reasons: &[String],
) -> ParseResult {
    let surface_adapter_used = typed_request.adapted;
    let surface_adapter_fallback =
        !surface_adapter_used && !matches!(typed_request.surface, RequestSurface::Legacy);
    let mut confidence: f32 = 0.20;
    let mut ambiguity = Vec::new();
    let mut reasons = if surface_adapter_used {
        adapter_reasons.to_vec()
    } else {
        vec!["legacy_intent_compatibility_shim".to_string()]
    };

    if typed_request.operation_kind != OperationKind::Unknown {
        confidence += 0.30;
        reasons.push(format!("operation_kind:{:?}", typed_request.operation_kind).to_lowercase());
    } else {
        ambiguity.push(AmbiguityReason::UnknownOperation);
        reasons.push("operation_kind:unknown".to_string());
    }
    if typed_request.resource_kind != ResourceKind::Unspecified {
        confidence += 0.20;
        reasons.push(format!("resource_kind:{:?}", typed_request.resource_kind).to_lowercase());
    }
    if typed_request.request_kind != RequestKind::Ambiguous {
        confidence += 0.10;
    }
    if surface_adapter_used {
        confidence += 0.15;
        reasons.push("surface_native_typed_adapter".to_string());
    } else if surface_adapter_fallback {
        ambiguity.push(AmbiguityReason::SurfaceAdapterFallback);
        confidence -= 0.15;
        reasons
            .push(format!("surface_adapter_fallback:{:?}", typed_request.surface).to_lowercase());
    }
    if operation_candidates.len() > 1 {
        ambiguity.push(AmbiguityReason::MultipleOperationCandidates);
        confidence -= 0.20;
        reasons.push(format!(
            "operation_candidates:{}",
            operation_candidates.len()
        ));
    }
    if resource_candidates.len() > 1 {
        ambiguity.push(AmbiguityReason::MultipleResourceCandidates);
        confidence -= 0.10;
        reasons.push(format!("resource_candidates:{}", resource_candidates.len()));
    }
    if !typed_request.target_refs.is_empty() {
        confidence += 0.10;
        reasons.push("targets:present".to_string());
    } else if matches!(
        typed_request.operation_kind,
        OperationKind::Assimilate | OperationKind::Mutate
    ) {
        ambiguity.push(AmbiguityReason::MissingTargetSignals);
        confidence -= 0.10;
    }
    if typed_request
        .payload
        .as_object()
        .map(|row| row.is_empty())
        .unwrap_or(true)
    {
        ambiguity.push(AmbiguityReason::LegacyCompatOnly);
    }
    if typed_request
        .target_descriptors
        .iter()
        .any(|row| matches!(row, TargetDescriptor::Unknown { .. }))
    {
        ambiguity.push(AmbiguityReason::UnresolvedTargetDomain);
        confidence -= 0.05;
    }

    confidence = confidence.clamp(0.0, 0.99);
    if confidence < 0.55 {
        ambiguity.push(AmbiguityReason::LowConfidence);
    }

    ParseResult {
        typed_request,
        confidence,
        ambiguity,
        reasons,
        surface_adapter_used,
        surface_adapter_fallback,
    }
}
