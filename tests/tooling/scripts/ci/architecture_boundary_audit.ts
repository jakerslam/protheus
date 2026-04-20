#!/usr/bin/env node
/* eslint-disable no-console */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cleanText, hasFlag, parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type CheckResult = {
  id: string;
  ok: boolean;
  detail: string;
};

const DEFAULT_OUT_JSON = 'core/local/artifacts/architecture_boundary_audit_current.json';
const DEFAULT_OUT_MD = 'local/workspace/reports/ARCHITECTURE_BOUNDARY_AUDIT_CURRENT.md';

function parseArgs(argv: string[]) {
  return {
    strict: hasFlag(argv, 'strict') || parseBool(readFlag(argv, 'strict'), false),
    outJson: cleanText(readFlag(argv, 'out-json') || DEFAULT_OUT_JSON, 400),
    outMd: cleanText(readFlag(argv, 'out-md') || DEFAULT_OUT_MD, 400),
  };
}

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

function isOrchestrationSurfaceShim(source: string, key: string): boolean {
  const normalizedKey = cleanText(key, 120);
  return (
    normalizedKey.length > 0 &&
    source.includes('adapters/runtime/orchestration_surface_modules.ts') &&
    source.includes(`bindOrchestrationSurfaceModule('${normalizedKey}', module)`)
  );
}

