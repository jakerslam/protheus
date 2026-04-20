
impl ToolBroker {
    fn ledger_error<E: std::fmt::Display>(&self, context: &str, err: E) -> BrokerError {
        BrokerError::LedgerWriteFailed(format!("{context}:{}:{err}", self.ledger_path.display()))
    }

    pub fn allow_tool_for(&mut self, caller: BrokerCaller, tool_name: &str) {
        self.allowed_tools
            .entry(caller)
            .or_default()
            .insert(clean_text(tool_name, 120).to_ascii_lowercase());
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
        capability_probe_for(
            &self.allowed_tools,
            caller,
            clean_text(tool_name, 120).as_str(),
        )
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
        let tool_name = clean_text(&request.tool_name, 120).to_ascii_lowercase();
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
            self.record_attempt_receipt(
                request.trace_id.as_str(),
                request.task_id.as_str(),
                request.caller,
                tool_name.as_str(),
                attempt_status,
                probe.reason.as_str(),
                probe.reason_code,
                event_ts,
                0,
                &probe,
            );
            return Err(BrokerError::UnauthorizedToolRequest(tool_name));
        }
        let normalized_args = match repair_and_validate_args(&tool_name, &request.args) {
            Ok(v) => v,
            Err(err) => {
                self.record_attempt_receipt(
                    request.trace_id.as_str(),
                    request.task_id.as_str(),
                    request.caller,
                    tool_name.as_str(),
                    ToolAttemptStatus::InvalidArgs,
                    "invalid_args",
                    ToolReasonCode::InvalidArgs,
                    event_ts,
                    0,
                    &probe,
                );
                return Err(err);
            }
        };
        let policy_revision = clean_text(
            request.policy_revision.as_deref().unwrap_or("policy_v1"),
            120,
        );
        let tool_version = clean_text(request.tool_version.as_deref().unwrap_or("tool_v1"), 120);
        let freshness_window_ms =
            dedupe_freshness_window_ms(&tool_name, request.freshness_window_ms);
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
        let execution = executor(&normalized_args);
        let duration_ms = now_ms().saturating_sub(started);
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
        let attempt_receipt = self.record_attempt_receipt(
            request.trace_id.as_str(),
            request.task_id.as_str(),
            request.caller,
            tool_name.as_str(),
            attempt_status,
            errors
                .first()
                .map(String::as_str)
                .unwrap_or(if status_tag == "ok" {
                    "ok"
                } else {
                    "execution_error"
                }),
            reason_code,
            event_ts,
            duration_ms,
            &probe,
        );
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
        lineage.push(format!("broker_event:{event_id}"));
        let normalized_result = NormalizedToolResult {
            result_id,
            result_content_id,
            result_event_id: event_id.clone(),
            trace_id: clean_text(&request.trace_id, 160),
            task_id: clean_text(&request.task_id, 160),
            tool_name,
            status,
            normalized_args,
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
        let attempt = ToolAttemptEnvelope {
            attempt: attempt_receipt,
            normalized_result: Some(normalized_result.clone()),
            raw_payload: Some(raw_payload.clone()),
            error: None,
        };
        Ok(ToolBrokerExecution {
            attempt,
            normalized_result,
            raw_payload,
        })
    }

    pub fn execute_and_envelope<F>(
        &mut self,
        request: ToolCallRequest,
        executor: F,
    ) -> ToolAttemptEnvelope
    where
        F: FnOnce(&Value) -> Result<Value, String>,
    {
        let before = self.attempt_receipts.len();
        match self.execute_and_normalize(request.clone(), executor) {
            Ok(out) => out.attempt,
            Err(err) => {
                let attempt = self
                    .attempt_receipts
                    .get(before)
                    .cloned()
                    .or_else(|| self.attempt_receipts.last().cloned())
                    .unwrap_or_else(|| ToolAttemptReceipt {
                        attempt_id: deterministic_hash(&json!({
                            "kind": "tool_attempt_receipt_fallback",
                            "trace_id": request.trace_id,
                            "task_id": request.task_id,
                            "tool_name": request.tool_name,
                            "timestamp": now_ms()
                        })),
                        attempt_sequence: self.attempt_receipts.len() as u64 + 1,
                        trace_id: clean_text(&request.trace_id, 160),
                        task_id: clean_text(&request.task_id, 160),
                        caller: request.caller,
                        tool_name: clean_text(&request.tool_name, 120),
                        status: ToolAttemptStatus::ExecutionError,
                        outcome: "error".to_string(),
                        reason_code: ToolReasonCode::ExecutionError,
                        reason: clean_text(&err.as_message(), 300),
                        latency_ms: 0,
                        required_args: Vec::new(),
                        backend: "unknown".to_string(),
                        discoverable: false,
                        timestamp: now_ms(),
                    });
                ToolAttemptEnvelope {
                    attempt,
                    normalized_result: None,
                    raw_payload: None,
                    error: Some(err.as_message()),
                }
            }
        }
    }

