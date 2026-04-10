impl MainNexusControlPlane {
    fn deny_delivery(
        &self,
        reason: impl Into<String>,
        lease_id: Option<String>,
    ) -> DirectDeliveryAuthorization {
        DirectDeliveryAuthorization {
            allowed: false,
            reason: reason.into(),
            local_resolution: false,
            lease_id,
            conduit_link_id: None,
        }
    }

    pub fn authorize_direct_delivery(
        &mut self,
        issuer: &str,
        input: DeliveryAuthorizationInput,
    ) -> DirectDeliveryAuthorization {
        let ts = input.now_ms.unwrap_or_else(now_ms);
        if input.source == input.target {
            self.metrics.local_resolution_count =
                self.metrics.local_resolution_count.saturating_add(1);
            if let Some(local) = self.sub_nexuses.get_mut(&input.source) {
                local.record_local_delivery();
            }
            self.refresh_metrics();
            return DirectDeliveryAuthorization {
                allowed: true,
                reason: "local_first_resolution".to_string(),
                local_resolution: true,
                lease_id: None,
                conduit_link_id: None,
            };
        }

        let Some(lease_id) = input.lease_id.clone() else {
            return self.deny_delivery("cross_module_delivery_requires_lease", None);
        };

        let Some(lease_snapshot) = self.leases.get(&lease_id).cloned() else {
            return self.deny_delivery("lease_missing", Some(lease_id));
        };

        if lease_snapshot.is_expired(ts) {
            self.revoke_lease(
                &lease_snapshot.lease_id,
                RevocationCause::Expired,
                issuer,
                ts,
            );
            return self.deny_delivery("lease_expired", Some(lease_snapshot.lease_id));
        }
        if lease_snapshot.is_revoked() {
            return self.deny_delivery("lease_revoked", Some(lease_snapshot.lease_id));
        }
        if self.registry.get(&input.source).is_none() || self.registry.get(&input.target).is_none()
        {
            self.revoke_lease(
                &lease_snapshot.lease_id,
                RevocationCause::RegistrationLost,
                issuer,
                ts,
            );
            return self.deny_delivery("registration_lost", Some(lease_snapshot.lease_id));
        }

        let source_lifecycle = self
            .registry
            .get(&input.source)
            .map(|row| row.lifecycle.clone())
            .unwrap_or(ModuleLifecycleState::Detached);
        let target_lifecycle = self
            .registry
            .get(&input.target)
            .map(|row| row.lifecycle.clone())
            .unwrap_or(ModuleLifecycleState::Detached);
        if !source_lifecycle.accepts_payload_delivery(ts)
            || !target_lifecycle.accepts_payload_delivery(ts)
        {
            return self.deny_delivery(
                "module_lifecycle_blocks_payload_delivery",
                Some(lease_snapshot.lease_id),
            );
        }

        let auth_input = LeaseAuthorizationInput {
            source: input.source.clone(),
            target: input.target.clone(),
            schema_id: input.schema_id,
            verb: input.verb,
            offered_verity: input.offered_verity,
            now_ms: ts,
        };
        if let Err(reason) = lease_snapshot.authorizes(&auth_input) {
            return self.deny_delivery(reason, Some(lease_snapshot.lease_id));
        }

        let (link, created) = self.conduit_manager.ensure_link(
            input.source.as_str(),
            input.target.as_str(),
            lease_snapshot.trust_class,
        );
        if created {
            let _ = self.emit_receipt(
                NexusReceiptKind::PlasticityEvent,
                issuer,
                Some(input.source.clone()),
                Some(input.target.clone()),
                vec!["nexus.plasticity".to_string()],
                None,
                None,
                None,
                None,
                None,
                json!({"event":"conduit_materialized","strategy":"lazy"}),
            );
        }
        self.conduit_manager
            .mark_used(input.source.as_str(), input.target.as_str(), ts);
        self.metrics.cross_module_resolution_count =
            self.metrics.cross_module_resolution_count.saturating_add(1);
        if let Some(source) = self.sub_nexuses.get_mut(&input.source) {
            source.record_cross_module_delivery();
        }
        self.refresh_metrics();
        DirectDeliveryAuthorization {
            allowed: true,
            reason: "authorized_via_lease".to_string(),
            local_resolution: false,
            lease_id: Some(lease_snapshot.lease_id),
            conduit_link_id: Some(link.link_id),
        }
    }

    pub fn revoke_leases_for_policy_change(
        &mut self,
        issuer: &str,
        source: Option<&str>,
        target: Option<&str>,
    ) -> usize {
        let ts = now_ms();
        let mut revoked = 0usize;
        let ids: Vec<String> = self
            .leases
            .values()
            .filter(|lease| {
                source.map(|row| row == lease.source).unwrap_or(true)
                    && target.map(|row| row == lease.target).unwrap_or(true)
            })
            .map(|lease| lease.lease_id.clone())
            .collect();
        for lease_id in ids {
            if self.revoke_lease(&lease_id, RevocationCause::PolicyChanged, issuer, ts) {
                revoked = revoked.saturating_add(1);
            }
        }
        self.refresh_metrics();
        revoked
    }

    pub fn teardown_idle_conduits(&mut self, issuer: &str, now_ms: u64) -> Vec<NexusReceipt> {
        let removed = self.conduit_manager.teardown_idle(now_ms);
        let mut receipts = Vec::new();
        for link in removed {
            receipts.push(self.emit_receipt(
                NexusReceiptKind::PlasticityEvent,
                issuer,
                Some(link.source),
                Some(link.target),
                vec!["nexus.plasticity".to_string()],
                None,
                None,
                None,
                None,
                None,
                json!({"event":"conduit_torn_down_idle","link_id": link.link_id}),
            ));
        }
        self.refresh_metrics();
        receipts
    }

    pub fn registry(&self) -> &NexusRegistry {
        &self.registry
    }

    pub fn receipts(&self) -> &[NexusReceipt] {
        self.receipts.as_slice()
    }

    pub fn metrics(&self) -> NexusMetrics {
        self.metrics.clone()
    }

    pub fn active_leases(&self) -> Vec<RouteLeaseCapability> {
        self.leases.values().cloned().collect()
    }

    pub fn active_conduits(&self) -> usize {
        self.conduit_manager.list().len()
    }
}