function toMarkdown(rows: CheckResult[]): string {
  const lines: string[] = [];
  lines.push('# Architecture Boundary Audit (Current)');
  lines.push('');
  lines.push(`Generated: ${new Date().toISOString()}`);
  lines.push('');
  lines.push('| Check | Result | Detail |');
  lines.push('| --- | --- | --- |');
  for (const row of rows) {
    lines.push(`| ${row.id} | ${row.ok ? 'pass' : 'fail'} | ${row.detail} |`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

function main() {
  const started = Date.now();
  const args = parseArgs(process.argv.slice(2));

  const nexusPolicy = read('core/layer2/nexus/src/policy.rs');
  const coreLib = read('core/layer2/nexus/src/lib.rs');
  const dashboardIngress = read('core/layer0/ops/src/dashboard_tool_turn_loop.rs');
  const orchestrationLib = read('surface/orchestration/src/lib.rs');
  const orchestrationSeq = read('surface/orchestration/src/sequencing.rs');
  const orchestrationContracts = read('surface/orchestration/src/contracts.rs');
  const orchestrationTransient = read('surface/orchestration/src/transient_context.rs');
  const rootSurfaceContract = JSON.parse(read('client/runtime/config/root_surface_contract.json'));
  const repoSurfacePolicy = JSON.parse(read('client/runtime/config/repo_surface_policy.json'));
  const clientSwarmWrapper = read('client/runtime/systems/autonomy/swarm_orchestration_runtime.ts');
  const surfaceSwarmRuntime = read('surface/orchestration/scripts/swarm_orchestration_runtime.ts');
  const clientSelfImproveWrapper = read('client/runtime/systems/autonomy/self_improvement_cadence_orchestrator.ts');
  const surfaceSelfImproveRuntime = read('surface/orchestration/scripts/self_improvement_cadence_orchestrator.ts');
  const clientRouteTaskWrapper = read('client/runtime/systems/routing/route_task.ts');
  const clientRouteExecuteWrapper = read('client/runtime/systems/routing/route_execute.ts');
  const clientProviderOnboardingWrapper = read('client/runtime/systems/routing/provider_onboarding_manifest.ts');
  const clientGatewayFailureClassifierWrapper = read('client/runtime/systems/routing/llm_gateway_failure_classifier.ts');
  const clientMorphPlannerWrapper = read('client/runtime/systems/fractal/morph_planner.ts');
  const clientValueOfInformationPlannerWrapper = read('client/runtime/systems/sensory/value_information_planner_bridge.ts');
  const clientTaskDecompositionWrapper = read('client/runtime/systems/execution/task_decomposition_primitive.ts');
  const clientLearningConduitWrapper = read('client/runtime/systems/workflow/learning_conduit.ts');
  const clientRelationshipManagerWrapper = read('client/runtime/systems/workflow/client_relationship_manager.ts');
  const clientUniversalOutreachWrapper = read('client/runtime/systems/workflow/universal_outreach_primitive.ts');
  const clientPaymentSkillsWrapper = read('client/runtime/systems/workflow/payment_skills_bridge.ts');
  const clientGatedAccountCreationWrapper = read('client/runtime/systems/workflow/gated_account_creation_organ.ts');
  const clientGatedSelfImprovementWrapper = read('client/runtime/systems/autonomy/gated_self_improvement_loop.ts');
  const clientHoldRemediationWrapper = read('client/runtime/systems/autonomy/hold_remediation_engine.ts');
  const clientLeverExperimentWrapper = read('client/runtime/systems/autonomy/lever_experiment_gate.ts');
  const clientModelCatalogWrapper = read('client/runtime/systems/autonomy/model_catalog_loop.ts');
  const clientProactiveT1Wrapper = read('client/runtime/systems/autonomy/proactive_t1_initiative_engine.ts');
  const clientZeroPermissionWrapper = read('client/runtime/systems/autonomy/zero_permission_conversational_layer.ts');
  const surfaceRouteTaskRuntime = read('surface/orchestration/scripts/route_task.ts');
  const surfaceRouteExecuteRuntime = read('surface/orchestration/scripts/route_execute.ts');
  const surfaceProviderOnboardingRuntime = read('surface/orchestration/scripts/provider_onboarding_manifest.ts');
  const surfaceGatewayFailureClassifierRuntime = read('surface/orchestration/scripts/llm_gateway_failure_classifier.ts');
  const surfaceMorphPlannerRuntime = read('surface/orchestration/scripts/morph_planner.ts');
  const surfaceValueOfInformationPlannerRuntime = read('surface/orchestration/scripts/value_of_information_collection_planner.ts');
  const surfaceTaskDecompositionRuntime = read('surface/orchestration/scripts/task_decomposition_primitive.ts');
  const surfaceLearningConduitRuntime = read('surface/orchestration/scripts/learning_conduit.ts');
  const surfaceRelationshipManagerRuntime = read('surface/orchestration/scripts/client_relationship_manager.ts');
  const surfaceUniversalOutreachRuntime = read('surface/orchestration/scripts/universal_outreach_primitive.ts');
  const surfacePaymentSkillsRuntime = read('surface/orchestration/scripts/payment_skills_bridge.ts');
  const surfaceGatedAccountCreationRuntime = read('surface/orchestration/scripts/gated_account_creation_organ.ts');
  const surfaceGatedSelfImprovementRuntime = read('surface/orchestration/scripts/gated_self_improvement_loop.ts');
  const surfaceHoldRemediationRuntime = read('surface/orchestration/scripts/hold_remediation_engine.ts');
  const surfaceLeverExperimentRuntime = read('surface/orchestration/scripts/lever_experiment_gate.ts');
  const surfaceModelCatalogRuntime = read('surface/orchestration/scripts/model_catalog_loop.ts');
  const surfaceProactiveT1Runtime = read('surface/orchestration/scripts/proactive_t1_initiative_engine.ts');
  const surfaceZeroPermissionRuntime = read('surface/orchestration/scripts/zero_permission_conversational_layer.ts');
  const clientPersonaWrapper = read('client/runtime/systems/personas/orchestration.ts');
  const surfacePersonaRuntime = read('surface/orchestration/scripts/personas_orchestration.ts');

  const checks: CheckResult[] = [
    {
      id: 'core_must_not_depend_on_orchestration_surface',
      ok: !coreLib.includes('infring_orchestration_surface_v1'),
      detail: 'core/layer2/nexus does not import orchestration crate',
    },
    {
      id: 'orchestration_surface_must_not_depend_on_client',
      ok: !orchestrationLib.includes('client::') && !orchestrationLib.includes('client/runtime'),
      detail: 'surface/orchestration has no client-layer dependency',
    },
    {
      id: 'client_ingress_routes_authorized_via_nexus',
      ok: dashboardIngress.includes('authorize_client_ingress_route_with_nexus_inner') &&
        dashboardIngress.includes('client_ingress_nexus_delivery_denied'),
      detail: 'client ingress routes flow through nexus authorization and fail closed on denied delivery',
    },
    {
      id: 'nexus_policy_can_block_source_target_pairs',
      ok: nexusPolicy.includes('block_pair') &&
        nexusPolicy.includes('source_target_pair_blocked'),
      detail: 'nexus policy supports explicit blocked source/target route pairs',
    },
    {
      id: 'orchestration_tool_calls_route_to_tool_broker',
      ok:
        orchestrationSeq.includes('CoreContractCall::ToolBrokerRequest') ||
        orchestrationContracts.includes('ToolBrokerRequest'),
      detail: 'tool-call classification routes through Tool Broker contract',
    },
    {
      id: 'orchestration_transient_state_is_sweepable',
      ok: orchestrationTransient.includes('sweep_expired'),
      detail: 'transient orchestration context supports deterministic sweep',
    },
    {
      id: 'root_surface_contract_allowlists_surface_root',
      ok: Array.isArray(rootSurfaceContract.allowed_root_dirs) &&
        rootSurfaceContract.allowed_root_dirs.includes('surface'),
      detail: 'root surface contract explicitly allowlists surface/',
    },
    {
      id: 'repo_surface_policy_declares_surface_code_root',
      ok: Array.isArray(repoSurfacePolicy.code_roots) && repoSurfacePolicy.code_roots.includes('surface'),
      detail: 'repo surface policy treats surface/ as canonical code root',
    },
    {
      id: 'client_swarm_orchestration_is_wrapper_only',
      ok: clientSwarmWrapper.includes('TypeScript compatibility shim only.') &&
        clientSwarmWrapper.includes('surface/orchestration/scripts/swarm_orchestration_runtime.ts'),
      detail: 'client swarm orchestration entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'orchestration_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfaceSwarmRuntime, 'swarm_orchestration_runtime'),
      detail: 'swarm orchestration coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_self_improvement_orchestration_is_wrapper_only',
      ok: clientSelfImproveWrapper.includes('TypeScript compatibility shim only.') &&
        clientSelfImproveWrapper.includes('surface/orchestration/scripts/self_improvement_cadence_orchestrator.ts'),
      detail: 'client self-improvement orchestration entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'self_improvement_orchestration_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceSelfImproveRuntime,
        'self_improvement_cadence_orchestrator',
      ),
      detail: 'self-improvement cadence orchestration coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_route_task_is_wrapper_only',
      ok: clientRouteTaskWrapper.includes('TypeScript compatibility shim only.') &&
        clientRouteTaskWrapper.includes('surface/orchestration/scripts/route_task.ts'),
      detail: 'client route_task entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'route_task_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfaceRouteTaskRuntime, 'route_task'),
      detail: 'route_task orchestration coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_route_execute_is_wrapper_only',
      ok: clientRouteExecuteWrapper.includes('TypeScript compatibility shim only.') &&
        clientRouteExecuteWrapper.includes('surface/orchestration/scripts/route_execute.ts'),
      detail: 'client route_execute entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'route_execute_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfaceRouteExecuteRuntime, 'route_execute'),
      detail: 'route_execute orchestration coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_provider_onboarding_manifest_is_wrapper_only',
      ok: clientProviderOnboardingWrapper.includes('TypeScript compatibility shim only.') &&
        clientProviderOnboardingWrapper.includes('surface/orchestration/scripts/provider_onboarding_manifest.ts'),
      detail: 'client provider_onboarding_manifest entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'provider_onboarding_manifest_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceProviderOnboardingRuntime,
        'provider_onboarding_manifest',
      ),
      detail: 'provider_onboarding_manifest orchestration coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_gateway_failure_classifier_is_wrapper_only',
      ok: clientGatewayFailureClassifierWrapper.includes('TypeScript compatibility shim only.') &&
        clientGatewayFailureClassifierWrapper.includes('surface/orchestration/scripts/llm_gateway_failure_classifier.ts'),
      detail: 'client llm_gateway_failure_classifier entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'gateway_failure_classifier_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceGatewayFailureClassifierRuntime,
        'llm_gateway_failure_classifier',
      ),
      detail: 'llm_gateway_failure_classifier orchestration coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_morph_planner_is_wrapper_only',
      ok: clientMorphPlannerWrapper.includes('TypeScript compatibility shim only.') &&
        clientMorphPlannerWrapper.includes('surface/orchestration/scripts/morph_planner.ts'),
      detail: 'client morph_planner entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'morph_planner_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfaceMorphPlannerRuntime, 'morph_planner'),
      detail: 'morph_planner coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_value_of_information_planner_is_wrapper_only',
      ok: clientValueOfInformationPlannerWrapper.includes('TypeScript compatibility shim only.') &&
        clientValueOfInformationPlannerWrapper.includes('surface/orchestration/scripts/value_of_information_collection_planner.ts'),
      detail: 'client value_of_information_collection_planner entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'value_of_information_planner_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceValueOfInformationPlannerRuntime,
        'value_of_information_collection_planner',
      ),
      detail: 'value_of_information_collection_planner coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_task_decomposition_is_wrapper_only',
      ok: clientTaskDecompositionWrapper.includes('TypeScript compatibility shim only.') &&
        clientTaskDecompositionWrapper.includes('surface/orchestration/scripts/task_decomposition_primitive.ts'),
      detail: 'client task_decomposition_primitive entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'task_decomposition_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfaceTaskDecompositionRuntime, 'task_decomposition_primitive'),
      detail: 'task_decomposition_primitive coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_learning_conduit_is_wrapper_only',
      ok: clientLearningConduitWrapper.includes('TypeScript compatibility shim only.') &&
        clientLearningConduitWrapper.includes('surface/orchestration/scripts/learning_conduit.ts'),
      detail: 'client learning_conduit entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'learning_conduit_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfaceLearningConduitRuntime, 'learning_conduit'),
      detail: 'learning_conduit coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_relationship_manager_is_wrapper_only',
      ok: clientRelationshipManagerWrapper.includes('TypeScript compatibility shim only.') &&
        clientRelationshipManagerWrapper.includes('surface/orchestration/scripts/client_relationship_manager.ts'),
      detail: 'client client_relationship_manager entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'relationship_manager_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceRelationshipManagerRuntime,
        'client_relationship_manager',
      ),
      detail: 'client_relationship_manager coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_universal_outreach_is_wrapper_only',
      ok: clientUniversalOutreachWrapper.includes('TypeScript compatibility shim only.') &&
        clientUniversalOutreachWrapper.includes('surface/orchestration/scripts/universal_outreach_primitive.ts'),
      detail: 'client universal_outreach_primitive entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'universal_outreach_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceUniversalOutreachRuntime,
        'universal_outreach_primitive',
      ),
      detail: 'universal_outreach_primitive coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_payment_skills_is_wrapper_only',
      ok: clientPaymentSkillsWrapper.includes('TypeScript compatibility shim only.') &&
        clientPaymentSkillsWrapper.includes('surface/orchestration/scripts/payment_skills_bridge.ts'),
      detail: 'client payment_skills_bridge entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'payment_skills_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfacePaymentSkillsRuntime, 'payment_skills_bridge'),
      detail: 'payment_skills_bridge coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_gated_account_creation_is_wrapper_only',
      ok: clientGatedAccountCreationWrapper.includes('TypeScript compatibility shim only.') &&
        clientGatedAccountCreationWrapper.includes('surface/orchestration/scripts/gated_account_creation_organ.ts'),
      detail: 'client gated_account_creation_organ entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'gated_account_creation_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceGatedAccountCreationRuntime,
        'gated_account_creation_organ',
      ),
      detail: 'gated_account_creation_organ coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_gated_self_improvement_is_wrapper_only',
      ok: clientGatedSelfImprovementWrapper.includes('TypeScript compatibility shim only.') &&
        clientGatedSelfImprovementWrapper.includes('surface/orchestration/scripts/gated_self_improvement_loop.ts'),
      detail: 'client gated_self_improvement_loop entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'gated_self_improvement_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceGatedSelfImprovementRuntime,
        'gated_self_improvement_loop',
      ),
      detail: 'gated_self_improvement_loop coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_hold_remediation_is_wrapper_only',
      ok: clientHoldRemediationWrapper.includes('TypeScript compatibility shim only.') &&
        clientHoldRemediationWrapper.includes('surface/orchestration/scripts/hold_remediation_engine.ts'),
      detail: 'client hold_remediation_engine entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'hold_remediation_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfaceHoldRemediationRuntime, 'hold_remediation_engine'),
      detail: 'hold_remediation_engine coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_lever_experiment_is_wrapper_only',
      ok: clientLeverExperimentWrapper.includes('TypeScript compatibility shim only.') &&
        clientLeverExperimentWrapper.includes('surface/orchestration/scripts/lever_experiment_gate.ts'),
      detail: 'client lever_experiment_gate entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'lever_experiment_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfaceLeverExperimentRuntime, 'lever_experiment_gate'),
      detail: 'lever_experiment_gate coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_model_catalog_is_wrapper_only',
      ok: clientModelCatalogWrapper.includes('TypeScript compatibility shim only.') &&
        clientModelCatalogWrapper.includes('surface/orchestration/scripts/model_catalog_loop.ts'),
      detail: 'client model_catalog_loop entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'model_catalog_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfaceModelCatalogRuntime, 'model_catalog_loop'),
      detail: 'model_catalog_loop coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_proactive_t1_is_wrapper_only',
      ok: clientProactiveT1Wrapper.includes('TypeScript compatibility shim only.') &&
        clientProactiveT1Wrapper.includes('surface/orchestration/scripts/proactive_t1_initiative_engine.ts'),
      detail: 'client proactive_t1_initiative_engine entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'proactive_t1_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceProactiveT1Runtime,
        'proactive_t1_initiative_engine',
      ),
      detail: 'proactive_t1_initiative_engine coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_zero_permission_is_wrapper_only',
      ok: clientZeroPermissionWrapper.includes('TypeScript compatibility shim only.') &&
        clientZeroPermissionWrapper.includes('surface/orchestration/scripts/zero_permission_conversational_layer.ts'),
      detail: 'client zero_permission_conversational_layer entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'zero_permission_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(
        surfaceZeroPermissionRuntime,
        'zero_permission_conversational_layer',
      ),
      detail: 'zero_permission_conversational_layer coordination implementation is hosted in surface/orchestration',
    },
    {
      id: 'client_persona_orchestration_is_wrapper_only',
      ok: clientPersonaWrapper.includes('TypeScript compatibility shim only.') &&
        clientPersonaWrapper.includes('surface/orchestration/scripts/personas_orchestration.ts'),
      detail: 'client persona orchestration entrypoint remains thin and delegates to surface/orchestration',
    },
    {
      id: 'persona_orchestration_runtime_lives_under_surface',
      ok: isOrchestrationSurfaceShim(surfacePersonaRuntime, 'personas_orchestration'),
      detail: 'persona orchestration coordination implementation is hosted in surface/orchestration',
    },
  ];

  const failures = checks.filter((row) => !row.ok);
  const payload = {
    ok: failures.length === 0,
    type: 'architecture_boundary_audit',
    generated_at: new Date().toISOString(),
    duration_ms: Date.now() - started,
    owner: 'ops',
    revision: currentRevision(process.cwd()),
    inputs: {
      strict: args.strict,
      out_json: args.outJson,
      out_markdown: args.outMd,
    },
    summary: {
      checks: checks.length,
      failure_count: failures.length,
      pass: failures.length === 0,
    },
    failures: failures.map((row) => ({ id: row.id, detail: row.detail })),
    artifact_paths: [args.outJson, args.outMd],
    checks,
  };

  writeJsonArtifact(resolve(args.outJson), payload);
  writeTextArtifact(resolve(args.outMd), toMarkdown(checks));
  return emitStructuredResult(payload, {
    outPath: '',
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(main());
