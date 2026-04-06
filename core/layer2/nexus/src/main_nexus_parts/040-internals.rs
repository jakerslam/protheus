impl MainNexusControlPlane {
    fn ensure_enabled(&self) -> Result<(), String> {
        if !self.feature_flags.hierarchical_nexus_enabled {
            return Err("hierarchical_nexus_disabled".to_string());
        }
        Ok(())
    }

    fn is_control_plane_only(schema_ids: &[String], verbs: &[String]) -> bool {
        schema_ids.iter().all(|schema| schema.starts_with("nexus."))
            && verbs.iter().all(|verb| {
                matches!(
                    verb.as_str(),
                    "register" | "transition" | "upsert" | "issue"
                )
            })
    }

    fn refresh_metrics(&mut self) {
        let total = self
            .metrics
            .local_resolution_count
            .saturating_add(self.metrics.cross_module_resolution_count);
        self.metrics.local_resolution_ratio = if total == 0 {
            1.0
        } else {
            self.metrics.local_resolution_count as f64 / total as f64
        };
        self.metrics.active_lease_count = self
            .leases
            .values()
            .filter(|lease| !lease.is_revoked() && !lease.is_expired(now_ms()))
            .count();
        self.metrics.revoked_lease_count = self
            .leases
            .values()
            .filter(|lease| lease.is_revoked())
            .count();
        self.metrics.active_conduit_count = self.conduit_manager.list().len();
    }

    fn emit_receipt(
        &mut self,
        kind: NexusReceiptKind,
        issuer: &str,
        source: Option<String>,
        target: Option<String>,
        schema_ids: Vec<String>,
        template_id: Option<String>,
        template_version: Option<u32>,
        ttl_ms: Option<u64>,
        policy_decision_ref: Option<PolicyDecisionRef>,
        revocation_cause: Option<RevocationCause>,
        metadata: Value,
    ) -> NexusReceipt {
        let ts = now_ms();
        let policy_ref = policy_decision_ref.map(|row| row.decision_id);
        let seed = json!({
            "kind": kind,
            "ts_ms": ts,
            "issuer": issuer,
            "source": source,
            "target": target,
            "schema_ids": schema_ids,
            "template_id": template_id,
            "template_version": template_version,
            "ttl_ms": ttl_ms,
            "policy_decision_ref": policy_ref,
            "revocation_cause": revocation_cause,
            "metadata": metadata
        });
        let receipt = NexusReceipt {
            receipt_id: format!("nexus_receipt_{}", deterministic_hash(&seed)),
            kind,
            ts_ms: ts,
            issuer: issuer.to_string(),
            source,
            target,
            schema_ids,
            template_id,
            template_version,
            ttl_ms,
            policy_decision_ref: seed
                .get("policy_decision_ref")
                .and_then(Value::as_str)
                .map(|row| row.to_string()),
            revocation_cause,
            metadata,
        };
        self.receipts.push(receipt.clone());
        receipt
    }

    fn revoke_lease(
        &mut self,
        lease_id: &str,
        cause: RevocationCause,
        issuer: &str,
        at_ms: u64,
    ) -> bool {
        let Some(mut lease) = self.leases.get(lease_id).cloned() else {
            return false;
        };
        if lease.revoked_at_ms.is_some() {
            return false;
        }
        lease.revoke(cause.clone(), at_ms);
        let source = lease.source.clone();
        let target = lease.target.clone();
        let schema_ids = lease.schema_ids.clone();
        let template_id = lease.template_id.clone();
        let template_version = lease.template_version;
        let decision = lease.policy_decision_ref.clone();
        self.leases.insert(lease.lease_id.clone(), lease);
        let _ = self.emit_receipt(
            NexusReceiptKind::LeaseRevoked,
            issuer,
            Some(source),
            Some(target),
            schema_ids,
            template_id,
            template_version,
            None,
            Some(decision),
            Some(cause),
            json!({"lease_id": lease_id}),
        );
        true
    }

    fn revoke_leases_for_node(
        &mut self,
        sub_nexus_id: &str,
        source_cause: RevocationCause,
        target_cause: RevocationCause,
        issuer: &str,
    ) {
        let ts = now_ms();
        let lease_ids: Vec<String> = self
            .leases
            .values()
            .filter(|lease| lease.source == sub_nexus_id || lease.target == sub_nexus_id)
            .map(|lease| lease.lease_id.clone())
            .collect();
        for lease_id in lease_ids {
            let cause = self
                .leases
                .get(&lease_id)
                .map(|lease| {
                    if lease.source == sub_nexus_id {
                        source_cause.clone()
                    } else {
                        target_cause.clone()
                    }
                })
                .unwrap_or(source_cause.clone());
            let _ = self.revoke_lease(&lease_id, cause, issuer, ts);
        }
    }

    fn sweep_expired_leases(&mut self, ts: u64, issuer: &str) {
        let expired_ids: Vec<String> = self
            .leases
            .values()
            .filter(|lease| lease.is_expired(ts) && !lease.is_revoked())
            .map(|lease| lease.lease_id.clone())
            .collect();
        for lease_id in expired_ids {
            let _ = self.revoke_lease(&lease_id, RevocationCause::Expired, issuer, ts);
        }
    }
}