    pub fn raw_payload(&self, raw_ref: &str) -> Option<&Value> {
        self.raw_payloads.get(raw_ref)
    }

    pub fn ledger_events(&self) -> &[ToolExecutionLedgerEvent] {
        self.ledger_events.as_slice()
    }

    pub fn ledger_path(&self) -> &PathBuf {
        &self.ledger_path
    }

    pub fn recover_from_ledger(&mut self) -> Result<usize, BrokerError> {
        if !self.ledger_path.exists() {
            return Ok(0);
        }
        let file = File::open(&self.ledger_path)
            .map_err(|err| self.ledger_error("open_for_recovery", err))?;
        self.dedupe_lookup.clear();
        self.ledger_events.clear();
        self.event_sequence = 0;
        let mut recovered = 0usize;
        for line in BufReader::new(file).lines() {
            let row = line.map_err(|err| self.ledger_error("read_recovery_line", err))?;
            let trimmed = row.trim();
            if trimmed.is_empty() {
                continue;
            }
            let event = serde_json::from_str::<ToolExecutionLedgerEvent>(trimmed)
                .map_err(|err| self.ledger_error("decode_recovery_line", err))?;
            self.event_sequence = self.event_sequence.max(event.event_sequence);
            self.dedupe_lookup
                .entry(event.dedupe_hash.clone())
                .or_insert_with(|| event.result_id.clone());
            self.ledger_events.push(event);
            recovered = recovered.saturating_add(1);
        }
        Ok(recovered)
    }

    fn persist_ledger_event(&self, event: &ToolExecutionLedgerEvent) -> Result<(), BrokerError> {
        if let Some(parent) = self.ledger_path.parent() {
            create_dir_all(parent).map_err(|err| self.ledger_error("create_dir", err))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.ledger_path)
            .map_err(|err| self.ledger_error("open", err))?;
        let row = serde_json::to_string(event)
            .map_err(|err| BrokerError::LedgerWriteFailed(format!("encode_event:{err}")))?;
        file.write_all(format!("{row}\n").as_bytes())
            .map_err(|err| self.ledger_error("append", err))?;
        Ok(())
    }

    fn record_attempt_receipt(
        &mut self,
        trace_id: &str,
        task_id: &str,
        caller: BrokerCaller,
        tool_name: &str,
        status: ToolAttemptStatus,
        reason: &str,
        reason_code: ToolReasonCode,
        timestamp: u64,
        latency_ms: u64,
        probe: &ToolCapabilityProbe,
    ) -> ToolAttemptReceipt {
        let attempt_sequence = self.attempt_receipts.len() as u64 + 1;
        let outcome = match status {
            ToolAttemptStatus::Ok => "ok",
            ToolAttemptStatus::Unavailable => "unavailable",
            ToolAttemptStatus::Blocked | ToolAttemptStatus::PolicyDenied => "blocked",
            ToolAttemptStatus::InvalidArgs
            | ToolAttemptStatus::ExecutionError
            | ToolAttemptStatus::TransportError
            | ToolAttemptStatus::Timeout => "error",
        };
        let receipt = ToolAttemptReceipt {
            attempt_id: deterministic_hash(&json!({
                "kind": "tool_attempt_receipt",
                "trace_id": trace_id,
                "task_id": task_id,
                "caller": format!("{caller:?}").to_ascii_lowercase(),
                "tool_name": tool_name,
                "outcome": outcome,
                "timestamp": timestamp,
                "sequence": attempt_sequence
            })),
            attempt_sequence,
            trace_id: clean_text(trace_id, 160),
            task_id: clean_text(task_id, 160),
            caller,
            tool_name: clean_text(tool_name, 120),
            status,
            outcome: clean_text(outcome, 40),
            reason_code,
            reason: clean_text(reason, 300),
            latency_ms,
            required_args: probe.required_args.clone(),
            backend: clean_text(&probe.backend, 120),
            discoverable: probe.discoverable,
            timestamp,
        };
        self.attempt_receipts.push(receipt.clone());
        receipt
    }
}
