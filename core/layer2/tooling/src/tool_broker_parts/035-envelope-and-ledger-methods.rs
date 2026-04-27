struct AttemptReceiptInput<'a> {
    trace_id: &'a str,
    task_id: &'a str,
    caller: BrokerCaller,
    tool_name: &'a str,
    status: ToolAttemptStatus,
    reason: &'a str,
    reason_code: ToolReasonCode,
    timestamp: u64,
    latency_ms: u64,
    probe: &'a ToolCapabilityProbe,
}

impl ToolBroker {
    pub fn execute_and_envelope<F>(
        &mut self,
        request: ToolCallRequest,
        executor: F,
    ) -> ToolAttemptEnvelope
    where
        F: FnOnce(&Value) -> Result<Value, String>,
    {
        let before_attempt = self.attempt_receipts.len();
        let before_receipt = self.execution_receipts.len();
        match self.execute_and_normalize(request.clone(), executor) {
            Ok(out) => out.attempt,
            Err(err) => {
                let attempt = self
                    .attempt_receipts
                    .get(before_attempt)
                    .cloned()
                    .or_else(|| self.attempt_receipts.last().cloned())
                    .unwrap_or_else(|| fallback_attempt_receipt(&request, &err));
                let execution_receipt = self
                    .execution_receipts
                    .get(before_receipt)
                    .cloned()
                    .or_else(|| self.execution_receipts.last().cloned())
                    .unwrap_or_else(|| fallback_execution_receipt(&attempt, &request, &err));
                ToolAttemptEnvelope {
                    attempt,
                    execution_receipt,
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

    pub fn execution_receipts(&self) -> &[ToolExecutionReceipt] {
        self.execution_receipts.as_slice()
    }

    pub fn execution_receipts_for_trace(&self, trace_id: &str) -> Vec<ToolExecutionReceipt> {
        let normalized = clean_text(trace_id, 160);
        self.execution_receipts
            .iter()
            .filter(|row| row.trace_id == normalized)
            .cloned()
            .collect::<Vec<_>>()
    }

    pub fn substrate_health_report(&self) -> ToolSubstrateHealthReport {
        let generated_at = now_ms();
        let backends = self.backend_registry();
        let catalog = self.capability_catalog();
        let mut report = ToolSubstrateHealthReport {
            generated_at,
            bounded_workspace_root: workspace_root_from_backends(&backends),
            backends,
            available_tool_count: count_tools(&catalog, ToolCapabilityStatus::Available),
            degraded_tool_count: count_tools(&catalog, ToolCapabilityStatus::Degraded),
            blocked_tool_count: count_tools(&catalog, ToolCapabilityStatus::Blocked),
            unavailable_tool_count: count_tools(&catalog, ToolCapabilityStatus::Unavailable),
            receipt_hash: String::new(),
        };
        report.receipt_hash = deterministic_hash(&json!({
            "kind": "tool_substrate_health_report",
            "generated_at": report.generated_at,
            "bounded_workspace_root": &report.bounded_workspace_root,
            "backends": &report.backends,
            "available_tool_count": report.available_tool_count,
            "degraded_tool_count": report.degraded_tool_count,
            "blocked_tool_count": report.blocked_tool_count,
            "unavailable_tool_count": report.unavailable_tool_count,
        }));
        report
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

    fn record_attempt_receipt(&mut self, input: AttemptReceiptInput<'_>) -> ToolAttemptReceipt {
        let attempt_sequence = self.attempt_receipts.len() as u64 + 1;
        let outcome = match input.status {
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
                "trace_id": input.trace_id,
                "task_id": input.task_id,
                "caller": format!("{:?}", input.caller).to_ascii_lowercase(),
                "tool_name": input.tool_name,
                "outcome": outcome,
                "timestamp": input.timestamp,
                "sequence": attempt_sequence
            })),
            attempt_sequence,
            trace_id: clean_text(input.trace_id, 160),
            task_id: clean_text(input.task_id, 160),
            caller: input.caller,
            tool_name: clean_text(input.tool_name, 120),
            status: input.status,
            outcome: clean_text(outcome, 40),
            reason_code: input.reason_code,
            reason: clean_text(input.reason, 300),
            latency_ms: input.latency_ms,
            required_args: input.probe.required_args.clone(),
            backend: clean_text(&input.probe.backend, 120),
            discoverable: input.probe.discoverable,
            timestamp: input.timestamp,
        };
        self.attempt_receipts.push(receipt.clone());
        receipt
    }
}

fn fallback_attempt_receipt(request: &ToolCallRequest, err: &BrokerError) -> ToolAttemptReceipt {
    ToolAttemptReceipt {
        attempt_id: deterministic_hash(&json!({
            "kind": "tool_attempt_receipt_fallback",
            "trace_id": &request.trace_id,
            "task_id": &request.task_id,
            "tool_name": &request.tool_name,
            "timestamp": now_ms()
        })),
        attempt_sequence: 0,
        trace_id: clean_text(&request.trace_id, 160),
        task_id: clean_text(&request.task_id, 160),
        caller: request.caller,
        tool_name: canonical_requested_tool_name(&request.tool_name),
        status: ToolAttemptStatus::ExecutionError,
        outcome: "error".to_string(),
        reason_code: ToolReasonCode::ExecutionError,
        reason: clean_text(&err.as_message(), 300),
        latency_ms: 0,
        required_args: Vec::new(),
        backend: "unknown".to_string(),
        discoverable: false,
        timestamp: now_ms(),
    }
}

fn fallback_execution_receipt(
    attempt: &ToolAttemptReceipt,
    request: &ToolCallRequest,
    err: &BrokerError,
) -> ToolExecutionReceipt {
    build_tool_execution_receipt(ToolExecutionReceiptInput {
        attempt,
        input_hash: input_hash_for_tool(&request.tool_name, &request.args),
        started_at: attempt.timestamp,
        ended_at: attempt.timestamp,
        data_ref: None,
        evidence_count: 0,
        error_code: Some(error_code_from_execution_error(&err.as_message())),
    })
}

fn workspace_root_from_backends(backends: &[ToolBackendHealth]) -> String {
    backends
        .iter()
        .find(|row| row.backend == "workspace_fs")
        .map(|row| row.source.clone())
        .unwrap_or_else(|| ".".to_string())
}

fn count_tools(catalog: &[ToolCapability], status: ToolCapabilityStatus) -> usize {
    catalog.iter().filter(|tool| tool.status == status).count()
}
