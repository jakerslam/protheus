pub fn compute_directive_clarification_exec_spec(
    input: &DirectiveClarificationExecSpecInput,
) -> DirectiveClarificationExecSpecOutput {
    let proposal_type =
        normalize_spaces(input.proposal_type.as_deref().unwrap_or("")).to_ascii_lowercase();
    if proposal_type != "directive_clarification" {
        return DirectiveClarificationExecSpecOutput {
            applicable: false,
            ok: false,
            reason: None,
            decision: None,
            objective_id: None,
            file: None,
            source: None,
            args: Vec::new(),
        };
    }

    let objective_id =
        sanitize_directive_objective_id(input.meta_directive_objective_id.as_deref().unwrap_or(""));
    let mut rel_file = if objective_id.is_empty() {
        String::new()
    } else {
        format!("client/runtime/config/directives/{objective_id}.yaml")
    };
    let mut source = if objective_id.is_empty() {
        String::new()
    } else {
        "meta.directive_objective_id".to_string()
    };
    if rel_file.is_empty() {
        let parsed = compute_parse_directive_file_arg(&ParseDirectiveFileArgInput {
            command: input.suggested_next_command.clone(),
        });
        if !parsed.file.is_empty() {
            rel_file = parsed.file;
            source = "suggested_next_command".to_string();
        }
    }

    if rel_file.is_empty() {
        return DirectiveClarificationExecSpecOutput {
            applicable: true,
            ok: false,
            reason: Some("directive_clarification_missing_file".to_string()),
            decision: None,
            objective_id: None,
            file: None,
            source: None,
            args: Vec::new(),
        };
    }

    let file_name = std::path::Path::new(&rel_file)
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_string();
    let file_name_lower = file_name.to_ascii_lowercase();
    let file_objective_id = if file_name_lower.ends_with(".yaml") {
        file_name[..file_name.len().saturating_sub(5)].to_string()
    } else if file_name_lower.ends_with(".yml") {
        file_name[..file_name.len().saturating_sub(4)].to_string()
    } else {
        file_name
    };
    let chosen_objective_id = if objective_id.is_empty() {
        file_objective_id
    } else {
        objective_id
    };

    DirectiveClarificationExecSpecOutput {
        applicable: true,
        ok: true,
        reason: None,
        decision: Some("DIRECTIVE_VALIDATE".to_string()),
        objective_id: Some(chosen_objective_id),
        file: Some(rel_file.clone()),
        source: if source.is_empty() {
            None
        } else {
            Some(source)
        },
        args: vec!["validate".to_string(), format!("--file={rel_file}")],
    }
}

pub fn compute_directive_decomposition_exec_spec(
    input: &DirectiveDecompositionExecSpecInput,
) -> DirectiveDecompositionExecSpecOutput {
    let proposal_type =
        normalize_spaces(input.proposal_type.as_deref().unwrap_or("")).to_ascii_lowercase();
    if proposal_type != "directive_decomposition" {
        return DirectiveDecompositionExecSpecOutput {
            applicable: false,
            ok: false,
            reason: None,
            decision: None,
            objective_id: None,
            source: None,
            args: Vec::new(),
        };
    }

    let objective_id =
        sanitize_directive_objective_id(input.meta_directive_objective_id.as_deref().unwrap_or(""));
    let command_id = compute_parse_directive_objective_arg(&ParseDirectiveObjectiveArgInput {
        command: input.suggested_next_command.clone(),
    })
    .objective_id;
    let chosen_id = if !objective_id.is_empty() {
        objective_id.clone()
    } else {
        command_id.clone()
    };
    let source = if !objective_id.is_empty() {
        "meta.directive_objective_id".to_string()
    } else if !command_id.is_empty() {
        "suggested_next_command".to_string()
    } else {
        String::new()
    };
    if chosen_id.is_empty() {
        return DirectiveDecompositionExecSpecOutput {
            applicable: true,
            ok: false,
            reason: Some("directive_decomposition_missing_objective_id".to_string()),
            decision: None,
            objective_id: None,
            source: None,
            args: Vec::new(),
        };
    }
    DirectiveDecompositionExecSpecOutput {
        applicable: true,
        ok: true,
        reason: None,
        decision: Some("DIRECTIVE_DECOMPOSE".to_string()),
        objective_id: Some(chosen_id.clone()),
        source: if source.is_empty() {
            None
        } else {
            Some(source)
        },
        args: vec!["decompose".to_string(), format!("--id={chosen_id}")],
    }
}

