impl MainNexusControlPlane {
    fn emit_conduit_materialized_if_created(
        &mut self,
        issuer: &str,
        source: &str,
        target: &str,
        trust_class: TrustClass,
        policy_decision: Option<PolicyDecisionRef>,
        metadata: Value,
    ) {
        let (_, created) = self
            .conduit_manager
            .ensure_link(source, target, trust_class);
        if created {
            let _ = self.emit_receipt(
                NexusReceiptKind::PlasticityEvent,
                issuer,
                Some(source.to_string()),
                Some(target.to_string()),
                vec!["nexus.plasticity".to_string()],
                None,
                None,
                None,
                policy_decision,
                None,
                metadata,
            );
        }
    }

    pub fn new(feature_flags: NexusFeatureFlags, policy: DefaultNexusPolicy) -> Self {
        Self {
            feature_flags,
            policy,
            registry: NexusRegistry::default(),
            template_registry: TemplateRegistry::default(),
            conduit_manager: ConduitManager::default(),
            sub_nexuses: BTreeMap::new(),
            leases: BTreeMap::new(),
            receipts: Vec::new(),
            metrics: NexusMetrics::default(),
        }
    }

    pub fn register_v1_adapters(&mut self, issuer: &str) -> Result<Vec<NexusReceipt>, String> {
        let adapters = vec![
            SubNexusRegistration::new(
                "stomach",
                ModuleKind::Stomach,
                TrustClass::InterModuleData,
                VerityClass::High,
            ),
            SubNexusRegistration::new(
                "context_stacks",
                ModuleKind::ContextStacks,
                TrustClass::InterModuleData,
                VerityClass::High,
            ),
            SubNexusRegistration::new(
                "client_ingress",
                ModuleKind::ClientIngress,
                TrustClass::ClientIngressBoundary,
                VerityClass::Standard,
            ),
        ];
        let mut out = Vec::new();
        for reg in adapters {
            if self.registry.contains(&reg.sub_nexus_id) {
                continue;
            }
            out.push(self.register_sub_nexus(issuer, reg)?);
        }
        Ok(out)
    }

    pub fn register_sub_nexus(
        &mut self,
        issuer: &str,
        registration: SubNexusRegistration,
    ) -> Result<NexusReceipt, String> {
        self.ensure_enabled()?;
        let context = PolicyEvaluationContext {
            issuer: issuer.to_string(),
            source: registration.sub_nexus_id.clone(),
            target: MAIN_NEXUS_ID.to_string(),
            schema_ids: vec!["nexus.registration".to_string()],
            verbs: vec!["register".to_string()],
            required_verity: registration.verity_class,
            template_id: None,
        };
        let policy_decision = self.policy.evaluate(&context);
        if !policy_decision.allow {
            return Err(format!("registration_denied:{}", policy_decision.reason));
        }
        self.registry.register(registration.clone())?;
        self.sub_nexuses.insert(
            registration.sub_nexus_id.clone(),
            SubNexus::from_registration(&registration),
        );
        self.emit_conduit_materialized_if_created(
            issuer,
            registration.sub_nexus_id.as_str(),
            MAIN_NEXUS_ID,
            TrustClass::InternalControl,
            Some(policy_decision.clone()),
            json!({"event":"conduit_materialized","local_first":true}),
        );
        let receipt = self.emit_receipt(
            NexusReceiptKind::Registration,
            issuer,
            Some(registration.sub_nexus_id.clone()),
            Some(MAIN_NEXUS_ID.to_string()),
            vec!["nexus.registration".to_string()],
            None,
            None,
            None,
            Some(policy_decision),
            None,
            json!({"module_kind": registration.module_kind, "trust_class": registration.trust_class}),
        );
        self.refresh_metrics();
        Ok(receipt)
    }

