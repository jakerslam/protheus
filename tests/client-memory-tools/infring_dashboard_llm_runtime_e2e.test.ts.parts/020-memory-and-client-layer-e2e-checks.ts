      {
        input: 'What were we doing one week ago? Return exact date and memory file path.',
      }
    );
    assert.strictEqual(memory.status, 200, 'memory query should return 200');
    const memoryLane = memory.body && memory.body.lane ? memory.body.lane : {};
    const memoryText = String(memoryLane.response || '');
    const memoryTools = Array.isArray(memoryLane.tools) ? memoryLane.tools : [];
    summary.checks.week_ago_no_placeholder = !/(<text response to user>|actual concrete response text)/i.test(memoryText);
    summary.checks.week_ago_memory_recall = Boolean(
      memory.body
      && memory.body.ok
      && /\b20\d{2}-\d{2}-\d{2}\b/.test(memoryText)
      && /local\/workspace\/memory\//.test(memoryText)
    );
    assert.strictEqual(summary.checks.week_ago_no_placeholder, true, 'week-ago response should not contain placeholder text');
    summary.checks.week_ago_used_memory_tool = memoryTools.some((tool) =>
      String(tool && tool.input ? tool.input : '').includes('local/workspace/memory/')
    );
    assert.strictEqual(summary.checks.week_ago_memory_recall, true, 'week-ago response should include exact date and memory path');
    assert.strictEqual(summary.checks.week_ago_used_memory_tool, true, 'week-ago response should include memory file read tool evidence');
    summary.evidence.week_ago = {
      response_excerpt: memoryText.slice(0, 300),
      tools: memoryTools.map((tool) => String(tool && tool.input ? tool.input : '')).slice(0, 4),
    };

    const clientLayer = await postAction(
      BASE_URL,
      'app.chat',
      {
        input: 'Summarize client layer now with memory entries, receipts, logs, health checks, attention queue, and cockpit.',
      }
    );
    assert.strictEqual(clientLayer.status, 200, 'client-layer query should return 200');
    const clientLane = clientLayer.body && clientLayer.body.lane ? clientLayer.body.lane : {};
    const clientText = String(clientLane.response || '');
    const clientTextLower = clientText.trim().toLowerCase();
    const placeholderLikeResponse =
      clientTextLower === '<text response to user>' ||
      clientTextLower === 'actual concrete response text' ||
      clientTextLower.includes('"response":"actual concrete response text"');
    summary.checks.client_layer_visibility = Boolean(
      clientLayer.body
      && clientLayer.body.ok
      && !placeholderLikeResponse
      && /memory|receipt|log|health|attention|cockpit/i.test(clientText)
    );
    assert.strictEqual(summary.checks.client_layer_visibility, true, 'client-layer response should expose runtime surfaces');
    summary.evidence.client_layer = {
      response_excerpt: clientText.slice(0, 300),
    };

    const suffix = String(Date.now()).slice(-6);
    const coordinatorShadow = `e2e-${suffix}-coord`;
    const researcherShadow = `e2e-${suffix}-res`;
    const swarm = await postAction(
      BASE_URL,
      'app.chat',
      {
        input: [
          'Run exactly these commands to create a swarm of subagents:',
          `infring-ops collab-plane launch-role --team=ops --role=coordinator --shadow=${coordinatorShadow}`,
          `infring-ops collab-plane launch-role --team=ops --role=researcher --shadow=${researcherShadow}`,
        ].join('\n'),
      }
    );
    assert.strictEqual(swarm.status, 200, 'swarm launch query should return 200');
    const swarmLane = swarm.body && swarm.body.lane ? swarm.body.lane : {};
    const swarmTools = Array.isArray(swarmLane.tools) ? swarmLane.tools : [];
    summary.checks.swarm_launch_commands_executed =
      swarmTools.filter((tool) => String(tool && tool.input ? tool.input : '').includes('collab-plane launch-role')).length >= 2;
    assert.strictEqual(summary.checks.swarm_launch_commands_executed, true, 'swarm action should execute launch-role commands');

    const snapshotAfter = await fetchJson(`${BASE_URL}/api/dashboard/snapshot`);
    assert.strictEqual(snapshotAfter.status, 200, 'snapshot-after endpoint should return 200');
    const collabString = JSON.stringify(
      snapshotAfter.body && snapshotAfter.body.collab && typeof snapshotAfter.body.collab === 'object'
        ? snapshotAfter.body.collab
        : {}
    );
    summary.checks.swarm_agents_visible_in_collab =
      collabString.includes(coordinatorShadow) && collabString.includes(researcherShadow);
    assert.strictEqual(summary.checks.swarm_agents_visible_in_collab, true, 'collab dashboard should contain newly created swarm shadows');
    summary.evidence.swarm = {
      lane_response: String(swarmLane.response || '').slice(0, 240),
      tool_inputs: swarmTools.map((tool) => String(tool && tool.input ? tool.input : '')).slice(0, 6),
      collab_contains: { coordinatorShadow, researcherShadow },
    };

    const terminalShadow = `e2e-${suffix}-term`;
    const createTerminalAgent = await fetchJson(
      `${BASE_URL}/api/agents`,
      {
        method: 'POST',
        body: JSON.stringify({ name: terminalShadow, role: 'builder' }),
      }
    );
    assert.strictEqual(createTerminalAgent.status, 200, 'terminal test agent create should return 200');
    const terminalAgentId = String(
      createTerminalAgent.body
      && (createTerminalAgent.body.id || createTerminalAgent.body.agent_id)
        ? (createTerminalAgent.body.id || createTerminalAgent.body.agent_id)
        : terminalShadow
    );

    const terminalFirst = await fetchJson(
      `${BASE_URL}/api/agents/${encodeURIComponent(terminalAgentId)}/terminal`,
      {
        method: 'POST',
        body: JSON.stringify({ command: 'cd client && pwd', cwd: ROOT }),
      }
    );
    assert.strictEqual(terminalFirst.status, 200, 'terminal first command should return 200');
    const terminalCwd = String((terminalFirst.body && terminalFirst.body.cwd) || '');

    const terminalSecond = await fetchJson(
      `${BASE_URL}/api/agents/${encodeURIComponent(terminalAgentId)}/terminal`,
      {
        method: 'POST',
        body: JSON.stringify({ command: 'pwd', cwd: terminalCwd }),
      }
    );
    assert.strictEqual(terminalSecond.status, 200, 'terminal second command should return 200');
    const terminalStdout = String((terminalSecond.body && terminalSecond.body.stdout) || '').trim();
    summary.checks.terminal_real_session_roundtrip = Boolean(
      terminalCwd.endsWith('/client')
      && terminalStdout.endsWith('/client')
    );
    assert.strictEqual(
      summary.checks.terminal_real_session_roundtrip,
      true,
      'terminal mode should preserve real shell cwd state across commands'
    );
    summary.evidence.terminal = {
      agent: terminalAgentId,
      first_cwd: terminalCwd,
      second_stdout: terminalStdout,
    };

    const runtimeSwarm = await postAction(
      BASE_URL,
      'dashboard.runtime.executeSwarmRecommendation',
      {}
    );
    assert.strictEqual(runtimeSwarm.status, 200, 'runtime swarm recommendation action should return 200');
    const runtimeSwarmLane = runtimeSwarm.body && runtimeSwarm.body.lane ? runtimeSwarm.body.lane : {};
    const runtimeSwarmResponse = String(runtimeSwarmLane.response || '');
    summary.checks.runtime_swarm_no_placeholder = !/(<text response to user>|actual concrete response text)/i.test(runtimeSwarmResponse);
    assert.strictEqual(
      summary.checks.runtime_swarm_no_placeholder,
      true,
      'runtime swarm recommendation should not contain placeholder response text'
    );
    summary.checks.runtime_swarm_recommendation_executed = !!(
      runtimeSwarm.body
      && runtimeSwarm.body.ok
      && runtimeSwarmLane.recommendation
      && Array.isArray(runtimeSwarmLane.turns)
      && runtimeSwarmLane.turns.length >= 1
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_recommendation_executed,
      true,
      'runtime swarm recommendation should execute at least one role turn'
    );
    summary.checks.runtime_swarm_policy_payload_present = Array.isArray(runtimeSwarmLane.policies);
    assert.strictEqual(
      summary.checks.runtime_swarm_policy_payload_present,
      true,
      'runtime swarm recommendation should include policy execution payload'
    );
    summary.checks.runtime_swarm_role_plan_present = Array.isArray(
      runtimeSwarmLane.recommendation && runtimeSwarmLane.recommendation.role_plan
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_role_plan_present,
      true,
      'runtime swarm recommendation should expose role plan'
    );
    const swarmScaleRequired = !!(
      runtimeSwarmLane.recommendation && runtimeSwarmLane.recommendation.swarm_scale_required
    );
    summary.checks.runtime_swarm_scale_metadata_present = !!(
      runtimeSwarmLane.recommendation
      && Number.isFinite(Number(runtimeSwarmLane.recommendation.active_swarm_agents))
      && Number.isFinite(Number(runtimeSwarmLane.recommendation.swarm_target_agents))
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_scale_metadata_present,
      true,
      'runtime swarm recommendation should expose active/target swarm capacity metadata'
    );
    summary.checks.runtime_swarm_reviewer_present_when_scaling =
      !swarmScaleRequired ||
      (Array.isArray(runtimeSwarmLane.recommendation.role_plan)
        && runtimeSwarmLane.recommendation.role_plan.some((row) => row && row.role === 'reviewer' && row.required === true));
    assert.strictEqual(
      summary.checks.runtime_swarm_reviewer_present_when_scaling,
      true,
      'runtime swarm recommendation should include reviewer role when swarm scaling is required'
    );
    const throttleRequired = !!(
      runtimeSwarmLane.recommendation && runtimeSwarmLane.recommendation.throttle_required
    );
    summary.checks.runtime_swarm_throttle_applied_when_required =
      !throttleRequired ||
      (Array.isArray(runtimeSwarmLane.policies)
        && runtimeSwarmLane.policies.some(
          (row) => row && row.policy === 'queue_throttle' && row.required === true && row.applied === true
        ));
    assert.strictEqual(
      summary.checks.runtime_swarm_throttle_applied_when_required,
      true,
      'runtime swarm recommendation should apply queue throttle when required'
    );
    summary.checks.runtime_swarm_predictive_drain_policy_present = !!(
      Array.isArray(runtimeSwarmLane.policies)
      && runtimeSwarmLane.policies.some((row) => row && row.policy === 'predictive_drain')
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_predictive_drain_policy_present,
      true,
      'runtime swarm recommendation should include predictive drain policy payload'
    );
    summary.checks.runtime_swarm_attention_drain_policy_present = !!(
      Array.isArray(runtimeSwarmLane.policies)
      && runtimeSwarmLane.policies.some((row) => row && row.policy === 'attention_queue_autodrain')
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_attention_drain_policy_present,
      true,
      'runtime swarm recommendation should include attention queue autodrain policy payload'
    );
    summary.checks.runtime_swarm_attention_compaction_policy_present = !!(
      Array.isArray(runtimeSwarmLane.policies)
      && runtimeSwarmLane.policies.some((row) => row && row.policy === 'attention_queue_compaction')
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_attention_compaction_policy_present,
      true,
      'runtime swarm recommendation should include attention queue compaction policy payload'
    );
    const coarseLaneDemotionPolicy = Array.isArray(runtimeSwarmLane.policies)
      ? runtimeSwarmLane.policies.find((row) => row && row.policy === 'coarse_lane_demotion')
      : null;
    const coarseConduitScaleUpPolicy = Array.isArray(runtimeSwarmLane.policies)
      ? runtimeSwarmLane.policies.find((row) => row && row.policy === 'coarse_conduit_scale_up')
      : null;
    const coarseStaleLaneDrainPolicy = Array.isArray(runtimeSwarmLane.policies)
      ? runtimeSwarmLane.policies.find((row) => row && row.policy === 'coarse_stale_lane_drain')
      : null;
    summary.checks.runtime_swarm_coarse_policy_payloads_present = !!(
      coarseLaneDemotionPolicy
      && coarseConduitScaleUpPolicy
      && coarseStaleLaneDrainPolicy
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_coarse_policy_payloads_present,
      true,
      'runtime swarm recommendation should include coarse-signal remediation policy payloads'
    );
    const coarseRemediationRequired = !!(
      runtimeSwarmLane.recommendation && runtimeSwarmLane.recommendation.coarse_signal_remediation_required
    );
    summary.checks.runtime_swarm_coarse_remediation_applied_when_required = !coarseRemediationRequired || (
      coarseLaneDemotionPolicy && coarseLaneDemotionPolicy.required === true && coarseLaneDemotionPolicy.applied === true
      && coarseConduitScaleUpPolicy && coarseConduitScaleUpPolicy.required === true && coarseConduitScaleUpPolicy.applied === true
      && coarseStaleLaneDrainPolicy && coarseStaleLaneDrainPolicy.required === true && coarseStaleLaneDrainPolicy.applied === true
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_coarse_remediation_applied_when_required,
      true,
      'runtime swarm recommendation should apply coarse remediation trio when coarse signal is detected'
    );
    const spineReliabilityPolicy = Array.isArray(runtimeSwarmLane.policies)
      ? runtimeSwarmLane.policies.find((row) => row && row.policy === 'spine_reliability_gate')
      : null;
    const humanEscalationGuardPolicy = Array.isArray(runtimeSwarmLane.policies)
      ? runtimeSwarmLane.policies.find((row) => row && row.policy === 'human_escalation_guard')
      : null;
    const runtimeSloGatePolicy = Array.isArray(runtimeSwarmLane.policies)
      ? runtimeSwarmLane.policies.find((row) => row && row.policy === 'runtime_slo_gate')
      : null;
    summary.checks.runtime_swarm_reliability_policy_payloads_present = !!(
      spineReliabilityPolicy && humanEscalationGuardPolicy
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_reliability_policy_payloads_present,
      true,
      'runtime swarm recommendation should include reliability guard policy payloads'
    );
    const reliabilityGateRequired = !!(
      runtimeSwarmLane.recommendation && runtimeSwarmLane.recommendation.reliability_gate_required
    );
    summary.checks.runtime_swarm_reliability_gate_applied_when_required = !reliabilityGateRequired || (
      spineReliabilityPolicy && spineReliabilityPolicy.required === true && spineReliabilityPolicy.applied === true
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_reliability_gate_applied_when_required,
      true,
      'runtime swarm recommendation should apply spine reliability gate when required'
    );
    summary.checks.runtime_swarm_slo_gate_payload_present = !!(
      runtimeSwarmLane.recommendation
      && runtimeSwarmLane.recommendation.slo_gate
      && runtimeSloGatePolicy
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_slo_gate_payload_present,
      true,
      'runtime swarm recommendation should include runtime SLO gate payload'
    );
    const sloGateRequired = !!(
      runtimeSwarmLane.recommendation && runtimeSwarmLane.recommendation.slo_gate_required
    );
    summary.checks.runtime_swarm_slo_gate_applied_when_required =
      !sloGateRequired ||
      (runtimeSloGatePolicy && runtimeSloGatePolicy.required === true && runtimeSloGatePolicy.applied === true);
    assert.strictEqual(
      summary.checks.runtime_swarm_slo_gate_applied_when_required,
      true,
      'runtime swarm recommendation should enforce runtime SLO gate when required'
    );
    summary.checks.runtime_swarm_slo_gate_thresholds_present = !!(
      runtimeSloGatePolicy
      && runtimeSloGatePolicy.thresholds
      && Number.isFinite(Number(runtimeSloGatePolicy.thresholds.spine_success_rate_min))
      && Number.isFinite(Number(runtimeSloGatePolicy.thresholds.receipt_latency_p95_max_ms))
      && Number.isFinite(Number(runtimeSloGatePolicy.thresholds.receipt_latency_p99_max_ms))
      && Number.isFinite(Number(runtimeSloGatePolicy.thresholds.queue_depth_max))
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_slo_gate_thresholds_present,
      true,
      'runtime SLO gate should expose threshold payload for policy enforcement'
    );
    summary.checks.runtime_swarm_no_invalid_conduit_command = !(
      Array.isArray(runtimeSwarmLane.policies)
      && runtimeSwarmLane.policies.some((row) => String((row && row.command) || '').includes('infring-ops conduit auto-balance'))
    );
    assert.strictEqual(
      summary.checks.runtime_swarm_no_invalid_conduit_command,
      true,
      'runtime swarm recommendation should avoid invalid conduit auto-balance command'
    );
    summary.evidence.runtime_swarm = {
      recommendation: runtimeSwarmLane.recommendation || null,
      executed_count: Number(runtimeSwarmLane.executed_count || 0),
      policies: Array.isArray(runtimeSwarmLane.policies) ? runtimeSwarmLane.policies : [],
      launches: Array.isArray(runtimeSwarmLane.launches) ? runtimeSwarmLane.launches : [],
      turns: Array.isArray(runtimeSwarmLane.turns)
        ? runtimeSwarmLane.turns.map((row) => ({
            role: row.role,
            shadow: row.shadow,
            ok: row.ok,
            response_excerpt: String(row.response || '').slice(0, 200),
          }))
        : [],
    };

    const archiveShadow = `e2e-${suffix}-archive`;
    const statusBeforeArchive = await fetchJson(`${BASE_URL}/api/status`);
    assert.strictEqual(statusBeforeArchive.status, 200, 'status-before-archive should return 200');
    const beforeArchiveCount = Number(
      statusBeforeArchive.body && statusBeforeArchive.body.agent_count != null
        ? statusBeforeArchive.body.agent_count
        : 0
    );

    const createArchiveAgent = await fetchJson(
      `${BASE_URL}/api/agents`,
      {
        method: 'POST',
        body: JSON.stringify({ name: archiveShadow, role: 'analyst' }),
      }
    );
    assert.strictEqual(createArchiveAgent.status, 200, 'archive-target agent create should return 200');
    const archiveAgentId = String(
      createArchiveAgent.body
      && (createArchiveAgent.body.id || createArchiveAgent.body.agent_id)
        ? (createArchiveAgent.body.id || createArchiveAgent.body.agent_id)
        : archiveShadow
    );

    const statusAfterCreate = await fetchJson(`${BASE_URL}/api/status`);
    assert.strictEqual(statusAfterCreate.status, 200, 'status-after-create should return 200');
    const afterCreateCount = Number(
      statusAfterCreate.body && statusAfterCreate.body.agent_count != null
        ? statusAfterCreate.body.agent_count
        : 0
    );
    summary.checks.archive_create_increments_count = afterCreateCount >= beforeArchiveCount;

    const archiveResult = await fetchJson(`${BASE_URL}/api/agents/${encodeURIComponent(archiveAgentId)}`, {
      method: 'DELETE',
    });
    assert.strictEqual(archiveResult.status, 200, 'archive should return 200');
    summary.checks.archive_delete_acknowledged = !!(
      archiveResult.body
      && archiveResult.body.archived === true
      && archiveResult.body.state === 'inactive'
    );

    const statusAfterArchive = await fetchJson(`${BASE_URL}/api/status`);
    assert.strictEqual(statusAfterArchive.status, 200, 'status-after-archive should return 200');
    const afterArchiveCount = Number(
      statusAfterArchive.body && statusAfterArchive.body.agent_count != null
        ? statusAfterArchive.body.agent_count
        : 0
    );
    summary.checks.archive_reduces_agent_count = afterArchiveCount <= Math.max(0, afterCreateCount - 1);

    const agentsAfterArchive = await fetchJson(`${BASE_URL}/api/agents`);
    assert.strictEqual(agentsAfterArchive.status, 200, 'agents-after-archive should return 200');
    const agentRows = Array.isArray(agentsAfterArchive.body) ? agentsAfterArchive.body : [];
    summary.checks.archived_hidden_from_agent_list = !agentRows.some((row) => row && row.id === archiveAgentId);
    summary.checks.archived_removed_from_collab_authority = !authorityAgentShadows().includes(archiveAgentId);
    assert.strictEqual(
      summary.checks.archived_removed_from_collab_authority,
      true,
      'archived agent should be removed from collab authority state'
    );

    const archivedMessage = await fetchJson(`${BASE_URL}/api/agents/${encodeURIComponent(archiveAgentId)}/message`, {
      method: 'POST',
      body: JSON.stringify({ message: 'still there?' }),
    });
    summary.checks.archived_agent_message_blocked = archivedMessage.status === 409
      && archivedMessage.body
      && archivedMessage.body.error === 'agent_inactive';
    assert.strictEqual(summary.checks.archived_agent_message_blocked, true, 'archived agent should reject chat message');

    const archivedGet = await fetchJson(`${BASE_URL}/api/agents/${encodeURIComponent(archiveAgentId)}`);
    assert.strictEqual(archivedGet.status, 200, 'get archived agent should return 200 inactive record');
    summary.checks.archived_agent_state_inactive = !!(
      archivedGet.body
      && archivedGet.body.state === 'inactive'
      && archivedGet.body.archived === true
    );
    summary.evidence.archive = {
      target_agent: archiveAgentId,