pub fn compute_parse_actuation_spec(input: &ParseActuationSpecInput) -> ParseActuationSpecOutput {
    let Some(proposal) = input.proposal.as_ref().and_then(|v| v.as_object()) else {
        return ParseActuationSpecOutput {
            has_spec: false,
            kind: None,
            params: None,
            context: None,
        };
    };

    let meta = proposal
        .get("meta")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let actuation = meta
        .get("actuation")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    if actuation.is_empty() {
        return ParseActuationSpecOutput {
            has_spec: false,
            kind: None,
            params: None,
            context: None,
        };
    }
    let kind = normalize_spaces(actuation.get("kind").and_then(|v| v.as_str()).unwrap_or(""));
    if kind.is_empty() {
        return ParseActuationSpecOutput {
            has_spec: false,
            kind: None,
            params: None,
            context: None,
        };
    }
    let params = actuation
        .get("params")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let action_spec = proposal
        .get("action_spec")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let guard_controls = meta
        .get("adaptive_mutation_guard_controls")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let guard_controls_obj = guard_controls.as_object().cloned().unwrap_or_default();

    let proposal_id = normalize_spaces(proposal.get("id").and_then(|v| v.as_str()).unwrap_or(""));
    let mut objective_id = String::new();
    for candidate in [
        meta.get("objective_id")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        meta.get("directive_objective_id")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        action_spec
            .get("objective_id")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    ] {
        let id = sanitize_directive_objective_id(candidate);
        if !id.is_empty() {
            objective_id = id;
            break;
        }
    }
    let first_non_empty = |vals: Vec<String>| -> Option<String> {
        vals.into_iter()
            .map(|v| normalize_spaces(&v))
            .find(|v| !v.is_empty())
    };
    let safety_attestation_id = first_non_empty(vec![
        guard_controls_obj
            .get("safety_attestation")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        meta.get("safety_attestation_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        meta.get("safety_attestation")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        meta.get("attestation_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    ]);
    let rollback_receipt_id = first_non_empty(vec![
        guard_controls_obj
            .get("rollback_receipt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        meta.get("rollback_receipt_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        meta.get("rollback_receipt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        action_spec
            .get("rollback_receipt_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    ]);
    let adaptive_mutation_guard_receipt_id = first_non_empty(vec![
        guard_controls_obj
            .get("guard_receipt_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        meta.get("adaptive_mutation_guard_receipt_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        meta.get("mutation_guard_receipt_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    ]);
    let applies = meta
        .get("adaptive_mutation_guard_applies")
        .and_then(|v| v.as_bool())
        == Some(true);
    let pass = meta
        .get("adaptive_mutation_guard_pass")
        .and_then(|v| v.as_bool())
        != Some(false);
    let reason = first_non_empty(vec![meta
        .get("adaptive_mutation_guard_reason")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()]);
    let reasons = meta
        .get("adaptive_mutation_guard_reasons")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .take(8)
                .cloned()
                .collect::<Vec<serde_json::Value>>()
        })
        .unwrap_or_default();

    ParseActuationSpecOutput {
        has_spec: true,
        kind: Some(kind),
        params: Some(params),
        context: Some(ParseActuationSpecContext {
            proposal_id: if proposal_id.is_empty() {
                None
            } else {
                Some(proposal_id)
            },
            objective_id: if objective_id.is_empty() {
                None
            } else {
                Some(objective_id)
            },
            safety_attestation_id,
            rollback_receipt_id,
            adaptive_mutation_guard_receipt_id,
            mutation_guard: ParseActuationSpecMutationGuard {
                applies,
                pass,
                reason,
                reasons,
                controls: guard_controls,
            },
        }),
    }
}

pub fn compute_task_from_proposal(input: &TaskFromProposalInput) -> TaskFromProposalOutput {
    let proposal_id = normalize_spaces(input.proposal_id.as_deref().unwrap_or(""));
    let proposal_id = if proposal_id.is_empty() {
        "unknown".to_string()
    } else {
        proposal_id
    };
    let proposal_type_raw = input.proposal_type.as_deref().unwrap_or("task").to_string();
    let proposal_type = Regex::new(r"[^a-z0-9_-]")
        .expect("valid proposal type sanitize regex")
        .replace_all(&proposal_type_raw.to_ascii_lowercase(), "")
        .to_string();
    let eyes_re = Regex::new(r"\[Eyes:[^\]]+\]\s*").expect("valid eyes strip regex");
    let title_raw = input.title.as_deref().unwrap_or("").to_string();
    let title_clean = eyes_re.replace_all(&title_raw, "").to_string();
    let title: String = title_clean.chars().take(140).collect();
    TaskFromProposalOutput {
        task: format!("Execute bounded proposal {proposal_id} ({proposal_type}): {title}"),
    }
}

pub fn compute_parse_objective_id_from_evidence_refs(
    input: &ParseObjectiveIdFromEvidenceRefsInput,
) -> ParseObjectiveIdFromEvidenceRefsOutput {
    let objective_set: std::collections::BTreeSet<String> = input
        .objective_ids
        .iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    let pulse_re =
        Regex::new(r"(?i)directive_pulse/([A-Za-z0-9_]+)").expect("valid pulse objective regex");
    let direct_re =
        Regex::new(r"(?i)\bdirective:([A-Za-z0-9_]+)").expect("valid direct objective regex");
    let fallback_re =
        Regex::new(r"\b(T[0-9]_[A-Za-z0-9_]+)\b").expect("valid fallback objective regex");
    for row in input.evidence_refs.iter() {
        let reference = normalize_spaces(row);
        if reference.is_empty() {
            continue;
        }
        let pulse_match = pulse_re
            .captures(&reference)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string());
        let direct_match = direct_re
            .captures(&reference)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string());
        let fallback_match = fallback_re
            .captures(&reference)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_string());
        let raw = normalize_spaces(
            pulse_match
                .as_deref()
                .or(direct_match.as_deref())
                .or(fallback_match.as_deref())
                .unwrap_or(""),
        );
        let sanitized =
            compute_sanitize_directive_objective_id(&SanitizeDirectiveObjectiveIdInput {
                value: Some(raw),
            });
        if sanitized.objective_id.is_empty() {
            continue;
        }
        let valid = objective_set.is_empty() || objective_set.contains(&sanitized.objective_id);
        return ParseObjectiveIdFromEvidenceRefsOutput {
            objective_id: Some(sanitized.objective_id),
            source: Some("evidence_ref".to_string()),
            valid: Some(valid),
        };
    }
    ParseObjectiveIdFromEvidenceRefsOutput {
        objective_id: None,
        source: None,
        valid: None,
    }
}

pub fn compute_parse_objective_id_from_command(
    input: &ParseObjectiveIdFromCommandInput,
) -> ParseObjectiveIdFromCommandOutput {
    let objective_set: std::collections::BTreeSet<String> = input
        .objective_ids
        .iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect();
    let objective_out = compute_parse_directive_objective_arg(&ParseDirectiveObjectiveArgInput {
        command: input.command.clone(),
    });
    if objective_out.objective_id.is_empty() {
        return ParseObjectiveIdFromCommandOutput {
            objective_id: None,
            source: None,
            valid: None,
        };
    }
    let valid = objective_set.is_empty() || objective_set.contains(&objective_out.objective_id);
    ParseObjectiveIdFromCommandOutput {
        objective_id: Some(objective_out.objective_id),
        source: Some("suggested_next_command".to_string()),
        valid: Some(valid),
    }
}