    pub fn set_module_lifecycle(
        &mut self,
        issuer: &str,
        sub_nexus_id: &str,
        next: ModuleLifecycleState,
    ) -> Result<NexusReceipt, String> {
        self.ensure_enabled()?;
        let prev = self
            .registry
            .get(sub_nexus_id)
            .map(|row| row.lifecycle.clone())
            .ok_or_else(|| "registration_missing".to_string())?;
        let context = PolicyEvaluationContext {
            issuer: issuer.to_string(),
            source: sub_nexus_id.to_string(),
            target: MAIN_NEXUS_ID.to_string(),
            schema_ids: vec!["nexus.lifecycle".to_string()],
            verbs: vec!["transition".to_string()],
            required_verity: VerityClass::Standard,
            template_id: None,
        };
        let policy_decision = self.policy.evaluate(&context);
        if !policy_decision.allow {
            return Err(format!(
                "lifecycle_transition_denied:{}",
                policy_decision.reason
            ));
        }

        self.registry.set_lifecycle(sub_nexus_id, next.clone())?;

        match next {
            ModuleLifecycleState::Quiesced => {
                self.revoke_leases_for_node(
                    sub_nexus_id,
                    RevocationCause::SourceQuiesced,
                    RevocationCause::TargetQuiesced,
                    issuer,
                );
            }
            ModuleLifecycleState::Detached => {
                self.revoke_leases_for_node(
                    sub_nexus_id,
                    RevocationCause::SourceDetached,
                    RevocationCause::TargetDetached,
                    issuer,
                );
                self.registry.unregister(sub_nexus_id);
                self.sub_nexuses.remove(sub_nexus_id);
                let removed = self.conduit_manager.remove_links_for_node(sub_nexus_id);
                if removed > 0 {
                    let _ = self.emit_receipt(
                        NexusReceiptKind::PlasticityEvent,
                        issuer,
                        Some(sub_nexus_id.to_string()),
                        None,
                        vec!["nexus.plasticity".to_string()],
                        None,
                        None,
                        None,
                        Some(policy_decision.clone()),
                        None,
                        json!({"event":"conduit_removed","count":removed}),
                    );
                }
            }
            ModuleLifecycleState::Active
            | ModuleLifecycleState::Draining { .. }
            | ModuleLifecycleState::Maintenance => {}
        }

        let receipt = self.emit_receipt(
            NexusReceiptKind::LifecycleTransition,
            issuer,
            Some(sub_nexus_id.to_string()),
            Some(MAIN_NEXUS_ID.to_string()),
            vec!["nexus.lifecycle".to_string()],
            None,
            None,
            None,
            Some(policy_decision),
            None,
            json!({"previous": prev, "next": next}),
        );
        self.refresh_metrics();
        Ok(receipt)
    }

    pub fn upsert_template(
        &mut self,
        issuer: &str,
        template: ConnectionTemplate,
    ) -> Result<NexusReceipt, String> {
        self.ensure_enabled()?;
        let context = PolicyEvaluationContext {
            issuer: issuer.to_string(),
            source: template.source.clone(),
            target: template.target.clone(),
            schema_ids: vec!["nexus.template".to_string()],
            verbs: vec!["upsert".to_string()],
            required_verity: template.required_verity,
            template_id: Some(template.template_id.clone()),
        };
        let policy_decision = self.policy.evaluate(&context);
        if !policy_decision.allow {
            return Err(format!("template_upsert_denied:{}", policy_decision.reason));
        }
        let template_id = template.template_id.clone();
        let template_fingerprint = template.template_fingerprint();
        self.template_registry.upsert(template.clone())?;
        Ok(self.emit_receipt(
            NexusReceiptKind::TemplateInstantiation,
            issuer,
            Some(template.source),
            Some(template.target),
            vec!["nexus.template".to_string()],
            Some(template_id),
            Some(template.version),
            Some(template.default_ttl_ms),
            Some(policy_decision),
            None,
            json!({"event":"template_upserted", "template_fingerprint": template_fingerprint}),
        ))
    }

