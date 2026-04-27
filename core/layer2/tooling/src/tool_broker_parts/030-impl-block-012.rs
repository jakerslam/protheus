impl ToolBroker {
    fn ledger_error<E: std::fmt::Display>(&self, context: &str, err: E) -> BrokerError {
        BrokerError::LedgerWriteFailed(format!("{context}:{}:{err}", self.ledger_path.display()))
    }

    pub fn allow_tool_for(&mut self, caller: BrokerCaller, tool_name: &str) {
        self.allowed_tools
            .entry(caller)
            .or_default()
            .insert(canonical_requested_tool_name(tool_name));
    }

    pub fn capability_catalog(&self) -> Vec<ToolCapability> {
        all_capabilities_for_callers(&self.allowed_tools)
    }

    pub fn grouped_capability_catalog(&self) -> Vec<ToolCapabilityCatalogGroup> {
        grouped_capabilities_for_callers(&self.allowed_tools)
    }

    pub fn backend_registry(&self) -> Vec<ToolBackendHealth> {
        live_backend_registry()
    }

    pub fn capability_probe(&self, caller: BrokerCaller, tool_name: &str) -> ToolCapabilityProbe {
        let canonical_name = canonical_requested_tool_name(tool_name);
        capability_probe_for(&self.allowed_tools, caller, canonical_name.as_str())
    }

    pub fn direct_tool_bypass_attempt(&self, caller: BrokerCaller) -> Result<(), BrokerError> {
        let caller_label = match caller {
            BrokerCaller::Client => "client",
            BrokerCaller::Worker => "worker",
            BrokerCaller::System => "system",
        };
        Err(BrokerError::DirectToolBypassDenied(format!(
            "tool_broker_required_for_external_calls:{caller_label}"
        )))
    }

    pub fn attempt_receipts(&self) -> &[ToolAttemptReceipt] {
        self.attempt_receipts.as_slice()
    }

    pub fn attempt_receipts_for_trace(&self, trace_id: &str) -> Vec<ToolAttemptReceipt> {
        let normalized = clean_text(trace_id, 160);
        self.attempt_receipts
            .iter()
            .filter(|row| row.trace_id == normalized)
            .cloned()
            .collect::<Vec<_>>()
    }

    pub fn execute_and_normalize<F>(
        &mut self,
        request: ToolCallRequest,
        executor: F,
    ) -> Result<ToolBrokerExecution, BrokerError>
    where
        F: FnOnce(&Value) -> Result<Value, String>,
    {
        let requested_tool_name = clean_text(&request.tool_name, 120).to_ascii_lowercase();
        let tool_name = canonical_requested_tool_name(&request.tool_name);
        let event_ts = now_ms();
        let probe = self.capability_probe(request.caller, &tool_name);
        if !probe.available {
            let attempt_status = match probe.reason_code {
                ToolReasonCode::UnknownTool | ToolReasonCode::TransportUnavailable => {
                    ToolAttemptStatus::Unavailable
                }
                ToolReasonCode::DaemonUnavailable | ToolReasonCode::WebsocketUnavailable => {
                    ToolAttemptStatus::Unavailable
                }
                ToolReasonCode::CallerNotAuthorized | ToolReasonCode::PolicyDenied => {
                    ToolAttemptStatus::Blocked
                }
                ToolReasonCode::AuthRequired => ToolAttemptStatus::Blocked,
                ToolReasonCode::BackendDegraded => ToolAttemptStatus::TransportError,
                ToolReasonCode::InvalidArgs => ToolAttemptStatus::InvalidArgs,
                ToolReasonCode::Timeout => ToolAttemptStatus::Timeout,
                ToolReasonCode::ExecutionError => ToolAttemptStatus::ExecutionError,
                ToolReasonCode::Ok => ToolAttemptStatus::Ok,
            };
            let attempt_receipt = self.record_attempt_receipt(AttemptReceiptInput {
                trace_id: request.trace_id.as_str(),
                task_id: request.task_id.as_str(),
                caller: request.caller,
                tool_name: tool_name.as_str(),
                status: attempt_status,
                reason: probe.reason.as_str(),
                reason_code: probe.reason_code,
                timestamp: event_ts,
                latency_ms: 0,
                probe: &probe,
            });
            let execution_receipt = build_tool_execution_receipt(ToolExecutionReceiptInput {
                attempt: &attempt_receipt,
                input_hash: input_hash_for_tool(&tool_name, &request.args),
                started_at: event_ts,
                ended_at: event_ts,
                data_ref: None,
                evidence_count: 0,
                error_code: error_code_for_attempt(&attempt_receipt),
            });
            self.execution_receipts.push(execution_receipt);
            return Err(BrokerError::UnauthorizedToolRequest(tool_name));
        }
        let normalized_args = match repair_and_validate_args(&tool_name, &request.args) {
            Ok(v) => v,
            Err(err) => {
                let attempt_receipt = self.record_attempt_receipt(AttemptReceiptInput {
                    trace_id: request.trace_id.as_str(),
                    task_id: request.task_id.as_str(),
                    caller: request.caller,
                    tool_name: tool_name.as_str(),
                    status: ToolAttemptStatus::InvalidArgs,
                    reason: "invalid_args",
                    reason_code: ToolReasonCode::InvalidArgs,
                    timestamp: event_ts,
                    latency_ms: 0,
                    probe: &probe,
                });
                let execution_receipt = build_tool_execution_receipt(ToolExecutionReceiptInput {
                    attempt: &attempt_receipt,
                    input_hash: input_hash_for_tool(&tool_name, &request.args),
                    started_at: event_ts,
                    ended_at: event_ts,
                    data_ref: None,
                    evidence_count: 0,
                    error_code: error_code_for_attempt(&attempt_receipt),
                });
                self.execution_receipts.push(execution_receipt);
                return Err(err);
            }
        };
        let route_hint = normalized_args
            .get("route_hint")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120))
            .unwrap_or_default();
        let synthesis_profile = normalized_args
            .get("synthesis_profile")
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120))
            .or_else(|| {
                let language = normalized_args
                    .get("language")
                    .and_then(Value::as_str)
                    .map(|v| clean_text(v, 60))
                    .unwrap_or_default();
                let provider = normalized_args
                    .get("provider")
                    .and_then(Value::as_str)
                    .map(|v| clean_text(v, 80))
                    .unwrap_or_default();
                let profile = match (language.is_empty(), provider.is_empty()) {
                    (false, false) => format!("provider={provider},language={language}"),
                    (false, true) => format!("language={language}"),
                    (true, false) => format!("provider={provider}"),
                    (true, true) => String::new(),
                };
                if profile.is_empty() {
                    None
                } else {
                    Some(profile)
                }
            })
            .unwrap_or_default();
        let context_mentions_count = normalized_args
            .get("context_mentions")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        let provider_surface_hint = normalized_args
            .get("provider")
            .or_else(|| normalized_args.get("provider_name"))
            .or_else(|| normalized_args.get("provider_family"))
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 80))
            .unwrap_or_default();
        let model_surface_hint = normalized_args
            .get("model")
            .or_else(|| normalized_args.get("model_id"))
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120))
            .unwrap_or_default();
        let policy_revision = clean_text(
            request.policy_revision.as_deref().unwrap_or("policy_v1"),
            120,
        );
        let tool_version = clean_text(request.tool_version.as_deref().unwrap_or("tool_v1"), 120);
        let freshness_window_ms = dedupe_freshness_window_ms(
            &tool_name,
            requested_freshness_window_ms(request.freshness_window_ms, &request.args),
        );
        let freshness_bucket = if freshness_window_ms == 0 {
            0
        } else {
            event_ts / freshness_window_ms
        };
        let dedupe_hash = deterministic_hash(&json!({
            "tool_name": tool_name,
            "normalized_args": normalized_args,
            "policy_revision": policy_revision,
            "tool_version": tool_version,
            "freshness_window_ms": freshness_window_ms,
            "freshness_bucket": freshness_bucket,
        }));
        let started = event_ts;
        let execution =
            execute_tool_with_payload_validation(&tool_name, &normalized_args, executor);
        let ended_at = now_ms();
        let duration_ms = ended_at.saturating_sub(started);
        let (status, raw_payload, errors) = match execution {
            Ok(raw_payload) => (NormalizedToolStatus::Ok, raw_payload, Vec::new()),
            Err(err) => (
                NormalizedToolStatus::Error,
                Value::Null,
                vec![clean_text(&err, 500)],
            ),
        };
        let (attempt_status, reason_code) = match status {
            NormalizedToolStatus::Ok => (ToolAttemptStatus::Ok, ToolReasonCode::Ok),
            NormalizedToolStatus::Blocked => {
                (ToolAttemptStatus::Blocked, ToolReasonCode::PolicyDenied)
            }
            NormalizedToolStatus::Error => (
                ToolAttemptStatus::ExecutionError,
                ToolReasonCode::ExecutionError,
            ),
        };
        let status_tag = match status {
            NormalizedToolStatus::Ok => "ok",
            NormalizedToolStatus::Error => "error",
            NormalizedToolStatus::Blocked => "blocked",
        };
        let attempt_receipt = self.record_attempt_receipt(AttemptReceiptInput {
            trace_id: request.trace_id.as_str(),
            task_id: request.task_id.as_str(),
            caller: request.caller,
            tool_name: tool_name.as_str(),
            status: attempt_status,
            reason: errors
                .first()
                .map(String::as_str)
                .unwrap_or(if status_tag == "ok" {
                    "ok"
                } else {
                    "execution_error"
                }),
            reason_code,
            timestamp: event_ts,
            latency_ms: duration_ms,
            probe: &probe,
        });
        let content_fingerprint = deterministic_hash(&json!({
            "kind": "normalized_tool_result_content",
            "tool_name": tool_name,
            "normalized_args": normalized_args,
            "status": status_tag,
            "raw_payload": raw_payload,
            "errors": errors,
            "policy_revision": policy_revision,
            "tool_version": tool_version
        }));
        let result_content_id = content_fingerprint.clone();
        let dedupe_allowed = matches!(status, NormalizedToolStatus::Ok) && !request.force_no_dedupe;
        let existing_result = if dedupe_allowed {
            self.dedupe_lookup.get(&dedupe_hash).cloned()
        } else {
            None
        };
        let result_id = existing_result.unwrap_or_else(|| result_content_id.clone());
        if dedupe_allowed {
            self.dedupe_lookup
                .entry(dedupe_hash.clone())
                .or_insert_with(|| result_id.clone());
        }
        self.event_sequence = self.event_sequence.saturating_add(1);
        let event_sequence = self.event_sequence;
        let event_id = deterministic_hash(&json!({
            "kind": "tool_execution_event",
            "trace_id": request.trace_id,
            "task_id": request.task_id,
            "caller": format!("{:?}", request.caller),
            "event_ts": event_ts,
            "event_sequence": event_sequence
        }));
        let raw_ref = format!("raw://{result_id}/{event_id}");
        self.raw_payloads
            .insert(raw_ref.clone(), raw_payload.clone());
        let metrics = NormalizedToolMetrics {
            duration_ms,
            output_bytes: serde_json::to_vec(&raw_payload)
                .map(|v| v.len())
                .unwrap_or(0),
        };
        let mut lineage = sanitize_lineage(&request.lineage);
        lineage.push(format!("policy_revision:{policy_revision}"));
        lineage.push(format!("tool_version:{tool_version}"));
        if freshness_window_ms > 0 {
            lineage.push(format!("freshness_window_ms:{freshness_window_ms}"));
            lineage.push(format!("freshness_bucket:{freshness_bucket}"));
        }
        if requested_tool_name != tool_name {
            lineage.push(format!("requested_tool_alias:{requested_tool_name}"));
            lineage.push(format!("canonical_tool:{tool_name}"));
        }
        if !route_hint.is_empty() {
            lineage.push(format!("route_hint:{route_hint}"));
        }
        if !synthesis_profile.is_empty() {
            lineage.push(format!("synthesis_profile:{synthesis_profile}"));
        }
        if context_mentions_count > 0 {
            lineage.push(format!("context_mentions_count:{context_mentions_count}"));
        }
        if !provider_surface_hint.is_empty() {
            lineage.push(format!("provider_surface:{provider_surface_hint}"));
        }
        if !model_surface_hint.is_empty() {
            lineage.push(format!("model_surface:{model_surface_hint}"));
        }
        lineage.push(format!("broker_event:{event_id}"));
        let normalized_result = NormalizedToolResult {
            result_id,
            result_content_id,
            result_event_id: event_id.clone(),
            trace_id: clean_text(&request.trace_id, 160),
            task_id: clean_text(&request.task_id, 160),
            tool_name,
            status,
            normalized_args: normalized_args.clone(),
            dedupe_hash,
            lineage,
            timestamp: event_ts,
            metrics,
            raw_ref,
            errors,
        };
        let ledger_event = ToolExecutionLedgerEvent {
            event_id,
            event_sequence,
            attempt_id: Some(attempt_receipt.attempt_id.clone()),
            attempt_sequence: attempt_receipt.attempt_sequence,
            result_id: normalized_result.result_id.clone(),
            result_content_id: normalized_result.result_content_id.clone(),
            trace_id: normalized_result.trace_id.clone(),
            task_id: normalized_result.task_id.clone(),
            caller: request.caller,
            tool_name: normalized_result.tool_name.clone(),
            status: normalized_result.status.clone(),
            dedupe_hash: normalized_result.dedupe_hash.clone(),
            policy_revision,
            tool_version,
            freshness_window_ms,
            freshness_bucket,
            raw_ref: normalized_result.raw_ref.clone(),
            timestamp: normalized_result.timestamp,
        };
        self.persist_ledger_event(&ledger_event)?;
        self.ledger_events.push(ledger_event);
        let execution_receipt = build_tool_execution_receipt(ToolExecutionReceiptInput {
            attempt: &attempt_receipt,
            input_hash: input_hash_for_tool(&normalized_result.tool_name, &normalized_args),
            started_at: started,
            ended_at,
            data_ref: Some(normalized_result.raw_ref.clone()),
            evidence_count: tool_payload_evidence_count(&raw_payload),
            error_code: error_code_for_attempt(&attempt_receipt),
        });
        self.execution_receipts.push(execution_receipt.clone());
        let attempt = ToolAttemptEnvelope {
            attempt: attempt_receipt,
            execution_receipt: execution_receipt.clone(),
            normalized_result: Some(normalized_result.clone()),
            raw_payload: Some(raw_payload.clone()),
            error: None,
        };
        Ok(ToolBrokerExecution {
            attempt,
            execution_receipt,
            normalized_result,
            raw_payload,
        })
    }
}
