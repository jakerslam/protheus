'use strict';

const { bindRuntimeSystemModule } = require('./runtime_system_bridge.ts');
const { bindSwarmOrchestrationRuntimeModule } = require('./swarm_bridge_modules.ts');

const ORCHESTRATION_SURFACE_REGISTRY = Object.freeze({
  adaptive_defense_expansion: Object.freeze({ scriptName: 'adaptive_defense_expansion', systemId: 'SYSTEMS-REDTEAM-ADAPTIVE_DEFENSE_EXPANSION' }),
  client_relationship_manager: Object.freeze({ scriptName: 'client_relationship_manager', systemId: 'SYSTEMS-WORKFLOW-CLIENT_RELATIONSHIP_MANAGER' }),
  economic_entity_manager: Object.freeze({ scriptName: 'economic_entity_manager', systemId: 'SYSTEMS-FINANCE-ECONOMIC_ENTITY_MANAGER' }),
  experiment_scheduler: Object.freeze({ scriptName: 'experiment_scheduler', systemId: 'SYSTEMS-SCIENCE-EXPERIMENT_SCHEDULER' }),
  gated_account_creation_organ: Object.freeze({ scriptName: 'gated_account_creation_organ', systemId: 'SYSTEMS-WORKFLOW-GATED_ACCOUNT_CREATION_ORGAN' }),
  gated_self_improvement_loop: Object.freeze({ scriptName: 'gated_self_improvement_loop', systemId: 'SYSTEMS-AUTONOMY-GATED_SELF_IMPROVEMENT_LOOP' }),
  hold_remediation_engine: Object.freeze({ scriptName: 'hold_remediation_engine', systemId: 'SYSTEMS-AUTONOMY-HOLD_REMEDIATION_ENGINE' }),
  learning_conduit: Object.freeze({ scriptName: 'learning_conduit', systemId: 'SYSTEMS-WORKFLOW-LEARNING_CONDUIT' }),
  lever_experiment_gate: Object.freeze({ scriptName: 'lever_experiment_gate', systemId: 'SYSTEMS-AUTONOMY-LEVER_EXPERIMENT_GATE' }),
  llm_gateway_failure_classifier: Object.freeze({ scriptName: 'llm_gateway_failure_classifier', systemId: 'SYSTEMS-ROUTING-LLM_GATEWAY_FAILURE_CLASSIFIER' }),
  meta_science_active_learning_loop: Object.freeze({ scriptName: 'meta_science_active_learning_loop', systemId: 'SYSTEMS-SCIENCE-META_SCIENCE_ACTIVE_LEARNING_LOOP' }),
  model_catalog_loop: Object.freeze({ scriptName: 'model_catalog_loop', systemId: 'SYSTEMS-AUTONOMY-MODEL_CATALOG_LOOP' }),
  morph_planner: Object.freeze({ scriptName: 'morph_planner', systemId: 'SYSTEMS-FRACTAL-MORPH_PLANNER' }),
  payment_skills_bridge: Object.freeze({ scriptName: 'payment_skills_bridge', systemId: 'SYSTEMS-WORKFLOW-PAYMENT_SKILLS_BRIDGE' }),
  personas_orchestration: Object.freeze({ scriptName: 'orchestration', systemId: 'SYSTEMS-PERSONAS-ORCHESTRATION' }),
  proactive_t1_initiative_engine: Object.freeze({ scriptName: 'proactive_t1_initiative_engine', systemId: 'SYSTEMS-AUTONOMY-PROACTIVE_T1_INITIATIVE_ENGINE' }),
  provider_onboarding_manifest: Object.freeze({ scriptName: 'provider_onboarding_manifest', systemId: 'SYSTEMS-ROUTING-PROVIDER_ONBOARDING_MANIFEST' }),
  quantum_security_primitive_synthesis: Object.freeze({ scriptName: 'quantum_security_primitive_synthesis', systemId: 'SYSTEMS-REDTEAM-QUANTUM_SECURITY_PRIMITIVE_SYNTHESIS' }),
  research_organ: Object.freeze({ scriptName: 'research_organ', systemId: 'SYSTEMS-RESEARCH-RESEARCH_ORGAN' }),
  route_execute: Object.freeze({ scriptName: 'route_execute', systemId: 'SYSTEMS-ROUTING-ROUTE_EXECUTE' }),
  route_task: Object.freeze({ scriptName: 'route_task', systemId: 'SYSTEMS-ROUTING-ROUTE_TASK' }),
  scientific_method_loop: Object.freeze({ scriptName: 'scientific_method_loop', systemId: 'SYSTEMS-SCIENCE-SCIENTIFIC_METHOD_LOOP' }),
  scientific_mode_v4: Object.freeze({ scriptName: 'scientific_mode_v4', systemId: 'SYSTEMS-SCIENCE-SCIENTIFIC_MODE_V4' }),
  self_improvement_cadence_orchestrator: Object.freeze({ scriptName: 'self_improvement_cadence_orchestrator', systemId: 'SYSTEMS-AUTONOMY-SELF_IMPROVEMENT_CADENCE_ORCHESTRATOR' }),
  self_improving_redteam_trainer: Object.freeze({ scriptName: 'self_improving_redteam_trainer', systemId: 'SYSTEMS-REDTEAM-SELF_IMPROVING_REDTEAM_TRAINER' }),
  strategy_learner: Object.freeze({ scriptName: 'strategy_learner', systemId: 'SYSTEMS-STRATEGY-STRATEGY_LEARNER' }),
  swarm_orchestration_runtime: Object.freeze({ kind: 'swarm' }),
  task_decomposition_primitive: Object.freeze({ scriptName: 'task_decomposition_primitive', systemId: 'SYSTEMS-EXECUTION-TASK_DECOMPOSITION_PRIMITIVE' }),
  universal_outreach_primitive: Object.freeze({ scriptName: 'universal_outreach_primitive', systemId: 'SYSTEMS-WORKFLOW-UNIVERSAL_OUTREACH_PRIMITIVE' }),
  value_of_information_collection_planner: Object.freeze({ scriptName: 'value_of_information_collection_planner', systemId: 'SYSTEMS-SENSORY-VALUE_OF_INFORMATION_COLLECTION_PLANNER' }),
  zero_permission_conversational_layer: Object.freeze({ scriptName: 'zero_permission_conversational_layer', systemId: 'SYSTEMS-AUTONOMY-ZERO_PERMISSION_CONVERSATIONAL_LAYER' }),
});

function bindOrchestrationSurfaceModule(moduleKey, currentModule, argv = process.argv.slice(2)) {
  const binding = ORCHESTRATION_SURFACE_REGISTRY[String(moduleKey)];
  if (!binding) {
    throw new Error(`unknown_orchestration_surface_module:${String(moduleKey)}`);
  }
  if (binding.kind === 'swarm') {
    return bindSwarmOrchestrationRuntimeModule(currentModule);
  }
  return bindRuntimeSystemModule(__dirname, binding.scriptName, binding.systemId, currentModule, argv);
}

module.exports = {
  ORCHESTRATION_SURFACE_REGISTRY,
  bindOrchestrationSurfaceModule,
};