    pub fn issue_route_lease_from_template(
        &mut self,
        issuer: &str,
        template_id: &str,
        version: u32,
        requested_ttl_ms: Option<u64>,
    ) -> Result<RouteLeaseCapability, String> {
        self.ensure_enabled()?;
        let template = self.template_registry.instantiate(template_id, version)?;
        let _ = self.emit_receipt(
            NexusReceiptKind::TemplateInstantiation,
            issuer,
            Some(template.source.clone()),
            Some(template.target.clone()),
            vec!["nexus.template".to_string()],
            Some(template.template_id.clone()),
            Some(template.version),
            Some(template.default_ttl_ms),
            None,
            None,
            json!({"event":"template_instantiated"}),
        );
        let req = LeaseIssueRequest {
            source: template.source.clone(),
            target: template.target.clone(),
            schema_ids: template.schema_ids.clone(),
            verbs: template.verbs.clone(),
            required_verity: template.required_verity,
            trust_class: template.trust_class,
            requested_ttl_ms: requested_ttl_ms.unwrap_or(template.default_ttl_ms),
            template_id: Some(template.template_id),
            template_version: Some(template.version),
        };
        self.issue_route_lease(issuer, req)
    }

    pub fn issue_route_lease(
        &mut self,
        issuer: &str,
        request: LeaseIssueRequest,
    ) -> Result<RouteLeaseCapability, String> {
        self.ensure_enabled()?;
        self.sweep_expired_leases(now_ms(), issuer);
        if request.schema_ids.is_empty() || request.verbs.is_empty() {
            return Err("lease_request_missing_schema_or_verb".to_string());
        }

        let source_reg = self
            .registry
            .get(&request.source)
            .cloned()
            .ok_or_else(|| "lease_source_missing".to_string())?;
        let target_reg = self
            .registry
            .get(&request.target)
            .cloned()
            .ok_or_else(|| "lease_target_missing".to_string())?;
        if !source_reg.lifecycle.accepts_new_leases() {
            return Err("lease_source_not_accepting_new_leases".to_string());
        }
        if !target_reg.lifecycle.accepts_new_leases() {
            return Err("lease_target_not_accepting_new_leases".to_string());
        }
        if (matches!(source_reg.lifecycle, ModuleLifecycleState::Maintenance)
            || matches!(target_reg.lifecycle, ModuleLifecycleState::Maintenance))
            && !Self::is_control_plane_only(request.schema_ids.as_slice(), request.verbs.as_slice())
        {
            return Err("lease_denied_maintenance_control_plane_only".to_string());
        }

        let context = PolicyEvaluationContext {
            issuer: issuer.to_string(),
            source: request.source.clone(),
            target: request.target.clone(),
            schema_ids: request.schema_ids.clone(),
            verbs: request.verbs.clone(),
            required_verity: request.required_verity,
            template_id: request.template_id.clone(),
        };
        let policy_decision = self.policy.evaluate(&context);
        if !policy_decision.allow {
            return Err(format!("lease_denied:{}", policy_decision.reason));
        }

        let bounded_ttl = request
            .requested_ttl_ms
            .max(1)
            .min(self.policy.max_ttl_ms(request.required_verity));
        let mut lease = RouteLeaseCapability::new(
            request.source.clone(),
            request.target.clone(),
            request.schema_ids.clone(),
            request.verbs.clone(),
            request.required_verity,
            request.trust_class,
            bounded_ttl,
            "pending",
            policy_decision.clone(),
            request.template_id.clone(),
            request.template_version,
        );

        let receipt = self.emit_receipt(
            NexusReceiptKind::LeaseIssued,
            issuer,
            Some(request.source.clone()),
            Some(request.target.clone()),
            request.schema_ids.clone(),
            request.template_id.clone(),
            request.template_version,
            Some(bounded_ttl),
            Some(policy_decision),
            None,
            json!({"lease_id": lease.lease_id, "verbs": request.verbs}),
        );
        lease.receipt_id = receipt.receipt_id.clone();
        self.leases.insert(lease.lease_id.clone(), lease.clone());

        self.emit_conduit_materialized_if_created(
            issuer,
            request.source.as_str(),
            request.target.as_str(),
            request.trust_class,
            None,
            json!({"event":"conduit_materialized","strategy":"lazy"}),
        );
        self.refresh_metrics();
        Ok(lease)
    }
}
