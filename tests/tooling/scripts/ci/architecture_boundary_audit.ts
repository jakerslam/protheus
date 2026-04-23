#!/usr/bin/env node
/* eslint-disable no-console */
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { cleanText, hasFlag, parseBool, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact, writeTextArtifact } from '../../lib/result.ts';

type CheckResult = {
  id: string;
  ok: boolean;
  detail: string;
};

type OrchestrationRegistryBinding = {
  key: string;
  scriptName: string;
  systemId: string;
  kind: string;
};

type OrchestrationShimBinding = {
  key: string;
  scriptFile: string;
  hasBindCall: boolean;
  hasClientRuntimeImport: boolean;
  hasSpawnToken: boolean;
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

function orchestrationRegistryHasKey(source: string, key: string): boolean {
  const normalizedKey = cleanText(key, 120);
  return normalizedKey.length > 0 && source.includes(`${normalizedKey}: Object.freeze(`);
}

function parseOrchestrationRegistryBindings(source: string): OrchestrationRegistryBinding[] {
  const rows: OrchestrationRegistryBinding[] = [];
  const rowPattern = /^\s*([a-z0-9_]+):\s*Object\.freeze\(\{([^}]*)\}\),?\s*$/gim;
  let match: RegExpExecArray | null = rowPattern.exec(source);
  while (match) {
    const key = cleanText(match[1] || '', 160);
    const body = String(match[2] || '');
    const scriptName = cleanText((body.match(/scriptName:\s*'([^']+)'/i)?.[1] || ''), 160);
    const systemId = cleanText((body.match(/systemId:\s*'([^']+)'/i)?.[1] || ''), 200);
    const kind = cleanText((body.match(/kind:\s*'([^']+)'/i)?.[1] || ''), 80);
    if (key.length > 0) {
      rows.push({ key, scriptName, systemId, kind });
    }
    match = rowPattern.exec(source);
  }
  return rows;
}

function parseOrchestrationShimBindings(scriptDir: string): OrchestrationShimBinding[] {
  const rows: OrchestrationShimBinding[] = [];
  for (const entry of readdirSync(scriptDir)) {
    if (!entry.endsWith('.ts')) {
      continue;
    }
    const source = read(join(scriptDir, entry));
    const keyMatch = source.match(/bindOrchestrationSurfaceModule\('([a-z0-9_]+)'/i);
    if (keyMatch && keyMatch[1]) {
      rows.push({
        key: cleanText(keyMatch[1], 160),
        scriptFile: cleanText(entry, 220),
        hasBindCall: source.includes('bindOrchestrationSurfaceModule('),
        hasClientRuntimeImport:
          source.includes("from '../../client/") ||
          source.includes("from \"../../client/") ||
          source.includes('client/runtime/systems/'),
        hasSpawnToken: /\bspawn(?:Sync)?\s*\(/.test(source),
      });
    }
  }
  return rows;
}

function parseOrchestrationShimKeys(scriptDir: string): string[] {
  const keys = new Set<string>();
  for (const row of parseOrchestrationShimBindings(scriptDir)) {
    keys.add(row.key);
  }
  return Array.from(keys).sort();
}

function isCanonicalToken(value: string): boolean {
  const normalized = cleanText(value, 200);
  return /^[a-z0-9_]+$/.test(normalized);
}

function isCanonicalSystemId(value: string): boolean {
  const normalized = cleanText(value, 220);
  return /^SYSTEMS-[A-Z0-9_]+(?:-[A-Z0-9_]+)+$/.test(normalized);
}

function collectOutOfOrderTokenRows(values: string[]): string[] {
  const outOfOrder: string[] = [];
  for (let i = 1; i < values.length; i += 1) {
    const prev = String(values[i - 1] || '');
    const next = String(values[i] || '');
    if (prev.localeCompare(next, 'en') > 0) {
      outOfOrder.push(`${prev}>${next}`);
    }
  }
  return outOfOrder;
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
  const dashboardIngressAuth = read(
    'core/layer0/ops/src/dashboard_tool_turn_loop_parts/030-authorize-client-ingress-route-with-nexus-inner-to-input-confirmed.rs',
  );
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
  const clientValueOfInformationPlannerWrapper = read(
    'client/runtime/systems/sensory/value_of_information_collection_planner.ts',
  );
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
  const orchestrationSurfaceRegistry = read('adapters/runtime/orchestration_surface_modules.ts');
  const orchestrationSurfaceRegistryBindings = parseOrchestrationRegistryBindings(orchestrationSurfaceRegistry);
  const nonSwarmRegistryBindings = orchestrationSurfaceRegistryBindings.filter((row) => row.kind !== 'swarm');
  const swarmRegistryBindings = orchestrationSurfaceRegistryBindings.filter((row) => row.kind === 'swarm');
  const orchestrationSurfaceRegistryBindingsByKey = new Map(
    orchestrationSurfaceRegistryBindings.map((row) => [row.key, row]),
  );
  const auditedOrchestrationModuleKeys = [
    'client_relationship_manager',
    'gated_account_creation_organ',
    'gated_self_improvement_loop',
    'hold_remediation_engine',
    'learning_conduit',
    'lever_experiment_gate',
    'llm_gateway_failure_classifier',
    'model_catalog_loop',
    'morph_planner',
    'payment_skills_bridge',
    'personas_orchestration',
    'proactive_t1_initiative_engine',
    'provider_onboarding_manifest',
    'route_execute',
    'route_task',
    'self_improvement_cadence_orchestrator',
    'swarm_orchestration_runtime',
    'task_decomposition_primitive',
    'universal_outreach_primitive',
    'value_of_information_collection_planner',
    'zero_permission_conversational_layer',
  ];
  const duplicateAuditedOrchestrationModuleKeys = Array.from(
    auditedOrchestrationModuleKeys.reduce((acc, key) => {
      const normalized = cleanText(key, 160);
      if (!normalized) return acc;
      acc.set(normalized, (acc.get(normalized) ?? 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([key, count]) => `${key}:${count}`);
  const outOfOrderAuditedOrchestrationModuleKeys = collectOutOfOrderTokenRows(
    auditedOrchestrationModuleKeys.map((key) => cleanText(key, 160)),
  );
  const auditedOrchestrationModuleKeySet = new Set<string>(
    auditedOrchestrationModuleKeys.map((key) => cleanText(key, 160)),
  );
  const allowedRegistryScriptAliases = new Map<string, string>();
  const missingOrchestrationRegistryKeys = auditedOrchestrationModuleKeys.filter(
    (key) => !orchestrationRegistryHasKey(orchestrationSurfaceRegistry, key),
  );
  const orchestrationShimBindings = parseOrchestrationShimBindings('surface/orchestration/scripts');
  const orchestrationShimKeys = parseOrchestrationShimKeys('surface/orchestration/scripts');
  const duplicateOrchestrationShimKeyBindings = Array.from(
    orchestrationShimBindings.reduce((acc, row) => {
      const scripts = acc.get(row.key) ?? [];
      scripts.push(row.scriptFile);
      acc.set(row.key, scripts);
      return acc;
    }, new Map<string, string[]>()),
  )
    .filter(([, scripts]) => scripts.length > 1)
    .map(([key, scripts]) => `${key}:${scripts.sort().join(',')}`);
  const missingRegistryKeysForShims = orchestrationShimKeys.filter(
    (key) => !orchestrationSurfaceRegistryBindingsByKey.has(key),
  );
  const missingShimKeysForAuditedModules = auditedOrchestrationModuleKeys.filter(
    (key) => !orchestrationShimKeys.includes(key),
  );
  const nonSwarmRegistryKeysOutsideAuditedList = nonSwarmRegistryBindings
    .map((row) => row.key)
    .filter((key) => !auditedOrchestrationModuleKeySet.has(key));
  const shimKeysOutsideAuditedList = orchestrationShimKeys
    .filter((key) => !auditedOrchestrationModuleKeySet.has(key));
  const nonSwarmRegistryBindingCountDelta =
    nonSwarmRegistryBindings.length - auditedOrchestrationModuleKeys.length;
  const shimBindingCountDelta =
    orchestrationShimKeys.length - auditedOrchestrationModuleKeys.length;
  const invalidRegistryBindings = orchestrationSurfaceRegistryBindings
    .filter((row) => row.kind !== 'swarm' && (row.scriptName.length === 0 || row.systemId.length === 0))
    .map((row) => row.key);
  const duplicateRegistryKeys = Array.from(
    orchestrationSurfaceRegistryBindings
      .filter((row) => row.kind !== 'swarm' && row.key.length > 0)
      .reduce((acc, row) => {
        acc.set(row.key, (acc.get(row.key) ?? 0) + 1);
        return acc;
      }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([key, count]) => `${key}:${count}`);
  const invalidRegistryScriptNameFormats = orchestrationSurfaceRegistryBindings
    .filter((row) => row.kind !== 'swarm' && !isCanonicalToken(row.scriptName))
    .map((row) => `${row.key}->${row.scriptName}`);
  const invalidRegistrySystemIdFormats = orchestrationSurfaceRegistryBindings
    .filter((row) => row.kind !== 'swarm' && !isCanonicalSystemId(row.systemId))
    .map((row) => `${row.key}->${row.systemId}`);
  const nonCanonicalRegistrySystemIdMappings = orchestrationSurfaceRegistryBindings
    .filter((row) => {
      if (row.kind === 'swarm' || row.scriptName.length === 0 || row.systemId.length === 0) {
        return false;
      }
      const allowedSuffixAlias = row.key === 'personas_orchestration' ? 'ORCHESTRATION' : null;
      if (allowedSuffixAlias && row.systemId.endsWith(allowedSuffixAlias)) {
        return false;
      }
      return !row.systemId.endsWith(row.scriptName.toUpperCase());
    })
    .map((row) => `${row.key}->${row.systemId}`);
  const allowedOrchestrationSystemIdNamespacePrefixes = new Set<string>([
    'SYSTEMS-ORCHESTRATION-',
    'SYSTEMS-WORKFLOW-',
    'SYSTEMS-FINANCE-',
    'SYSTEMS-SCIENCE-',
    'SYSTEMS-AUTONOMY-',
    'SYSTEMS-ROUTING-',
    'SYSTEMS-FRACTAL-',
    'SYSTEMS-REDTEAM-',
    'SYSTEMS-RESEARCH-',
    'SYSTEMS-STRATEGY-',
    'SYSTEMS-EXECUTION-',
    'SYSTEMS-SENSORY-',
    'SYSTEMS-PERSONAS-',
  ]);
  const invalidRegistrySystemIdNamespaces = orchestrationSurfaceRegistryBindings
    .filter((row) => row.kind !== 'swarm' && row.systemId.length > 0)
    .filter((row) => {
      const prefix = row.systemId.match(/^SYSTEMS-[A-Z0-9_]+-/)?.[0] || '';
      return !allowedOrchestrationSystemIdNamespacePrefixes.has(prefix);
    })
    .map((row) => `${row.key}->${row.systemId}`);
  const nonCanonicalRegistryScriptMappings = orchestrationSurfaceRegistryBindings
    .filter((row) => {
      if (row.kind === 'swarm') {
        return false;
      }
      const alias = allowedRegistryScriptAliases.get(row.key);
      if (alias) {
        return row.scriptName !== alias;
      }
      return row.scriptName !== row.key;
    })
    .map((row) => `${row.key}->${row.scriptName}`);
  const duplicateRegistryScriptNames = Array.from(
    orchestrationSurfaceRegistryBindings
      .filter((row) => row.kind !== 'swarm' && row.scriptName.length > 0)
      .reduce((acc, row) => {
        const keys = acc.get(row.scriptName) ?? [];
        keys.push(row.key);
        acc.set(row.scriptName, keys);
        return acc;
      }, new Map<string, string[]>()),
  )
    .filter(([, keys]) => keys.length > 1)
    .map(([scriptName, keys]) => `${scriptName}:${keys.join(',')}`);
  const duplicateRegistrySystemIds = Array.from(
    orchestrationSurfaceRegistryBindings
      .filter((row) => row.kind !== 'swarm' && row.systemId.length > 0)
      .reduce((acc, row) => {
        const keys = acc.get(row.systemId) ?? [];
        keys.push(row.key);
        acc.set(row.systemId, keys);
        return acc;
      }, new Map<string, string[]>()),
  )
    .filter(([, keys]) => keys.length > 1)
    .map(([systemId, keys]) => `${systemId}:${keys.join(',')}`);
  const outOfOrderRegistryScriptNames = collectOutOfOrderTokenRows(
    nonSwarmRegistryBindings.map((row) => row.scriptName),
  );
  const outOfOrderRegistrySystemIds = collectOutOfOrderTokenRows(
    nonSwarmRegistryBindings.map((row) => row.systemId),
  );
  const outOfOrderOrchestrationShimKeys = collectOutOfOrderTokenRows(
    orchestrationShimBindings.map((row) => row.key),
  );
  const duplicateOrchestrationShimScriptFiles = Array.from(
    orchestrationShimBindings.reduce((acc, row) => {
      const count = acc.get(row.scriptFile) ?? 0;
      acc.set(row.scriptFile, count + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([scriptFile, count]) => `${scriptFile}:${count}`);
  const shimScriptNameMismatches = orchestrationShimBindings
    .filter((row) => orchestrationSurfaceRegistryBindingsByKey.has(row.key))
    .filter((row) => {
      const binding = orchestrationSurfaceRegistryBindingsByKey.get(row.key);
      if (!binding || binding.kind === 'swarm') {
        return false;
      }
      return `${binding.scriptName}.ts` !== row.scriptFile;
    })
    .map((row) => {
      const binding = orchestrationSurfaceRegistryBindingsByKey.get(row.key);
      return `${row.key}:${row.scriptFile}->${binding?.scriptName || 'missing'}`;
    });
  const invalidRegistryKeyFormats = nonSwarmRegistryBindings
    .filter((row) => !isCanonicalToken(row.key))
    .map((row) => `${row.key}`);
  const nonSwarmBindingsWithExplicitKind = nonSwarmRegistryBindings
    .filter((row) => row.kind.length > 0)
    .map((row) => `${row.key}->${row.kind}`);
  const invalidSwarmRegistryBindings = swarmRegistryBindings
    .filter((row) =>
      row.key !== 'swarm_orchestration_runtime'
      || row.scriptName.length > 0
      || row.systemId.length > 0,
    )
    .map((row) => `${row.key}:script=${row.scriptName || 'none'}:system=${row.systemId || 'none'}`);
  const outOfOrderRegistryKeys = collectOutOfOrderTokenRows(nonSwarmRegistryBindings.map((row) => row.key));
  const expectedNamespacePrefixByAuditedKey = new Map<string, string>([
    ['client_relationship_manager', 'SYSTEMS-WORKFLOW-'],
    ['gated_account_creation_organ', 'SYSTEMS-WORKFLOW-'],
    ['gated_self_improvement_loop', 'SYSTEMS-AUTONOMY-'],
    ['hold_remediation_engine', 'SYSTEMS-AUTONOMY-'],
    ['learning_conduit', 'SYSTEMS-WORKFLOW-'],
    ['lever_experiment_gate', 'SYSTEMS-AUTONOMY-'],
    ['llm_gateway_failure_classifier', 'SYSTEMS-ROUTING-'],
    ['model_catalog_loop', 'SYSTEMS-AUTONOMY-'],
    ['morph_planner', 'SYSTEMS-FRACTAL-'],
    ['payment_skills_bridge', 'SYSTEMS-WORKFLOW-'],
    ['personas_orchestration', 'SYSTEMS-PERSONAS-'],
    ['proactive_t1_initiative_engine', 'SYSTEMS-AUTONOMY-'],
    ['provider_onboarding_manifest', 'SYSTEMS-ROUTING-'],
    ['route_execute', 'SYSTEMS-ROUTING-'],
    ['route_task', 'SYSTEMS-ROUTING-'],
    ['self_improvement_cadence_orchestrator', 'SYSTEMS-AUTONOMY-'],
    ['swarm_orchestration_runtime', 'SYSTEMS-AUTONOMY-'],
    ['task_decomposition_primitive', 'SYSTEMS-EXECUTION-'],
    ['universal_outreach_primitive', 'SYSTEMS-WORKFLOW-'],
    ['value_of_information_collection_planner', 'SYSTEMS-SENSORY-'],
    ['zero_permission_conversational_layer', 'SYSTEMS-AUTONOMY-'],
  ]);
  const missingExpectedNamespacePrefixKeys = auditedOrchestrationModuleKeys
    .filter((key) => !expectedNamespacePrefixByAuditedKey.has(key));
  const unknownExpectedNamespacePrefixKeys = Array.from(
    expectedNamespacePrefixByAuditedKey.keys(),
  )
    .filter((key) => !auditedOrchestrationModuleKeySet.has(key));
  const auditedNamespacePrefixMismatches = nonSwarmRegistryBindings
    .filter((row) => expectedNamespacePrefixByAuditedKey.has(row.key))
    .filter((row) => {
      const expectedPrefix = expectedNamespacePrefixByAuditedKey.get(row.key) || '';
      return !row.systemId.startsWith(expectedPrefix);
    })
    .map((row) => `${row.key}->${row.systemId}`);
  const registryScriptBindingRows = orchestrationSurfaceRegistryBindings
    .filter((row) => row.kind !== 'swarm' && row.scriptName.length > 0)
    .map((row) => {
      const scriptPath = join('surface/orchestration/scripts', `${row.scriptName}.ts`);
      const scriptExists = existsSync(scriptPath);
      const keyBindingMatches = scriptExists ? isOrchestrationSurfaceShim(read(scriptPath), row.key) : false;
      return {
        key: row.key,
        scriptPath,
        scriptExists,
        keyBindingMatches,
      };
    });
  const missingRegistryScriptFiles = registryScriptBindingRows
    .filter((row) => !row.scriptExists)
    .map((row) => `${row.key}:${row.scriptPath}`);
  const registryScriptKeyBindingMismatches = registryScriptBindingRows
    .filter((row) => row.scriptExists && !row.keyBindingMatches)
    .map((row) => `${row.key}:${row.scriptPath}`);
  const rootAllowedRootDirs = Array.isArray(rootSurfaceContract.allowed_root_dirs)
    ? rootSurfaceContract.allowed_root_dirs.map((row: unknown) => cleanText(String(row || ''), 240))
    : [];
  const rootAllowedRootDirsSet = new Set<string>(rootAllowedRootDirs);
  const duplicateRootAllowedRootDirs = Array.from(
    rootAllowedRootDirs.reduce((acc, row) => {
      if (!row) return acc;
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`);
  const invalidRootAllowedRootDirs = rootAllowedRootDirs
    .filter((row) => row.length === 0 || row.includes('\\') || row.startsWith('/') || row.includes('..'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const repoCodeRoots = Array.isArray(repoSurfacePolicy.code_roots)
    ? repoSurfacePolicy.code_roots.map((row: unknown) => cleanText(String(row || ''), 240))
    : [];
  const repoCodeRootsSet = new Set<string>(repoCodeRoots);
  const duplicateRepoCodeRoots = Array.from(
    repoCodeRoots.reduce((acc, row) => {
      if (!row) return acc;
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`);
  const invalidRepoCodeRoots = repoCodeRoots
    .filter((row) => row.length === 0 || row.includes('\\') || row.startsWith('/') || row.includes('..'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const missingRootAllowlistEntriesForRepoCodeRoots = repoCodeRoots
    .filter((row) => !rootAllowedRootDirsSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const repoRootRules = repoSurfacePolicy.root_rules && typeof repoSurfacePolicy.root_rules === 'object'
    ? repoSurfacePolicy.root_rules
    : {};
  const repoRootRuleKeys = Object.keys(repoRootRules)
    .map((row) => cleanText(String(row || ''), 240))
    .filter((row) => row.length > 0)
    .sort((a, b) => a.localeCompare(b, 'en'));
  const missingRootRuleKeysForCodeRoots = repoCodeRoots
    .filter((row) => !repoRootRuleKeys.includes(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const unknownRootRuleKeysOutsideCodeRoots = repoRootRuleKeys
    .filter((row) => !repoCodeRootsSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootRuleRowsMissingNotes = repoRootRuleKeys
    .filter((rootKey) => {
      const notes = cleanText(String((repoRootRules as any)[rootKey]?.notes || ''), 400);
      return notes.length === 0;
    })
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootRuleRowsMissingExtensionContract = repoRootRuleKeys
    .filter((rootKey) => {
      const row = (repoRootRules as any)[rootKey] || {};
      const hasAllowed = Array.isArray(row.allowed_extensions);
      const hasTarget = Array.isArray(row.target_extensions);
      return !hasAllowed && !hasTarget;
    })
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootRuleRowsWithEmptyExtensionContract = repoRootRuleKeys
    .filter((rootKey) => {
      const row = (repoRootRules as any)[rootKey] || {};
      const extRows = new Set<string>();
      for (const ext of Array.isArray(row.allowed_extensions) ? row.allowed_extensions : []) {
        extRows.add(cleanText(String(ext || '').toLowerCase(), 80));
      }
      for (const ext of Array.isArray(row.target_extensions) ? row.target_extensions : []) {
        extRows.add(cleanText(String(ext || '').toLowerCase(), 80));
      }
      return extRows.size === 0;
    })
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootRuleRowsWithInvalidExtensionTokens = repoRootRuleKeys
    .flatMap((rootKey) => {
      const row = (repoRootRules as any)[rootKey] || {};
      const extRows = [
        ...(Array.isArray(row.allowed_extensions) ? row.allowed_extensions : []),
        ...(Array.isArray(row.target_extensions) ? row.target_extensions : []),
      ]
        .map((ext) => cleanText(String(ext || '').toLowerCase(), 80))
        .filter((ext) => ext.length > 0);
      return extRows
        .filter((ext) => !/^[a-z0-9]+$/.test(ext))
        .map((ext) => `${rootKey}:${ext}`);
    })
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootRuleRowsWithDuplicateExtensionTokens = repoRootRuleKeys
    .flatMap((rootKey) => {
      const row = (repoRootRules as any)[rootKey] || {};
      const extRows = [
        ...(Array.isArray(row.allowed_extensions) ? row.allowed_extensions : []),
        ...(Array.isArray(row.target_extensions) ? row.target_extensions : []),
      ]
        .map((ext) => cleanText(String(ext || '').toLowerCase(), 80))
        .filter((ext) => ext.length > 0);
      const duplicates = Array.from(
        extRows.reduce((acc, ext) => {
          acc.set(ext, (acc.get(ext) || 0) + 1);
          return acc;
        }, new Map<string, number>()),
      )
        .filter(([, count]) => count > 1)
        .map(([ext, count]) => `${rootKey}:${ext}:${count}`);
      return duplicates;
    })
    .sort((a, b) => a.localeCompare(b, 'en'));
  const deprecatedRootEntries = Array.isArray(rootSurfaceContract.deprecated_root_entries)
    ? rootSurfaceContract.deprecated_root_entries.map((row: unknown) => cleanText(String(row || ''), 240))
    : [];
  const deprecatedRootEntriesSet = new Set<string>(deprecatedRootEntries);
  const deprecatedEntriesOverlappingCodeRoots = repoCodeRoots
    .filter((row) => deprecatedRootEntriesSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const deprecatedEntriesOverlappingAllowedRootDirs = rootAllowedRootDirs
    .filter((row) => deprecatedRootEntriesSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const runtimeExceptions = Array.isArray(repoSurfacePolicy.runtime_exceptions)
    ? repoSurfacePolicy.runtime_exceptions.map((row: unknown) => cleanText(String(row || ''), 240))
    : [];
  const invalidRuntimeExceptions = runtimeExceptions
    .filter((row) => row.length === 0 || row.includes('\\') || row.startsWith('/') || row.includes('..'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const forbiddenPathPrefixes = Array.isArray(repoSurfacePolicy.forbidden_path_prefixes)
    ? repoSurfacePolicy.forbidden_path_prefixes.map((row: unknown) => cleanText(String(row || ''), 240))
    : [];
  const invalidForbiddenPathPrefixes = forbiddenPathPrefixes
    .filter((row) => row.length === 0 || row.includes('\\') || row.startsWith('/') || row.includes('..'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootAllowedRootFiles = Array.isArray(rootSurfaceContract.allowed_root_files)
    ? rootSurfaceContract.allowed_root_files.map((row: unknown) => cleanText(String(row || ''), 240))
    : [];
  const rootAllowedRootFileSet = new Set<string>(rootAllowedRootFiles);
  const ignoreExactPaths = Array.isArray(repoSurfacePolicy.ignore_exact_paths)
    ? repoSurfacePolicy.ignore_exact_paths.map((row: unknown) => cleanText(String(row || ''), 240))
    : [];
  const ignoreExactPathsMissingFromRootAllowedFiles = ignoreExactPaths
    .filter((row) => !rootAllowedRootFileSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const duplicateRootAllowedRootFiles = Array.from(
    rootAllowedRootFiles.reduce((acc, row) => {
      if (!row) return acc;
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`);
  const invalidRootAllowedRootFiles = rootAllowedRootFiles
    .filter((row) => row.length === 0 || row.includes('/') || row.includes('\\') || row.includes('..'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootAllowedRootFilesOverlappingDeprecatedEntries = rootAllowedRootFiles
    .filter((row) => deprecatedRootEntriesSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredCanonicalRootFiles = [
    'ARCHITECTURE.md',
    'Cargo.toml',
    'README.md',
    'install.ps1',
    'install.sh',
    'package.json',
    'verify.sh',
  ];
  const missingRequiredCanonicalRootFiles = requiredCanonicalRootFiles
    .filter((row) => !rootAllowedRootFileSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredCanonicalRootDirs = [
    'adapters',
    'client',
    'core',
    'docs',
    'local',
    'surface',
    'tests',
  ];
  const missingRequiredCanonicalRootDirs = requiredCanonicalRootDirs
    .filter((row) => !rootAllowedRootDirsSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredRepoCodeRoots = ['adapters', 'client', 'core', 'surface', 'tests'];
  const missingRequiredRepoCodeRoots = requiredRepoCodeRoots
    .filter((row) => !repoCodeRootsSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const outOfOrderRootAllowedRootDirs = collectOutOfOrderTokenRows(rootAllowedRootDirs);
  const outOfOrderRootAllowedRootFiles = collectOutOfOrderTokenRows(rootAllowedRootFiles);
  const outOfOrderRepoCodeRoots = collectOutOfOrderTokenRows(repoCodeRoots);
  const outOfOrderRepoRootRuleKeys = collectOutOfOrderTokenRows(repoRootRuleKeys);
  const duplicateRuntimeExceptions = Array.from(
    runtimeExceptions.reduce((acc, row) => {
      if (!row) return acc;
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`);
  const runtimeExceptionsOverlappingCodeRoots = runtimeExceptions
    .map((row) => row.replace(/\/+$/, ''))
    .filter((row) => repoCodeRootsSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const duplicateForbiddenPathPrefixes = Array.from(
    forbiddenPathPrefixes.reduce((acc, row) => {
      if (!row) return acc;
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`);
  const forbiddenPathPrefixesOverlappingCodeRoots = forbiddenPathPrefixes
    .map((row) => row.replace(/\/+$/, ''))
    .filter((row) => repoCodeRootsSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignorePathPrefixes = Array.isArray(repoSurfacePolicy.ignore_path_prefixes)
    ? repoSurfacePolicy.ignore_path_prefixes.map((row: unknown) => cleanText(String(row || ''), 240))
    : [];
  const duplicateIgnorePathPrefixes = Array.from(
    ignorePathPrefixes.reduce((acc, row) => {
      if (!row) return acc;
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`);
  const invalidIgnorePathPrefixes = ignorePathPrefixes
    .filter((row) => row.length === 0 || row.includes('\\') || row.startsWith('/') || row.includes('..'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignorePathPrefixesOverlappingCodeRoots = ignorePathPrefixes
    .map((row) => row.replace(/\/+$/, ''))
    .filter((row) => repoCodeRootsSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const duplicateIgnoreExactPaths = Array.from(
    ignoreExactPaths.reduce((acc, row) => {
      if (!row) return acc;
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`);
  const invalidIgnoreExactPaths = ignoreExactPaths
    .filter((row) => row.length === 0 || row.includes('\\') || row.startsWith('/') || row.includes('..'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootSurfaceContractVersion = cleanText(String(rootSurfaceContract.version || ''), 120);
  const rootSurfaceContractEnabled = rootSurfaceContract.enabled === true;
  const rootSurfaceContractPaths = rootSurfaceContract.paths && typeof rootSurfaceContract.paths === 'object'
    ? rootSurfaceContract.paths
    : {};
  const rootSurfaceLatestPath = cleanText(String((rootSurfaceContractPaths as any).latest_path || ''), 320);
  const rootSurfaceReceiptsPath = cleanText(String((rootSurfaceContractPaths as any).receipts_path || ''), 320);
  const rootSurfacePathRows = [rootSurfaceLatestPath, rootSurfaceReceiptsPath].filter((row) => row.length > 0);
  const duplicateRootSurfacePathRows = Array.from(
    rootSurfacePathRows.reduce((acc, row) => {
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`);
  const invalidRootSurfacePathRows = rootSurfacePathRows
    .filter((row) => row.includes('\\') || row.startsWith('/') || row.includes('..'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const repoSurfacePolicyVersion = cleanText(String(repoSurfacePolicy.version || ''), 120);
  const repoSurfacePolicyVersionIsIsoDate = /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/.test(
    repoSurfacePolicyVersion,
  );
  const outOfOrderRuntimeExceptions = collectOutOfOrderTokenRows(runtimeExceptions);
  const outOfOrderForbiddenPathPrefixes = collectOutOfOrderTokenRows(forbiddenPathPrefixes);
  const outOfOrderIgnorePathPrefixes = collectOutOfOrderTokenRows(ignorePathPrefixes);
  const outOfOrderIgnoreExactPaths = collectOutOfOrderTokenRows(ignoreExactPaths);
  const requiredRuntimeExceptions = [
    'core/local/artifacts/',
    'local/state/',
    'node_modules/',
  ];
  const missingRequiredRuntimeExceptions = requiredRuntimeExceptions
    .filter((row) => !runtimeExceptions.includes(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredForbiddenPathPrefixes = ['client/cli/apps/'];
  const missingRequiredForbiddenPathPrefixes = requiredForbiddenPathPrefixes
    .filter((row) => !forbiddenPathPrefixes.includes(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredIgnoreExactPaths = ['install.ps1', 'install.sh', 'verify.sh'];
  const missingRequiredIgnoreExactPaths = requiredIgnoreExactPaths
    .filter((row) => !ignoreExactPaths.includes(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootRuleAllowedExtensionSetByRoot = new Map<string, Set<string>>();
  const rootRuleTargetExtensionSetByRoot = new Map<string, Set<string>>();
  for (const rootKey of repoRootRuleKeys) {
    const row = (repoRootRules as any)[rootKey] || {};
    const allowedSet = new Set<string>();
    const targetSet = new Set<string>();
    for (const ext of Array.isArray(row.allowed_extensions) ? row.allowed_extensions : []) {
      const normalized = cleanText(String(ext || '').toLowerCase(), 80);
      if (normalized.length > 0) allowedSet.add(normalized);
    }
    for (const ext of Array.isArray(row.target_extensions) ? row.target_extensions : []) {
      const normalized = cleanText(String(ext || '').toLowerCase(), 80);
      if (normalized.length > 0) {
        targetSet.add(normalized);
        allowedSet.add(normalized);
      }
    }
    rootRuleAllowedExtensionSetByRoot.set(rootKey, allowedSet);
    rootRuleTargetExtensionSetByRoot.set(rootKey, targetSet);
  }
  const coreAllowedExtensions = rootRuleAllowedExtensionSetByRoot.get('core') ?? new Set<string>();
  const surfaceTargetExtensions = rootRuleTargetExtensionSetByRoot.get('surface') ?? new Set<string>();
  const clientTargetExtensions = rootRuleTargetExtensionSetByRoot.get('client') ?? new Set<string>();
  const adaptersAllowedExtensions = rootRuleAllowedExtensionSetByRoot.get('adapters') ?? new Set<string>();
  const testsAllowedExtensions = rootRuleAllowedExtensionSetByRoot.get('tests') ?? new Set<string>();
  const packagesAllowedExtensions = rootRuleAllowedExtensionSetByRoot.get('packages') ?? new Set<string>();
  const appsAllowedExtensions = rootRuleAllowedExtensionSetByRoot.get('apps') ?? new Set<string>();
  const rootAllowedRootDirsWithPathSeparators = rootAllowedRootDirs
    .filter((row) => row.includes('/') || row.includes('\\'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const repoCodeRootsWithPathSeparators = repoCodeRoots
    .filter((row) => row.includes('/') || row.includes('\\'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const runtimeExceptionsWithoutTrailingSlash = runtimeExceptions
    .filter((row) => !row.endsWith('/'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const forbiddenPathPrefixesWithoutTrailingSlash = forbiddenPathPrefixes
    .filter((row) => !row.endsWith('/'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignorePathPrefixesWithoutTrailingSlash = ignorePathPrefixes
    .filter((row) => !row.endsWith('/'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignoreExactPathsWithTrailingSlash = ignoreExactPaths
    .filter((row) => row.endsWith('/'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const runtimeExceptionForbiddenPrefixOverlaps = runtimeExceptions
    .flatMap((runtimeRow) =>
      forbiddenPathPrefixes
        .filter(
          (forbiddenRow) =>
            runtimeRow.startsWith(forbiddenRow) || forbiddenRow.startsWith(runtimeRow),
        )
        .map((forbiddenRow) => `${runtimeRow}<->${forbiddenRow}`),
    )
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignorePrefixForbiddenPrefixOverlaps = ignorePathPrefixes
    .flatMap((ignoreRow) =>
      forbiddenPathPrefixes
        .filter(
          (forbiddenRow) => ignoreRow.startsWith(forbiddenRow) || forbiddenRow.startsWith(ignoreRow),
        )
        .map((forbiddenRow) => `${ignoreRow}<->${forbiddenRow}`),
    )
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignoreExactPathsUnderForbiddenPrefixes = ignoreExactPaths
    .filter((row) => forbiddenPathPrefixes.some((prefix) => row.startsWith(prefix)))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignorePrefixRuntimeExceptionOverlaps = ignorePathPrefixes
    .flatMap((ignoreRow) =>
      runtimeExceptions
        .filter((runtimeRow) => ignoreRow.startsWith(runtimeRow) || runtimeRow.startsWith(ignoreRow))
        .map((runtimeRow) => `${ignoreRow}<->${runtimeRow}`),
    )
    .sort((a, b) => a.localeCompare(b, 'en'));
  const coreRuleForbiddenJsTokens = ['js', 'jsx', 'ts', 'tsx'].filter((token) =>
    coreAllowedExtensions.has(token),
  );
  const surfaceTargetExtensionsUnexpectedTokens = Array.from(surfaceTargetExtensions)
    .filter((token) => !new Set(['rs', 'ts', 'tsx']).has(token))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const surfaceAllowedExtensionsMissingTargetTokens = Array.from(surfaceTargetExtensions)
    .filter((token) => !(rootRuleAllowedExtensionSetByRoot.get('surface') ?? new Set<string>()).has(token))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const clientTargetExtensionsMissingWebTokens = ['html', 'css', 'scss'].filter(
    (token) => !clientTargetExtensions.has(token),
  );
  const packagesAllowedExtensionsMissingBaseline = ['ts', 'js'].filter(
    (token) => !packagesAllowedExtensions.has(token),
  );
  const appsAllowedExtensionsMissingBaseline = ['rs', 'ts', 'json', 'yaml', 'yml', 'toml'].filter(
    (token) => !appsAllowedExtensions.has(token),
  );
  const adaptersAllowedExtensionsMissingBaseline = ['rs', 'ts', 'json', 'yaml', 'yml'].filter(
    (token) => !adaptersAllowedExtensions.has(token),
  );
  const testsAllowedExtensionsMissingBaseline = ['sh'].filter(
    (token) => !testsAllowedExtensions.has(token),
  );
  const rootRuleLegacyDebtExtensionsByRoot = new Map<string, string[]>();
  for (const rootKey of repoRootRuleKeys) {
    const row = (repoRootRules as any)[rootKey] || {};
    const legacyRows = Array.isArray(row.legacy_debt_extensions)
      ? row.legacy_debt_extensions
          .map((ext: unknown) => cleanText(String(ext || '').toLowerCase(), 80))
          .filter((ext: string) => ext.length > 0)
      : [];
    if (legacyRows.length > 0) {
      rootRuleLegacyDebtExtensionsByRoot.set(rootKey, legacyRows);
    }
  }
  const legacyDebtInvalidTokens = Array.from(rootRuleLegacyDebtExtensionsByRoot.entries())
    .flatMap(([rootKey, rows]) =>
      rows.filter((token) => !/^[a-z0-9]+$/.test(token)).map((token) => `${rootKey}:${token}`),
    )
    .sort((a, b) => a.localeCompare(b, 'en'));
  const legacyDebtDuplicateTokens = Array.from(rootRuleLegacyDebtExtensionsByRoot.entries())
    .flatMap(([rootKey, rows]) =>
      Array.from(
        rows.reduce((acc, row) => {
          acc.set(row, (acc.get(row) || 0) + 1);
          return acc;
        }, new Map<string, number>()),
      )
        .filter(([, count]) => count > 1)
        .map(([row, count]) => `${rootKey}:${row}:${count}`),
    )
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredLegacyDebtRoots = ['surface', 'client'];
  const missingRequiredLegacyDebtRoots = requiredLegacyDebtRoots
    .filter((rootKey) => !rootRuleLegacyDebtExtensionsByRoot.has(rootKey))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootSurfaceStatePathPrefix = 'client/runtime/local/state/ops/root_surface_contract/';
  const rootSurfaceLatestPathUsesCanonicalPrefix =
    rootSurfaceLatestPath.length > 0 && rootSurfaceLatestPath.startsWith(rootSurfaceStatePathPrefix);
  const rootSurfaceReceiptsPathUsesCanonicalPrefix =
    rootSurfaceReceiptsPath.length > 0 && rootSurfaceReceiptsPath.startsWith(rootSurfaceStatePathPrefix);
  const rootSurfaceLatestPathHasCanonicalExtension = rootSurfaceLatestPath.endsWith('.json');
  const rootSurfaceReceiptsPathHasCanonicalExtension = rootSurfaceReceiptsPath.endsWith('.jsonl');
  const disallowedCodeRoots = ['local', 'docs', 'setup'];
  const disallowedCodeRootsPresent = disallowedCodeRoots
    .filter((row) => repoCodeRootsSet.has(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const requiredRuntimeExceptionRoots = ['local/', 'docs/', 'setup/'];
  const missingRequiredRuntimeExceptionRoots = requiredRuntimeExceptionRoots
    .filter((row) => !runtimeExceptions.includes(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignorePathPrefixesOutsideCodeRoots = ignorePathPrefixes
    .filter((row) => row.length > 0)
    .filter((row) => {
      const firstSegment = row.replace(/\/+$/, '').split('/').filter(Boolean)[0] || '';
      return firstSegment.length > 0 && !repoCodeRootsSet.has(firstSegment);
    })
    .sort((a, b) => a.localeCompare(b, 'en'));
  const forbiddenPrefixesOutsideCodeRoots = forbiddenPathPrefixes
    .filter((row) => row.length > 0)
    .filter((row) => {
      const firstSegment = row.replace(/\/+$/, '').split('/').filter(Boolean)[0] || '';
      return firstSegment.length > 0 && !repoCodeRootsSet.has(firstSegment);
    })
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignoreExactPathsWithNestedSegments = ignoreExactPaths
    .filter((row) => row.includes('/'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const packagesMissingWebExtensionBaseline = ['html', 'css'].filter(
    (token) => !packagesAllowedExtensions.has(token),
  );
  const packagesContainsRustExtension = packagesAllowedExtensions.has('rs');
  const appsMissingPythonShellBaseline = ['py', 'sh'].filter(
    (token) => !appsAllowedExtensions.has(token),
  );
  const adaptersMissingPythonShellBaseline = ['py', 'sh'].filter(
    (token) => !adaptersAllowedExtensions.has(token),
  );
  const testsUnexpectedManifestExtensions = ['json', 'yaml', 'yml', 'toml'].filter(
    (token) => testsAllowedExtensions.has(token),
  );
  const legacyDebtRootsOutsideRootRules = Array.from(rootRuleLegacyDebtExtensionsByRoot.keys())
    .filter((rootKey) => !repoRootRuleKeys.includes(rootKey))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootSurfaceContractVersionIsIsoDate = /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/.test(
    rootSurfaceContractVersion,
  );
  const rootSurfacePathRowsWithWhitespace = rootSurfacePathRows
    .filter((row) => /\s/.test(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootSurfacePathRowsOutsideOpsStatePrefix = rootSurfacePathRows
    .filter((row) => !row.startsWith('client/runtime/local/state/ops/'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootSurfacePathRowsNotLowercase = rootSurfacePathRows
    .filter((row) => row !== row.toLowerCase())
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootSurfaceLatestPathHasCurrentSnapshotSuffix = rootSurfaceLatestPath.endsWith('_current.json');
  const rootSurfaceReceiptsPathHasCurrentSnapshotSuffix = rootSurfaceReceiptsPath.endsWith('_current.jsonl');
  const nonCanonicalRootAllowedRootDirs = rootAllowedRootDirs
    .filter((row) => !isCanonicalToken(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const nonCanonicalRepoCodeRoots = repoCodeRoots
    .filter((row) => !isCanonicalToken(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootAllowedRootFilesWithWhitespace = rootAllowedRootFiles
    .filter((row) => /\s/.test(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignoreExactPathsWithWhitespace = ignoreExactPaths
    .filter((row) => /\s/.test(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const runtimeExceptionsNotLowercase = runtimeExceptions
    .filter((row) => row !== row.toLowerCase())
    .sort((a, b) => a.localeCompare(b, 'en'));
  const forbiddenPathPrefixesNotLowercase = forbiddenPathPrefixes
    .filter((row) => row !== row.toLowerCase())
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignorePathPrefixesNotLowercase = ignorePathPrefixes
    .filter((row) => row !== row.toLowerCase())
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignoreExactPathsNotLowercase = ignoreExactPaths
    .filter((row) => row !== row.toLowerCase())
    .sort((a, b) => a.localeCompare(b, 'en'));
  const nonCanonicalRepoRootRuleKeys = repoRootRuleKeys
    .filter((row) => !isCanonicalToken(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const nonSwarmRegistryKeysMissingShimBindings = nonSwarmRegistryBindings
    .filter((row) => !orchestrationShimBindings.some((shim) => shim.key === row.key))
    .map((row) => row.key)
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootAllowedRootDirsCountMeetsRequiredBaseline =
    rootAllowedRootDirs.length >= requiredCanonicalRootDirs.length;
  const repoCodeRootsCountMeetsRequiredBaseline = repoCodeRoots.length >= requiredRepoCodeRoots.length;
  const repoRootRuleCountMatchesCodeRoots = repoRootRuleKeys.length === repoCodeRoots.length;
  const runtimeExceptionsWithWhitespace = runtimeExceptions
    .filter((row) => /\s/.test(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const forbiddenPathPrefixesWithWhitespace = forbiddenPathPrefixes
    .filter((row) => /\s/.test(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignorePathPrefixesWithWhitespace = ignorePathPrefixes
    .filter((row) => /\s/.test(row))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignoreExactPathsWithDoubleSlash = ignoreExactPaths
    .filter((row) => row.includes('//'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const runtimeExceptionsWithDoubleSlash = runtimeExceptions
    .filter((row) => row.includes('//'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const forbiddenPathPrefixesWithDoubleSlash = forbiddenPathPrefixes
    .filter((row) => row.includes('//'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const ignorePathPrefixesWithDoubleSlash = ignorePathPrefixes
    .filter((row) => row.includes('//'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootSurfacePathRowsWithDoubleSlash = rootSurfacePathRows
    .filter((row) => row.includes('//'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const rootSurfacePathRowsWithTrailingSlash = rootSurfacePathRows
    .filter((row) => row.endsWith('/'))
    .sort((a, b) => a.localeCompare(b, 'en'));
  const auditedClientWrapperBindings = [
    {
      key: 'client_relationship_manager',
      expectedSurfaceScript: 'surface/orchestration/scripts/client_relationship_manager.ts',
      source: clientRelationshipManagerWrapper,
    },
    {
      key: 'gated_account_creation_organ',
      expectedSurfaceScript: 'surface/orchestration/scripts/gated_account_creation_organ.ts',
      source: clientGatedAccountCreationWrapper,
    },
    {
      key: 'gated_self_improvement_loop',
      expectedSurfaceScript: 'surface/orchestration/scripts/gated_self_improvement_loop.ts',
      source: clientGatedSelfImprovementWrapper,
    },
    {
      key: 'hold_remediation_engine',
      expectedSurfaceScript: 'surface/orchestration/scripts/hold_remediation_engine.ts',
      source: clientHoldRemediationWrapper,
    },
    {
      key: 'learning_conduit',
      expectedSurfaceScript: 'surface/orchestration/scripts/learning_conduit.ts',
      source: clientLearningConduitWrapper,
    },
    {
      key: 'lever_experiment_gate',
      expectedSurfaceScript: 'surface/orchestration/scripts/lever_experiment_gate.ts',
      source: clientLeverExperimentWrapper,
    },
    {
      key: 'llm_gateway_failure_classifier',
      expectedSurfaceScript: 'surface/orchestration/scripts/llm_gateway_failure_classifier.ts',
      source: clientGatewayFailureClassifierWrapper,
    },
    {
      key: 'model_catalog_loop',
      expectedSurfaceScript: 'surface/orchestration/scripts/model_catalog_loop.ts',
      source: clientModelCatalogWrapper,
    },
    {
      key: 'morph_planner',
      expectedSurfaceScript: 'surface/orchestration/scripts/morph_planner.ts',
      source: clientMorphPlannerWrapper,
    },
    {
      key: 'payment_skills_bridge',
      expectedSurfaceScript: 'surface/orchestration/scripts/payment_skills_bridge.ts',
      source: clientPaymentSkillsWrapper,
    },
    {
      key: 'personas_orchestration',
      expectedSurfaceScript: 'surface/orchestration/scripts/personas_orchestration.ts',
      source: clientPersonaWrapper,
    },
    {
      key: 'proactive_t1_initiative_engine',
      expectedSurfaceScript: 'surface/orchestration/scripts/proactive_t1_initiative_engine.ts',
      source: clientProactiveT1Wrapper,
    },
    {
      key: 'provider_onboarding_manifest',
      expectedSurfaceScript: 'surface/orchestration/scripts/provider_onboarding_manifest.ts',
      source: clientProviderOnboardingWrapper,
    },
    {
      key: 'route_execute',
      expectedSurfaceScript: 'surface/orchestration/scripts/route_execute.ts',
      source: clientRouteExecuteWrapper,
    },
    {
      key: 'route_task',
      expectedSurfaceScript: 'surface/orchestration/scripts/route_task.ts',
      source: clientRouteTaskWrapper,
    },
    {
      key: 'self_improvement_cadence_orchestrator',
      expectedSurfaceScript: 'surface/orchestration/scripts/self_improvement_cadence_orchestrator.ts',
      source: clientSelfImproveWrapper,
    },
    {
      key: 'swarm_orchestration_runtime',
      expectedSurfaceScript: 'surface/orchestration/scripts/swarm_orchestration_runtime.ts',
      source: clientSwarmWrapper,
    },
    {
      key: 'task_decomposition_primitive',
      expectedSurfaceScript: 'surface/orchestration/scripts/task_decomposition_primitive.ts',
      source: clientTaskDecompositionWrapper,
    },
    {
      key: 'universal_outreach_primitive',
      expectedSurfaceScript: 'surface/orchestration/scripts/universal_outreach_primitive.ts',
      source: clientUniversalOutreachWrapper,
    },
    {
      key: 'value_of_information_collection_planner',
      expectedSurfaceScript: 'surface/orchestration/scripts/value_of_information_collection_planner.ts',
      source: clientValueOfInformationPlannerWrapper,
    },
    {
      key: 'zero_permission_conversational_layer',
      expectedSurfaceScript: 'surface/orchestration/scripts/zero_permission_conversational_layer.ts',
      source: clientZeroPermissionWrapper,
    },
  ];
  const auditedClientWrapperKeys = new Set<string>(
    auditedClientWrapperBindings.map((row) => row.key),
  );
  const missingClientWrapperKeysForAuditedModules = auditedOrchestrationModuleKeys
    .filter((key) => !auditedClientWrapperKeys.has(key));
  const unknownClientWrapperKeys = Array.from(auditedClientWrapperKeys)
    .filter((key) => !auditedOrchestrationModuleKeySet.has(key));
  const clientWrapperBindingCountDelta =
    auditedClientWrapperBindings.length - auditedOrchestrationModuleKeys.length;
  const clientWrapperRowsMissingCompatibilityMarker = auditedClientWrapperBindings
    .filter((row) => !row.source.includes('TypeScript compatibility shim only.'))
    .map((row) => row.key);
  const clientWrapperRowsMissingExpectedSurfaceDelegation = auditedClientWrapperBindings
    .filter((row) => !row.source.includes(row.expectedSurfaceScript))
    .map((row) => `${row.key}->${row.expectedSurfaceScript}`);
  const clientWrapperRowsWithNonDeterministicDelegationCount = auditedClientWrapperBindings
    .filter((row) => {
      const matches =
        row.source.match(/surface\/orchestration\/scripts\/[a-z0-9_]+\.ts/g) || [];
      return matches.length !== 1;
    })
    .map((row) => {
      const matches =
        row.source.match(/surface\/orchestration\/scripts\/[a-z0-9_]+\.ts/g) || [];
      return `${row.key}:${matches.length}`;
    });
  const clientWrapperRowsWithAuthorityTokens = auditedClientWrapperBindings
    .filter((row) =>
      row.source.includes('authorize_client_ingress_route_with_nexus_inner')
      || row.source.includes('client_ingress_nexus_delivery_denied')
      || row.source.includes('CoreContractCall::')
      || row.source.includes('ToolBrokerRequest'),
    )
    .map((row) => row.key);
  const clientWrapperRowsWithSpawnTokens = auditedClientWrapperBindings
    .filter((row) => /\bspawn(?:Sync)?\s*\(/.test(row.source))
    .map((row) => row.key);
  const auditedClientWrapperKeyRows = auditedClientWrapperBindings.map((row) => row.key);
  const auditedClientWrapperScriptPathRows = auditedClientWrapperBindings.map(
    (row) => row.expectedSurfaceScript,
  );
  const clientWrapperKeysWithNonCanonicalTokenFormat = auditedClientWrapperBindings
    .filter((row) => !isCanonicalToken(row.key))
    .map((row) => row.key)
    .sort((a, b) => a.localeCompare(b, 'en'));
  const clientWrapperExpectedSurfaceScriptsWithInvalidPathFormat = auditedClientWrapperBindings
    .filter((row) => !/^surface\/orchestration\/scripts\/[a-z0-9_]+\.ts$/.test(row.expectedSurfaceScript))
    .map((row) => `${row.key}->${row.expectedSurfaceScript}`)
    .sort((a, b) => a.localeCompare(b, 'en'));
  const clientWrapperExpectedSurfaceScriptKeyMismatches = auditedClientWrapperBindings
    .filter((row) => {
      const fileToken = row.expectedSurfaceScript.split('/').pop()?.replace(/\.ts$/, '') || '';
      return fileToken !== row.key;
    })
    .map((row) => `${row.key}->${row.expectedSurfaceScript}`)
    .sort((a, b) => a.localeCompare(b, 'en'));
  const duplicateClientWrapperExpectedSurfaceScripts = Array.from(
    auditedClientWrapperScriptPathRows.reduce((acc, row) => {
      if (!row) return acc;
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`)
    .sort((a, b) => a.localeCompare(b, 'en'));
  const duplicateClientWrapperKeys = Array.from(
    auditedClientWrapperKeyRows.reduce((acc, row) => {
      if (!row) return acc;
      acc.set(row, (acc.get(row) || 0) + 1);
      return acc;
    }, new Map<string, number>()),
  )
    .filter(([, count]) => count > 1)
    .map(([row, count]) => `${row}:${count}`)
    .sort((a, b) => a.localeCompare(b, 'en'));
  const outOfOrderClientWrapperKeys = collectOutOfOrderTokenRows(auditedClientWrapperKeyRows);
  const outOfOrderClientWrapperExpectedSurfaceScripts = collectOutOfOrderTokenRows(
    auditedClientWrapperScriptPathRows,
  );
  const nonSwarmRegistryBindingCountMatchesScriptBindingRows =
    registryScriptBindingRows.length === nonSwarmRegistryBindings.length;
  const invalidOrchestrationShimKeyFormats = orchestrationShimBindings
    .filter((row) => !isCanonicalToken(row.key))
    .map((row) => row.key);
  const invalidOrchestrationShimScriptFileFormats = orchestrationShimBindings
    .filter((row) => !/^[a-z0-9_]+\.ts$/.test(row.scriptFile))
    .map((row) => row.scriptFile);
  const orchestrationShimScriptFileKeyMismatches = orchestrationShimBindings
    .filter((row) => row.scriptFile !== `${row.key}.ts`)
    .map((row) => `${row.key}->${row.scriptFile}`);
  const orchestrationShimRowsMissingBindCall = orchestrationShimBindings
    .filter((row) => !row.hasBindCall)
    .map((row) => row.scriptFile);
  const orchestrationShimRowsWithClientRuntimeImport = orchestrationShimBindings
    .filter((row) => row.hasClientRuntimeImport)
    .map((row) => row.scriptFile);
  const orchestrationShimRowsWithSpawnTokens = orchestrationShimBindings
    .filter((row) => row.hasSpawnToken)
    .map((row) => row.scriptFile);
  const nonSwarmBindingsMissingScriptName = nonSwarmRegistryBindings
    .filter((row) => row.scriptName.length === 0)
    .map((row) => row.key);
  const nonSwarmBindingsMissingSystemId = nonSwarmRegistryBindings
    .filter((row) => row.systemId.length === 0)
    .map((row) => row.key);
  const nonSwarmBindingsWithNonCanonicalScriptPathTokens = nonSwarmRegistryBindings
    .filter((row) =>
      row.scriptName.includes('/')
      || row.scriptName.includes('\\')
      || row.scriptName.endsWith('.ts'),
    )
    .map((row) => `${row.key}->${row.scriptName}`);
  const expectedNamespacePrefixRows = Array.from(
    expectedNamespacePrefixByAuditedKey.entries(),
  );
  const invalidExpectedNamespacePrefixValues = expectedNamespacePrefixRows
    .filter(([, prefix]) => !/^SYSTEMS-[A-Z0-9_]+-$/.test(prefix))
    .map(([key, prefix]) => `${key}->${prefix}`);
  const disallowedExpectedNamespacePrefixValues = expectedNamespacePrefixRows
    .filter(([, prefix]) => !allowedOrchestrationSystemIdNamespacePrefixes.has(prefix))
    .map(([key, prefix]) => `${key}->${prefix}`);

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
      ok: (
        dashboardIngress.includes('authorize_client_ingress_route_with_nexus_inner') &&
        dashboardIngress.includes('client_ingress_nexus_delivery_denied')
      ) || (
        dashboardIngressAuth.includes('authorize_client_ingress_route_with_nexus_inner') &&
        dashboardIngressAuth.includes('client_ingress_nexus_delivery_denied')
      ),
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
      id: 'root_surface_contract_allowed_root_dirs_present',
      ok: rootAllowedRootDirs.length > 0,
      detail:
        rootAllowedRootDirs.length > 0
          ? `allowed_root_dirs_count=${rootAllowedRootDirs.length}`
          : 'allowed_root_dirs missing or empty',
    },
    {
      id: 'root_surface_contract_allowed_root_dirs_are_unique',
      ok: duplicateRootAllowedRootDirs.length === 0,
      detail:
        duplicateRootAllowedRootDirs.length === 0
          ? 'root surface contract allowed_root_dirs contains no duplicates'
          : `duplicate_allowed_root_dirs=${duplicateRootAllowedRootDirs.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_dirs_use_relative_non_traversal_paths',
      ok: invalidRootAllowedRootDirs.length === 0,
      detail:
        invalidRootAllowedRootDirs.length === 0
          ? 'root surface contract allowed_root_dirs uses relative non-traversal path tokens'
          : `invalid_allowed_root_dirs=${invalidRootAllowedRootDirs.join(',')}`,
    },
    {
      id: 'repo_surface_policy_code_roots_present',
      ok: repoCodeRoots.length > 0,
      detail:
        repoCodeRoots.length > 0
          ? `code_roots_count=${repoCodeRoots.length}`
          : 'repo surface policy code_roots missing or empty',
    },
    {
      id: 'repo_surface_policy_code_roots_are_unique',
      ok: duplicateRepoCodeRoots.length === 0,
      detail:
        duplicateRepoCodeRoots.length === 0
          ? 'repo surface policy code_roots contains no duplicates'
          : `duplicate_code_roots=${duplicateRepoCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_code_roots_use_relative_non_traversal_paths',
      ok: invalidRepoCodeRoots.length === 0,
      detail:
        invalidRepoCodeRoots.length === 0
          ? 'repo surface policy code_roots uses relative non-traversal path tokens'
          : `invalid_code_roots=${invalidRepoCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_code_roots_allowlisted_in_root_surface_contract',
      ok: missingRootAllowlistEntriesForRepoCodeRoots.length === 0,
      detail:
        missingRootAllowlistEntriesForRepoCodeRoots.length === 0
          ? 'every repo surface policy code root is allowlisted by root surface contract'
          : `missing_allowlist_entries_for_code_roots=${missingRootAllowlistEntriesForRepoCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rules_cover_all_code_roots',
      ok: missingRootRuleKeysForCodeRoots.length === 0,
      detail:
        missingRootRuleKeysForCodeRoots.length === 0
          ? 'repo surface policy root_rules provides rule coverage for every code root'
          : `missing_root_rule_keys_for_code_roots=${missingRootRuleKeysForCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rules_have_no_unknown_root_keys',
      ok: unknownRootRuleKeysOutsideCodeRoots.length === 0,
      detail:
        unknownRootRuleKeysOutsideCodeRoots.length === 0
          ? 'repo surface policy root_rules does not declare unknown roots outside code_roots'
          : `unknown_root_rule_keys_outside_code_roots=${unknownRootRuleKeysOutsideCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rules_include_notes',
      ok: rootRuleRowsMissingNotes.length === 0,
      detail:
        rootRuleRowsMissingNotes.length === 0
          ? 'repo surface policy root_rules entries include non-empty notes'
          : `root_rule_rows_missing_notes=${rootRuleRowsMissingNotes.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rules_define_extension_contracts',
      ok: rootRuleRowsMissingExtensionContract.length === 0,
      detail:
        rootRuleRowsMissingExtensionContract.length === 0
          ? 'repo surface policy root_rules entries declare allowed_extensions or target_extensions'
          : `root_rule_rows_missing_extension_contract=${rootRuleRowsMissingExtensionContract.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rules_extension_contracts_nonempty',
      ok: rootRuleRowsWithEmptyExtensionContract.length === 0,
      detail:
        rootRuleRowsWithEmptyExtensionContract.length === 0
          ? 'repo surface policy root_rules extension contracts contain at least one extension token'
          : `root_rule_rows_with_empty_extension_contract=${rootRuleRowsWithEmptyExtensionContract.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rules_extension_tokens_canonical',
      ok: rootRuleRowsWithInvalidExtensionTokens.length === 0,
      detail:
        rootRuleRowsWithInvalidExtensionTokens.length === 0
          ? 'repo surface policy root_rules extension tokens use canonical lowercase alphanumeric format'
          : `root_rule_rows_with_invalid_extension_tokens=${rootRuleRowsWithInvalidExtensionTokens.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rules_extension_tokens_unique_per_root',
      ok: rootRuleRowsWithDuplicateExtensionTokens.length === 0,
      detail:
        rootRuleRowsWithDuplicateExtensionTokens.length === 0
          ? 'repo surface policy root_rules extension contracts are duplicate-free per root'
          : `root_rule_rows_with_duplicate_extension_tokens=${rootRuleRowsWithDuplicateExtensionTokens.join(',')}`,
    },
    {
      id: 'root_surface_contract_deprecated_entries_exclude_repo_code_roots',
      ok: deprecatedEntriesOverlappingCodeRoots.length === 0,
      detail:
        deprecatedEntriesOverlappingCodeRoots.length === 0
          ? 'root surface contract deprecated_root_entries does not overlap repo code_roots'
          : `deprecated_entries_overlapping_code_roots=${deprecatedEntriesOverlappingCodeRoots.join(',')}`,
    },
    {
      id: 'root_surface_contract_deprecated_entries_exclude_allowed_root_dirs',
      ok: deprecatedEntriesOverlappingAllowedRootDirs.length === 0,
      detail:
        deprecatedEntriesOverlappingAllowedRootDirs.length === 0
          ? 'root surface contract deprecated_root_entries does not overlap allowed_root_dirs'
          : `deprecated_entries_overlapping_allowed_root_dirs=${deprecatedEntriesOverlappingAllowedRootDirs.join(',')}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_use_relative_non_traversal_paths',
      ok: invalidRuntimeExceptions.length === 0,
      detail:
        invalidRuntimeExceptions.length === 0
          ? 'repo surface policy runtime_exceptions uses relative non-traversal path entries'
          : `invalid_runtime_exceptions=${invalidRuntimeExceptions.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_path_prefixes_present',
      ok: forbiddenPathPrefixes.length > 0,
      detail:
        forbiddenPathPrefixes.length > 0
          ? `forbidden_path_prefixes_count=${forbiddenPathPrefixes.length}`
          : 'forbidden_path_prefixes missing or empty',
    },
    {
      id: 'repo_surface_policy_forbidden_path_prefixes_use_relative_non_traversal_paths',
      ok: invalidForbiddenPathPrefixes.length === 0,
      detail:
        invalidForbiddenPathPrefixes.length === 0
          ? 'repo surface policy forbidden_path_prefixes uses relative non-traversal path entries'
          : `invalid_forbidden_path_prefixes=${invalidForbiddenPathPrefixes.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_are_allowlisted_root_files',
      ok: ignoreExactPathsMissingFromRootAllowedFiles.length === 0,
      detail:
        ignoreExactPathsMissingFromRootAllowedFiles.length === 0
          ? 'repo surface policy ignore_exact_paths is covered by root surface contract allowed_root_files'
          : `ignore_exact_paths_missing_from_allowed_root_files=${ignoreExactPathsMissingFromRootAllowedFiles.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_files_present',
      ok: rootAllowedRootFiles.length > 0,
      detail:
        rootAllowedRootFiles.length > 0
          ? `allowed_root_files_count=${rootAllowedRootFiles.length}`
          : 'allowed_root_files missing or empty',
    },
    {
      id: 'root_surface_contract_allowed_root_files_are_unique',
      ok: duplicateRootAllowedRootFiles.length === 0,
      detail:
        duplicateRootAllowedRootFiles.length === 0
          ? 'root surface contract allowed_root_files contains no duplicates'
          : `duplicate_allowed_root_files=${duplicateRootAllowedRootFiles.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_files_use_root_filename_tokens',
      ok: invalidRootAllowedRootFiles.length === 0,
      detail:
        invalidRootAllowedRootFiles.length === 0
          ? 'root surface contract allowed_root_files uses canonical root filename tokens'
          : `invalid_allowed_root_files=${invalidRootAllowedRootFiles.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_files_include_required_baseline',
      ok: missingRequiredCanonicalRootFiles.length === 0,
      detail:
        missingRequiredCanonicalRootFiles.length === 0
          ? 'root surface contract allowlists required baseline root files'
          : `missing_required_root_files=${missingRequiredCanonicalRootFiles.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_files_exclude_deprecated_entries',
      ok: rootAllowedRootFilesOverlappingDeprecatedEntries.length === 0,
      detail:
        rootAllowedRootFilesOverlappingDeprecatedEntries.length === 0
          ? 'root surface contract allowed_root_files excludes deprecated root entries'
          : `allowed_root_files_overlapping_deprecated_entries=${rootAllowedRootFilesOverlappingDeprecatedEntries.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_dirs_include_required_baseline',
      ok: missingRequiredCanonicalRootDirs.length === 0,
      detail:
        missingRequiredCanonicalRootDirs.length === 0
          ? 'root surface contract allowlists required canonical root directories'
          : `missing_required_root_dirs=${missingRequiredCanonicalRootDirs.join(',')}`,
    },
    {
      id: 'repo_surface_policy_code_roots_include_required_authority_roots',
      ok: missingRequiredRepoCodeRoots.length === 0,
      detail:
        missingRequiredRepoCodeRoots.length === 0
          ? 'repo surface policy code_roots includes required authority roots'
          : `missing_required_code_roots=${missingRequiredRepoCodeRoots.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_dirs_are_sorted',
      ok: outOfOrderRootAllowedRootDirs.length === 0,
      detail:
        outOfOrderRootAllowedRootDirs.length === 0
          ? 'root surface contract allowed_root_dirs is lexicographically sorted'
          : `out_of_order_allowed_root_dirs=${outOfOrderRootAllowedRootDirs.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_files_are_sorted',
      ok: outOfOrderRootAllowedRootFiles.length === 0,
      detail:
        outOfOrderRootAllowedRootFiles.length === 0
          ? 'root surface contract allowed_root_files is lexicographically sorted'
          : `out_of_order_allowed_root_files=${outOfOrderRootAllowedRootFiles.join(',')}`,
    },
    {
      id: 'repo_surface_policy_code_roots_are_sorted',
      ok: outOfOrderRepoCodeRoots.length === 0,
      detail:
        outOfOrderRepoCodeRoots.length === 0
          ? 'repo surface policy code_roots is lexicographically sorted'
          : `out_of_order_code_roots=${outOfOrderRepoCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_keys_are_sorted',
      ok: outOfOrderRepoRootRuleKeys.length === 0,
      detail:
        outOfOrderRepoRootRuleKeys.length === 0
          ? 'repo surface policy root_rules keys are lexicographically sorted'
          : `out_of_order_root_rule_keys=${outOfOrderRepoRootRuleKeys.join(',')}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_are_unique',
      ok: duplicateRuntimeExceptions.length === 0,
      detail:
        duplicateRuntimeExceptions.length === 0
          ? 'repo surface policy runtime_exceptions contains no duplicates'
          : `duplicate_runtime_exceptions=${duplicateRuntimeExceptions.join(',')}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_exclude_code_root_entries',
      ok: runtimeExceptionsOverlappingCodeRoots.length === 0,
      detail:
        runtimeExceptionsOverlappingCodeRoots.length === 0
          ? 'repo surface policy runtime_exceptions excludes direct code-root entries'
          : `runtime_exceptions_overlapping_code_roots=${runtimeExceptionsOverlappingCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_path_prefixes_are_unique',
      ok: duplicateForbiddenPathPrefixes.length === 0,
      detail:
        duplicateForbiddenPathPrefixes.length === 0
          ? 'repo surface policy forbidden_path_prefixes contains no duplicates'
          : `duplicate_forbidden_path_prefixes=${duplicateForbiddenPathPrefixes.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_path_prefixes_exclude_code_root_entries',
      ok: forbiddenPathPrefixesOverlappingCodeRoots.length === 0,
      detail:
        forbiddenPathPrefixesOverlappingCodeRoots.length === 0
          ? 'repo surface policy forbidden_path_prefixes excludes direct code-root entries'
          : `forbidden_path_prefixes_overlapping_code_roots=${forbiddenPathPrefixesOverlappingCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_path_prefixes_present',
      ok: ignorePathPrefixes.length > 0,
      detail:
        ignorePathPrefixes.length > 0
          ? `ignore_path_prefixes_count=${ignorePathPrefixes.length}`
          : 'ignore_path_prefixes missing or empty',
    },
    {
      id: 'repo_surface_policy_ignore_path_prefixes_are_unique',
      ok: duplicateIgnorePathPrefixes.length === 0,
      detail:
        duplicateIgnorePathPrefixes.length === 0
          ? 'repo surface policy ignore_path_prefixes contains no duplicates'
          : `duplicate_ignore_path_prefixes=${duplicateIgnorePathPrefixes.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_path_prefixes_use_relative_non_traversal_paths',
      ok: invalidIgnorePathPrefixes.length === 0,
      detail:
        invalidIgnorePathPrefixes.length === 0
          ? 'repo surface policy ignore_path_prefixes uses relative non-traversal path entries'
          : `invalid_ignore_path_prefixes=${invalidIgnorePathPrefixes.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_path_prefixes_exclude_code_root_entries',
      ok: ignorePathPrefixesOverlappingCodeRoots.length === 0,
      detail:
        ignorePathPrefixesOverlappingCodeRoots.length === 0
          ? 'repo surface policy ignore_path_prefixes excludes direct code-root entries'
          : `ignore_path_prefixes_overlapping_code_roots=${ignorePathPrefixesOverlappingCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_are_unique_and_canonical',
      ok: duplicateIgnoreExactPaths.length === 0 && invalidIgnoreExactPaths.length === 0,
      detail:
        duplicateIgnoreExactPaths.length === 0 && invalidIgnoreExactPaths.length === 0
          ? 'repo surface policy ignore_exact_paths is duplicate-free and uses canonical relative path entries'
          : `duplicate_ignore_exact_paths=${duplicateIgnoreExactPaths.join(',')};invalid_ignore_exact_paths=${invalidIgnoreExactPaths.join(',')}`,
    },
    {
      id: 'root_surface_contract_version_present',
      ok: rootSurfaceContractVersion.length > 0,
      detail:
        rootSurfaceContractVersion.length > 0
          ? `root_surface_contract_version=${rootSurfaceContractVersion}`
          : 'root surface contract version missing',
    },
    {
      id: 'root_surface_contract_enabled_true',
      ok: rootSurfaceContractEnabled,
      detail: `root_surface_contract_enabled=${rootSurfaceContractEnabled}`,
    },
    {
      id: 'root_surface_contract_paths_latest_and_receipts_present',
      ok: rootSurfaceLatestPath.length > 0 && rootSurfaceReceiptsPath.length > 0,
      detail:
        `latest_path=${rootSurfaceLatestPath || 'missing'};` +
        `receipts_path=${rootSurfaceReceiptsPath || 'missing'}`,
    },
    {
      id: 'root_surface_contract_paths_use_relative_non_traversal_paths',
      ok: invalidRootSurfacePathRows.length === 0,
      detail:
        invalidRootSurfacePathRows.length === 0
          ? 'root surface contract paths use relative non-traversal entries'
          : `invalid_root_surface_contract_paths=${invalidRootSurfacePathRows.join(',')}`,
    },
    {
      id: 'root_surface_contract_paths_are_distinct',
      ok: duplicateRootSurfacePathRows.length === 0,
      detail:
        duplicateRootSurfacePathRows.length === 0
          ? 'root surface contract latest/receipts paths are distinct'
          : `duplicate_root_surface_contract_paths=${duplicateRootSurfacePathRows.join(',')}`,
    },
    {
      id: 'repo_surface_policy_version_present',
      ok: repoSurfacePolicyVersion.length > 0,
      detail:
        repoSurfacePolicyVersion.length > 0
          ? `repo_surface_policy_version=${repoSurfacePolicyVersion}`
          : 'repo surface policy version missing',
    },
    {
      id: 'repo_surface_policy_version_uses_iso_date_format',
      ok: repoSurfacePolicyVersionIsIsoDate,
      detail: `repo_surface_policy_version=${repoSurfacePolicyVersion || 'missing'}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_are_sorted',
      ok: outOfOrderRuntimeExceptions.length === 0,
      detail:
        outOfOrderRuntimeExceptions.length === 0
          ? 'repo surface policy runtime_exceptions is lexicographically sorted'
          : `out_of_order_runtime_exceptions=${outOfOrderRuntimeExceptions.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_path_prefixes_are_sorted',
      ok: outOfOrderForbiddenPathPrefixes.length === 0,
      detail:
        outOfOrderForbiddenPathPrefixes.length === 0
          ? 'repo surface policy forbidden_path_prefixes is lexicographically sorted'
          : `out_of_order_forbidden_path_prefixes=${outOfOrderForbiddenPathPrefixes.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_path_prefixes_are_sorted',
      ok: outOfOrderIgnorePathPrefixes.length === 0,
      detail:
        outOfOrderIgnorePathPrefixes.length === 0
          ? 'repo surface policy ignore_path_prefixes is lexicographically sorted'
          : `out_of_order_ignore_path_prefixes=${outOfOrderIgnorePathPrefixes.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_are_sorted',
      ok: outOfOrderIgnoreExactPaths.length === 0,
      detail:
        outOfOrderIgnoreExactPaths.length === 0
          ? 'repo surface policy ignore_exact_paths is lexicographically sorted'
          : `out_of_order_ignore_exact_paths=${outOfOrderIgnoreExactPaths.join(',')}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_include_required_baseline',
      ok: missingRequiredRuntimeExceptions.length === 0,
      detail:
        missingRequiredRuntimeExceptions.length === 0
          ? 'repo surface policy runtime_exceptions includes required baseline entries'
          : `missing_required_runtime_exceptions=${missingRequiredRuntimeExceptions.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_path_prefixes_include_required_baseline',
      ok: missingRequiredForbiddenPathPrefixes.length === 0,
      detail:
        missingRequiredForbiddenPathPrefixes.length === 0
          ? 'repo surface policy forbidden_path_prefixes includes required baseline entries'
          : `missing_required_forbidden_path_prefixes=${missingRequiredForbiddenPathPrefixes.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_include_required_baseline',
      ok: missingRequiredIgnoreExactPaths.length === 0,
      detail:
        missingRequiredIgnoreExactPaths.length === 0
          ? 'repo surface policy ignore_exact_paths includes required baseline entries'
          : `missing_required_ignore_exact_paths=${missingRequiredIgnoreExactPaths.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_core_extensions_include_rs',
      ok: coreAllowedExtensions.has('rs'),
      detail: `core_allowed_extensions_has_rs=${coreAllowedExtensions.has('rs')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_core_extensions_exclude_typescript',
      ok: !coreAllowedExtensions.has('ts') && !coreAllowedExtensions.has('tsx'),
      detail:
        `core_allowed_extensions_has_ts=${coreAllowedExtensions.has('ts')};` +
        `core_allowed_extensions_has_tsx=${coreAllowedExtensions.has('tsx')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_surface_targets_include_rust_and_typescript',
      ok:
        surfaceTargetExtensions.has('rs') &&
        surfaceTargetExtensions.has('ts') &&
        surfaceTargetExtensions.has('tsx'),
      detail:
        `surface_target_extensions_has_rs=${surfaceTargetExtensions.has('rs')};` +
        `surface_target_extensions_has_ts=${surfaceTargetExtensions.has('ts')};` +
        `surface_target_extensions_has_tsx=${surfaceTargetExtensions.has('tsx')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_client_targets_include_typescript_only',
      ok:
        clientTargetExtensions.has('ts') &&
        clientTargetExtensions.has('tsx') &&
        !clientTargetExtensions.has('rs'),
      detail:
        `client_target_extensions_has_ts=${clientTargetExtensions.has('ts')};` +
        `client_target_extensions_has_tsx=${clientTargetExtensions.has('tsx')};` +
        `client_target_extensions_has_rs=${clientTargetExtensions.has('rs')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_adapters_extensions_include_rust_and_typescript',
      ok: adaptersAllowedExtensions.has('rs') && adaptersAllowedExtensions.has('ts'),
      detail:
        `adapters_allowed_extensions_has_rs=${adaptersAllowedExtensions.has('rs')};` +
        `adapters_allowed_extensions_has_ts=${adaptersAllowedExtensions.has('ts')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_tests_extensions_include_rust_and_typescript',
      ok: testsAllowedExtensions.has('rs') && testsAllowedExtensions.has('ts'),
      detail:
        `tests_allowed_extensions_has_rs=${testsAllowedExtensions.has('rs')};` +
        `tests_allowed_extensions_has_ts=${testsAllowedExtensions.has('ts')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_dirs_are_single_segment_tokens',
      ok: rootAllowedRootDirsWithPathSeparators.length === 0,
      detail:
        rootAllowedRootDirsWithPathSeparators.length === 0
          ? 'root surface contract allowed_root_dirs uses single-segment token entries'
          : `allowed_root_dirs_with_path_separators=${rootAllowedRootDirsWithPathSeparators.join(',')}`,
    },
    {
      id: 'repo_surface_policy_code_roots_are_single_segment_tokens',
      ok: repoCodeRootsWithPathSeparators.length === 0,
      detail:
        repoCodeRootsWithPathSeparators.length === 0
          ? 'repo surface policy code_roots uses single-segment token entries'
          : `code_roots_with_path_separators=${repoCodeRootsWithPathSeparators.join(',')}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_use_directory_prefix_shape',
      ok: runtimeExceptionsWithoutTrailingSlash.length === 0,
      detail:
        runtimeExceptionsWithoutTrailingSlash.length === 0
          ? 'repo surface policy runtime_exceptions entries use directory-prefix shape (trailing slash)'
          : `runtime_exceptions_without_trailing_slash=${runtimeExceptionsWithoutTrailingSlash.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_path_prefixes_use_directory_prefix_shape',
      ok: forbiddenPathPrefixesWithoutTrailingSlash.length === 0,
      detail:
        forbiddenPathPrefixesWithoutTrailingSlash.length === 0
          ? 'repo surface policy forbidden_path_prefixes entries use directory-prefix shape (trailing slash)'
          : `forbidden_path_prefixes_without_trailing_slash=${forbiddenPathPrefixesWithoutTrailingSlash.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_path_prefixes_use_directory_prefix_shape',
      ok: ignorePathPrefixesWithoutTrailingSlash.length === 0,
      detail:
        ignorePathPrefixesWithoutTrailingSlash.length === 0
          ? 'repo surface policy ignore_path_prefixes entries use directory-prefix shape (trailing slash)'
          : `ignore_path_prefixes_without_trailing_slash=${ignorePathPrefixesWithoutTrailingSlash.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_use_file_path_shape',
      ok: ignoreExactPathsWithTrailingSlash.length === 0,
      detail:
        ignoreExactPathsWithTrailingSlash.length === 0
          ? 'repo surface policy ignore_exact_paths entries use file-path shape (no trailing slash)'
          : `ignore_exact_paths_with_trailing_slash=${ignoreExactPathsWithTrailingSlash.join(',')}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_do_not_overlap_forbidden_prefixes',
      ok: runtimeExceptionForbiddenPrefixOverlaps.length === 0,
      detail:
        runtimeExceptionForbiddenPrefixOverlaps.length === 0
          ? 'repo surface policy runtime_exceptions does not overlap forbidden_path_prefixes'
          : `runtime_exception_forbidden_prefix_overlaps=${runtimeExceptionForbiddenPrefixOverlaps.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_prefixes_do_not_overlap_forbidden_prefixes',
      ok: ignorePrefixForbiddenPrefixOverlaps.length === 0,
      detail:
        ignorePrefixForbiddenPrefixOverlaps.length === 0
          ? 'repo surface policy ignore_path_prefixes does not overlap forbidden_path_prefixes'
          : `ignore_prefix_forbidden_prefix_overlaps=${ignorePrefixForbiddenPrefixOverlaps.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_do_not_fall_under_forbidden_prefixes',
      ok: ignoreExactPathsUnderForbiddenPrefixes.length === 0,
      detail:
        ignoreExactPathsUnderForbiddenPrefixes.length === 0
          ? 'repo surface policy ignore_exact_paths entries do not fall under forbidden_path_prefixes'
          : `ignore_exact_paths_under_forbidden_prefixes=${ignoreExactPathsUnderForbiddenPrefixes.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_prefixes_do_not_overlap_runtime_exceptions',
      ok: ignorePrefixRuntimeExceptionOverlaps.length === 0,
      detail:
        ignorePrefixRuntimeExceptionOverlaps.length === 0
          ? 'repo surface policy ignore_path_prefixes does not overlap runtime_exceptions'
          : `ignore_prefix_runtime_exception_overlaps=${ignorePrefixRuntimeExceptionOverlaps.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_core_extensions_exclude_js_and_ts_tokens',
      ok: coreRuleForbiddenJsTokens.length === 0,
      detail:
        coreRuleForbiddenJsTokens.length === 0
          ? 'core root-rule allowed extensions excludes JavaScript/TypeScript tokens'
          : `core_rule_forbidden_js_ts_tokens=${coreRuleForbiddenJsTokens.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_surface_targets_are_canonical',
      ok: surfaceTargetExtensionsUnexpectedTokens.length === 0,
      detail:
        surfaceTargetExtensionsUnexpectedTokens.length === 0
          ? 'surface root-rule target extensions are canonical (rs, ts, tsx)'
          : `surface_target_extension_unexpected_tokens=${surfaceTargetExtensionsUnexpectedTokens.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_surface_allowed_extensions_cover_targets',
      ok: surfaceAllowedExtensionsMissingTargetTokens.length === 0,
      detail:
        surfaceAllowedExtensionsMissingTargetTokens.length === 0
          ? 'surface root-rule allowed extensions covers all target extension tokens'
          : `surface_allowed_extensions_missing_target_tokens=${surfaceAllowedExtensionsMissingTargetTokens.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_client_targets_include_web_tokens',
      ok: clientTargetExtensionsMissingWebTokens.length === 0,
      detail:
        clientTargetExtensionsMissingWebTokens.length === 0
          ? 'client root-rule target extensions includes html/css/scss web tokens'
          : `client_target_extensions_missing_web_tokens=${clientTargetExtensionsMissingWebTokens.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_packages_allowed_extensions_include_baseline',
      ok: packagesAllowedExtensionsMissingBaseline.length === 0,
      detail:
        packagesAllowedExtensionsMissingBaseline.length === 0
          ? 'packages root-rule allowed extensions includes baseline tokens (ts, js)'
          : `packages_allowed_extensions_missing_baseline=${packagesAllowedExtensionsMissingBaseline.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_apps_allowed_extensions_include_baseline',
      ok: appsAllowedExtensionsMissingBaseline.length === 0,
      detail:
        appsAllowedExtensionsMissingBaseline.length === 0
          ? 'apps root-rule allowed extensions includes baseline tokens (rs, ts, json, yaml, yml, toml)'
          : `apps_allowed_extensions_missing_baseline=${appsAllowedExtensionsMissingBaseline.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_adapters_allowed_extensions_include_baseline',
      ok: adaptersAllowedExtensionsMissingBaseline.length === 0,
      detail:
        adaptersAllowedExtensionsMissingBaseline.length === 0
          ? 'adapters root-rule allowed extensions includes baseline tokens (rs, ts, json, yaml, yml)'
          : `adapters_allowed_extensions_missing_baseline=${adaptersAllowedExtensionsMissingBaseline.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_tests_allowed_extensions_include_baseline',
      ok: testsAllowedExtensionsMissingBaseline.length === 0,
      detail:
        testsAllowedExtensionsMissingBaseline.length === 0
          ? 'tests root-rule allowed extensions includes baseline token (sh)'
          : `tests_allowed_extensions_missing_baseline=${testsAllowedExtensionsMissingBaseline.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_legacy_debt_tokens_are_canonical',
      ok: legacyDebtInvalidTokens.length === 0,
      detail:
        legacyDebtInvalidTokens.length === 0
          ? 'root-rule legacy debt extension tokens use canonical lowercase alphanumeric format'
          : `legacy_debt_invalid_tokens=${legacyDebtInvalidTokens.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_legacy_debt_contracts_present_and_duplicate_free',
      ok: legacyDebtDuplicateTokens.length === 0 && missingRequiredLegacyDebtRoots.length === 0,
      detail:
        legacyDebtDuplicateTokens.length === 0 && missingRequiredLegacyDebtRoots.length === 0
          ? 'required legacy debt extension contracts are present and duplicate-free for expected roots'
          : `legacy_debt_duplicate_tokens=${legacyDebtDuplicateTokens.join(',')};missing_required_legacy_debt_roots=${missingRequiredLegacyDebtRoots.join(',')}`,
    },
    {
      id: 'root_surface_contract_latest_path_uses_canonical_state_prefix',
      ok: rootSurfaceLatestPathUsesCanonicalPrefix,
      detail:
        `latest_path=${rootSurfaceLatestPath || 'missing'};` +
        `required_prefix=${rootSurfaceStatePathPrefix};` +
        `uses_prefix=${rootSurfaceLatestPathUsesCanonicalPrefix}`,
    },
    {
      id: 'root_surface_contract_receipts_path_uses_canonical_state_prefix',
      ok: rootSurfaceReceiptsPathUsesCanonicalPrefix,
      detail:
        `receipts_path=${rootSurfaceReceiptsPath || 'missing'};` +
        `required_prefix=${rootSurfaceStatePathPrefix};` +
        `uses_prefix=${rootSurfaceReceiptsPathUsesCanonicalPrefix}`,
    },
    {
      id: 'root_surface_contract_latest_path_has_json_extension',
      ok: rootSurfaceLatestPathHasCanonicalExtension,
      detail:
        `latest_path=${rootSurfaceLatestPath || 'missing'};` +
        `has_json_extension=${rootSurfaceLatestPathHasCanonicalExtension}`,
    },
    {
      id: 'root_surface_contract_receipts_path_has_jsonl_extension',
      ok: rootSurfaceReceiptsPathHasCanonicalExtension,
      detail:
        `receipts_path=${rootSurfaceReceiptsPath || 'missing'};` +
        `has_jsonl_extension=${rootSurfaceReceiptsPathHasCanonicalExtension}`,
    },
    {
      id: 'repo_surface_policy_code_roots_exclude_non_code_operational_roots',
      ok: disallowedCodeRootsPresent.length === 0,
      detail:
        disallowedCodeRootsPresent.length === 0
          ? 'repo surface policy code_roots excludes non-code operational roots (local/docs/setup)'
          : `disallowed_code_roots_present=${disallowedCodeRootsPresent.join(',')}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_include_non_code_operational_roots',
      ok: missingRequiredRuntimeExceptionRoots.length === 0,
      detail:
        missingRequiredRuntimeExceptionRoots.length === 0
          ? 'repo surface policy runtime_exceptions includes local/docs/setup operational roots'
          : `missing_required_runtime_exception_roots=${missingRequiredRuntimeExceptionRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_path_prefixes_are_scoped_to_code_root_domains',
      ok: ignorePathPrefixesOutsideCodeRoots.length === 0,
      detail:
        ignorePathPrefixesOutsideCodeRoots.length === 0
          ? 'repo surface policy ignore_path_prefixes remains scoped to declared code-root domains'
          : `ignore_path_prefixes_outside_code_roots=${ignorePathPrefixesOutsideCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_prefixes_are_scoped_to_code_root_domains',
      ok: forbiddenPrefixesOutsideCodeRoots.length === 0,
      detail:
        forbiddenPrefixesOutsideCodeRoots.length === 0
          ? 'repo surface policy forbidden_path_prefixes remains scoped to declared code-root domains'
          : `forbidden_prefixes_outside_code_roots=${forbiddenPrefixesOutsideCodeRoots.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_remain_root_level',
      ok: ignoreExactPathsWithNestedSegments.length === 0,
      detail:
        ignoreExactPathsWithNestedSegments.length === 0
          ? 'repo surface policy ignore_exact_paths remains root-level file-only contract'
          : `ignore_exact_paths_with_nested_segments=${ignoreExactPathsWithNestedSegments.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_packages_include_web_extension_baseline',
      ok: packagesMissingWebExtensionBaseline.length === 0,
      detail:
        packagesMissingWebExtensionBaseline.length === 0
          ? 'packages root-rule allowed extensions includes web baseline tokens (html, css)'
          : `packages_missing_web_extension_baseline=${packagesMissingWebExtensionBaseline.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_packages_exclude_rust_extension',
      ok: !packagesContainsRustExtension,
      detail: `packages_contains_rust_extension=${packagesContainsRustExtension}`,
    },
    {
      id: 'repo_surface_policy_root_rule_apps_include_python_and_shell_baseline',
      ok: appsMissingPythonShellBaseline.length === 0,
      detail:
        appsMissingPythonShellBaseline.length === 0
          ? 'apps root-rule allowed extensions includes python and shell baseline tokens'
          : `apps_missing_python_shell_baseline=${appsMissingPythonShellBaseline.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_adapters_include_python_and_shell_baseline',
      ok: adaptersMissingPythonShellBaseline.length === 0,
      detail:
        adaptersMissingPythonShellBaseline.length === 0
          ? 'adapters root-rule allowed extensions includes python and shell baseline tokens'
          : `adapters_missing_python_shell_baseline=${adaptersMissingPythonShellBaseline.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_tests_exclude_manifest_extensions',
      ok: testsUnexpectedManifestExtensions.length === 0,
      detail:
        testsUnexpectedManifestExtensions.length === 0
          ? 'tests root-rule allowed extensions excludes manifest/data extensions (json/yaml/yml/toml)'
          : `tests_unexpected_manifest_extensions=${testsUnexpectedManifestExtensions.join(',')}`,
    },
    {
      id: 'repo_surface_policy_legacy_debt_roots_must_map_to_declared_root_rules',
      ok: legacyDebtRootsOutsideRootRules.length === 0,
      detail:
        legacyDebtRootsOutsideRootRules.length === 0
          ? 'legacy debt extension roots map to declared repo root-rule entries'
          : `legacy_debt_roots_outside_root_rules=${legacyDebtRootsOutsideRootRules.join(',')}`,
    },
    {
      id: 'root_surface_contract_state_paths_are_distinct_from_repo_surface_policy_version_token',
      ok:
        rootSurfaceLatestPath !== repoSurfacePolicyVersion &&
        rootSurfaceReceiptsPath !== repoSurfacePolicyVersion,
      detail:
        `latest_path=${rootSurfaceLatestPath || 'missing'};` +
        `receipts_path=${rootSurfaceReceiptsPath || 'missing'};` +
        `repo_surface_policy_version=${repoSurfacePolicyVersion || 'missing'}`,
    },
    {
      id: 'repo_surface_policy_required_runtime_exception_roots_are_unique',
      ok: new Set(requiredRuntimeExceptionRoots).size === requiredRuntimeExceptionRoots.length,
      detail:
        `required_runtime_exception_roots=${requiredRuntimeExceptionRoots.join(',')};` +
        `unique_count=${new Set(requiredRuntimeExceptionRoots).size}`,
    },
    {
      id: 'repo_surface_policy_required_ignore_exact_baseline_is_duplicate_free',
      ok: new Set(requiredIgnoreExactPaths).size === requiredIgnoreExactPaths.length,
      detail:
        `required_ignore_exact_baseline=${requiredIgnoreExactPaths.join(',')};` +
        `unique_count=${new Set(requiredIgnoreExactPaths).size}`,
    },
    {
      id: 'repo_surface_policy_required_forbidden_prefix_baseline_is_duplicate_free',
      ok: new Set(requiredForbiddenPathPrefixes).size === requiredForbiddenPathPrefixes.length,
      detail:
        `required_forbidden_prefix_baseline=${requiredForbiddenPathPrefixes.join(',')};` +
        `unique_count=${new Set(requiredForbiddenPathPrefixes).size}`,
    },
    {
      id: 'root_surface_contract_state_path_contract_count_exact',
      ok: rootSurfacePathRows.length === 2,
      detail:
        `state_path_rows=${rootSurfacePathRows.length};` +
        `latest_path_present=${rootSurfaceLatestPath.length > 0};` +
        `receipts_path_present=${rootSurfaceReceiptsPath.length > 0}`,
    },
    {
      id: 'root_surface_contract_version_uses_iso_date_format',
      ok: rootSurfaceContractVersionIsIsoDate,
      detail: `root_surface_contract_version=${rootSurfaceContractVersion || 'missing'}`,
    },
    {
      id: 'root_surface_contract_state_paths_have_no_whitespace',
      ok: rootSurfacePathRowsWithWhitespace.length === 0,
      detail:
        rootSurfacePathRowsWithWhitespace.length === 0
          ? 'root surface contract latest/receipts paths do not contain whitespace'
          : `root_surface_state_paths_with_whitespace=${rootSurfacePathRowsWithWhitespace.join(',')}`,
    },
    {
      id: 'root_surface_contract_state_paths_use_ops_state_prefix',
      ok: rootSurfacePathRowsOutsideOpsStatePrefix.length === 0,
      detail:
        rootSurfacePathRowsOutsideOpsStatePrefix.length === 0
          ? 'root surface contract latest/receipts paths stay under client/runtime/local/state/ops/'
          : `root_surface_paths_outside_ops_state_prefix=${rootSurfacePathRowsOutsideOpsStatePrefix.join(',')}`,
    },
    {
      id: 'root_surface_contract_state_paths_are_lowercase',
      ok: rootSurfacePathRowsNotLowercase.length === 0,
      detail:
        rootSurfacePathRowsNotLowercase.length === 0
          ? 'root surface contract latest/receipts paths use lowercase canonical path tokens'
          : `root_surface_paths_not_lowercase=${rootSurfacePathRowsNotLowercase.join(',')}`,
    },
    {
      id: 'root_surface_contract_latest_path_uses_current_snapshot_suffix',
      ok: rootSurfaceLatestPathHasCurrentSnapshotSuffix,
      detail:
        `latest_path=${rootSurfaceLatestPath || 'missing'};` +
        `has_current_snapshot_suffix=${rootSurfaceLatestPathHasCurrentSnapshotSuffix}`,
    },
    {
      id: 'root_surface_contract_receipts_path_uses_current_snapshot_suffix',
      ok: rootSurfaceReceiptsPathHasCurrentSnapshotSuffix,
      detail:
        `receipts_path=${rootSurfaceReceiptsPath || 'missing'};` +
        `has_current_snapshot_suffix=${rootSurfaceReceiptsPathHasCurrentSnapshotSuffix}`,
    },
    {
      id: 'root_surface_contract_allowed_root_dirs_use_canonical_token_format',
      ok: nonCanonicalRootAllowedRootDirs.length === 0,
      detail:
        nonCanonicalRootAllowedRootDirs.length === 0
          ? 'root surface contract allowed_root_dirs uses canonical [a-z0-9_]+ tokens'
          : `non_canonical_allowed_root_dirs=${nonCanonicalRootAllowedRootDirs.join(',')}`,
    },
    {
      id: 'repo_surface_policy_code_roots_use_canonical_token_format',
      ok: nonCanonicalRepoCodeRoots.length === 0,
      detail:
        nonCanonicalRepoCodeRoots.length === 0
          ? 'repo surface policy code_roots uses canonical [a-z0-9_]+ tokens'
          : `non_canonical_code_roots=${nonCanonicalRepoCodeRoots.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_files_have_no_whitespace',
      ok: rootAllowedRootFilesWithWhitespace.length === 0,
      detail:
        rootAllowedRootFilesWithWhitespace.length === 0
          ? 'root surface contract allowed_root_files entries do not contain whitespace'
          : `allowed_root_files_with_whitespace=${rootAllowedRootFilesWithWhitespace.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_have_no_whitespace',
      ok: ignoreExactPathsWithWhitespace.length === 0,
      detail:
        ignoreExactPathsWithWhitespace.length === 0
          ? 'repo surface policy ignore_exact_paths entries do not contain whitespace'
          : `ignore_exact_paths_with_whitespace=${ignoreExactPathsWithWhitespace.join(',')}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_are_lowercase',
      ok: runtimeExceptionsNotLowercase.length === 0,
      detail:
        runtimeExceptionsNotLowercase.length === 0
          ? 'repo surface policy runtime_exceptions uses lowercase canonical path tokens'
          : `runtime_exceptions_not_lowercase=${runtimeExceptionsNotLowercase.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_path_prefixes_are_lowercase',
      ok: forbiddenPathPrefixesNotLowercase.length === 0,
      detail:
        forbiddenPathPrefixesNotLowercase.length === 0
          ? 'repo surface policy forbidden_path_prefixes uses lowercase canonical path tokens'
          : `forbidden_path_prefixes_not_lowercase=${forbiddenPathPrefixesNotLowercase.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_path_prefixes_are_lowercase',
      ok: ignorePathPrefixesNotLowercase.length === 0,
      detail:
        ignorePathPrefixesNotLowercase.length === 0
          ? 'repo surface policy ignore_path_prefixes uses lowercase canonical path tokens'
          : `ignore_path_prefixes_not_lowercase=${ignorePathPrefixesNotLowercase.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_are_lowercase',
      ok: ignoreExactPathsNotLowercase.length === 0,
      detail:
        ignoreExactPathsNotLowercase.length === 0
          ? 'repo surface policy ignore_exact_paths uses lowercase canonical filename tokens'
          : `ignore_exact_paths_not_lowercase=${ignoreExactPathsNotLowercase.join(',')}`,
    },
    {
      id: 'repo_surface_policy_root_rule_keys_use_canonical_token_format',
      ok: nonCanonicalRepoRootRuleKeys.length === 0,
      detail:
        nonCanonicalRepoRootRuleKeys.length === 0
          ? 'repo surface policy root_rules keys use canonical [a-z0-9_]+ tokens'
          : `non_canonical_root_rule_keys=${nonCanonicalRepoRootRuleKeys.join(',')}`,
    },
    {
      id: 'orchestration_registry_non_swarm_entries_present',
      ok: nonSwarmRegistryBindings.length > 0,
      detail:
        nonSwarmRegistryBindings.length > 0
          ? `non_swarm_registry_binding_count=${nonSwarmRegistryBindings.length}`
          : 'non-swarm orchestration registry bindings missing',
    },
    {
      id: 'orchestration_surface_shim_bindings_present',
      ok: orchestrationShimBindings.length > 0,
      detail:
        orchestrationShimBindings.length > 0
          ? `orchestration_shim_binding_count=${orchestrationShimBindings.length}`
          : 'surface/orchestration shim bindings missing',
    },
    {
      id: 'orchestration_audited_module_keys_present',
      ok: auditedOrchestrationModuleKeys.length > 0,
      detail:
        auditedOrchestrationModuleKeys.length > 0
          ? `audited_orchestration_module_count=${auditedOrchestrationModuleKeys.length}`
          : 'audited orchestration module keyset missing',
    },
    {
      id: 'orchestration_client_wrapper_bindings_present',
      ok: auditedClientWrapperBindings.length > 0,
      detail:
        auditedClientWrapperBindings.length > 0
          ? `client_wrapper_binding_count=${auditedClientWrapperBindings.length}`
          : 'client compatibility wrapper bindings missing',
    },
    {
      id: 'orchestration_registry_non_swarm_keys_backed_by_surface_shims',
      ok: nonSwarmRegistryKeysMissingShimBindings.length === 0,
      detail:
        nonSwarmRegistryKeysMissingShimBindings.length === 0
          ? 'every non-swarm orchestration registry key has a corresponding surface shim key binding'
          : `non_swarm_registry_keys_missing_shim_bindings=${nonSwarmRegistryKeysMissingShimBindings.join(',')}`,
    },
    {
      id: 'root_surface_contract_allowed_root_dirs_count_meets_required_baseline',
      ok: rootAllowedRootDirsCountMeetsRequiredBaseline,
      detail:
        `allowed_root_dirs_count=${rootAllowedRootDirs.length};` +
        `required_minimum=${requiredCanonicalRootDirs.length}`,
    },
    {
      id: 'repo_surface_policy_code_roots_count_meets_required_baseline',
      ok: repoCodeRootsCountMeetsRequiredBaseline,
      detail:
        `code_roots_count=${repoCodeRoots.length};` +
        `required_minimum=${requiredRepoCodeRoots.length}`,
    },
    {
      id: 'repo_surface_policy_root_rule_count_matches_code_roots',
      ok: repoRootRuleCountMatchesCodeRoots,
      detail:
        `root_rule_count=${repoRootRuleKeys.length};` +
        `code_roots_count=${repoCodeRoots.length}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_have_no_whitespace',
      ok: runtimeExceptionsWithWhitespace.length === 0,
      detail:
        runtimeExceptionsWithWhitespace.length === 0
          ? 'repo surface policy runtime_exceptions entries do not contain whitespace'
          : `runtime_exceptions_with_whitespace=${runtimeExceptionsWithWhitespace.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_prefixes_have_no_whitespace',
      ok: forbiddenPathPrefixesWithWhitespace.length === 0,
      detail:
        forbiddenPathPrefixesWithWhitespace.length === 0
          ? 'repo surface policy forbidden_path_prefixes entries do not contain whitespace'
          : `forbidden_prefixes_with_whitespace=${forbiddenPathPrefixesWithWhitespace.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_prefixes_have_no_whitespace',
      ok: ignorePathPrefixesWithWhitespace.length === 0,
      detail:
        ignorePathPrefixesWithWhitespace.length === 0
          ? 'repo surface policy ignore_path_prefixes entries do not contain whitespace'
          : `ignore_prefixes_with_whitespace=${ignorePathPrefixesWithWhitespace.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_exact_paths_have_no_double_slash',
      ok: ignoreExactPathsWithDoubleSlash.length === 0,
      detail:
        ignoreExactPathsWithDoubleSlash.length === 0
          ? 'repo surface policy ignore_exact_paths entries avoid double-slash tokens'
          : `ignore_exact_paths_with_double_slash=${ignoreExactPathsWithDoubleSlash.join(',')}`,
    },
    {
      id: 'repo_surface_policy_runtime_exceptions_have_no_double_slash',
      ok: runtimeExceptionsWithDoubleSlash.length === 0,
      detail:
        runtimeExceptionsWithDoubleSlash.length === 0
          ? 'repo surface policy runtime_exceptions entries avoid double-slash tokens'
          : `runtime_exceptions_with_double_slash=${runtimeExceptionsWithDoubleSlash.join(',')}`,
    },
    {
      id: 'repo_surface_policy_forbidden_prefixes_have_no_double_slash',
      ok: forbiddenPathPrefixesWithDoubleSlash.length === 0,
      detail:
        forbiddenPathPrefixesWithDoubleSlash.length === 0
          ? 'repo surface policy forbidden_path_prefixes entries avoid double-slash tokens'
          : `forbidden_prefixes_with_double_slash=${forbiddenPathPrefixesWithDoubleSlash.join(',')}`,
    },
    {
      id: 'repo_surface_policy_ignore_prefixes_have_no_double_slash',
      ok: ignorePathPrefixesWithDoubleSlash.length === 0,
      detail:
        ignorePathPrefixesWithDoubleSlash.length === 0
          ? 'repo surface policy ignore_path_prefixes entries avoid double-slash tokens'
          : `ignore_prefixes_with_double_slash=${ignorePathPrefixesWithDoubleSlash.join(',')}`,
    },
    {
      id: 'root_surface_contract_state_paths_have_no_double_slash',
      ok: rootSurfacePathRowsWithDoubleSlash.length === 0,
      detail:
        rootSurfacePathRowsWithDoubleSlash.length === 0
          ? 'root surface contract latest/receipts state paths avoid double-slash tokens'
          : `root_surface_paths_with_double_slash=${rootSurfacePathRowsWithDoubleSlash.join(',')}`,
    },
    {
      id: 'root_surface_contract_state_paths_do_not_end_with_slash',
      ok: rootSurfacePathRowsWithTrailingSlash.length === 0,
      detail:
        rootSurfacePathRowsWithTrailingSlash.length === 0
          ? 'root surface contract latest/receipts state paths do not end with trailing slash'
          : `root_surface_paths_with_trailing_slash=${rootSurfacePathRowsWithTrailingSlash.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_keys_use_canonical_token_format',
      ok: clientWrapperKeysWithNonCanonicalTokenFormat.length === 0,
      detail:
        clientWrapperKeysWithNonCanonicalTokenFormat.length === 0
          ? 'client compatibility wrapper keys use canonical [a-z0-9_]+ tokens'
          : `client_wrapper_keys_with_noncanonical_token_format=${clientWrapperKeysWithNonCanonicalTokenFormat.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_expected_surface_paths_use_canonical_format',
      ok: clientWrapperExpectedSurfaceScriptsWithInvalidPathFormat.length === 0,
      detail:
        clientWrapperExpectedSurfaceScriptsWithInvalidPathFormat.length === 0
          ? 'client wrapper expected surface paths use canonical surface/orchestration/scripts/<token>.ts format'
          : `client_wrapper_expected_surface_paths_with_invalid_format=${clientWrapperExpectedSurfaceScriptsWithInvalidPathFormat.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_expected_surface_script_names_align_with_keys',
      ok: clientWrapperExpectedSurfaceScriptKeyMismatches.length === 0,
      detail:
        clientWrapperExpectedSurfaceScriptKeyMismatches.length === 0
          ? 'client wrapper expected surface script filename token aligns with wrapper key'
          : `client_wrapper_expected_surface_script_key_mismatches=${clientWrapperExpectedSurfaceScriptKeyMismatches.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_expected_surface_paths_are_unique',
      ok: duplicateClientWrapperExpectedSurfaceScripts.length === 0,
      detail:
        duplicateClientWrapperExpectedSurfaceScripts.length === 0
          ? 'client wrapper expected surface script paths are unique'
          : `duplicate_client_wrapper_expected_surface_paths=${duplicateClientWrapperExpectedSurfaceScripts.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_keys_are_unique',
      ok: duplicateClientWrapperKeys.length === 0,
      detail:
        duplicateClientWrapperKeys.length === 0
          ? 'client wrapper key map is duplicate-free'
          : `duplicate_client_wrapper_keys=${duplicateClientWrapperKeys.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_keys_are_sorted',
      ok: outOfOrderClientWrapperKeys.length === 0,
      detail:
        outOfOrderClientWrapperKeys.length === 0
          ? 'client wrapper keys are lexicographically sorted for deterministic review diffs'
          : `out_of_order_client_wrapper_keys=${outOfOrderClientWrapperKeys.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_expected_surface_paths_are_sorted',
      ok: outOfOrderClientWrapperExpectedSurfaceScripts.length === 0,
      detail:
        outOfOrderClientWrapperExpectedSurfaceScripts.length === 0
          ? 'client wrapper expected surface script paths are lexicographically sorted'
          : `out_of_order_client_wrapper_expected_surface_paths=${outOfOrderClientWrapperExpectedSurfaceScripts.join(',')}`,
    },
    {
      id: 'orchestration_registry_non_swarm_binding_count_matches_script_binding_rows',
      ok: nonSwarmRegistryBindingCountMatchesScriptBindingRows,
      detail:
        `non_swarm_registry_binding_count=${nonSwarmRegistryBindings.length};` +
        `registry_script_binding_rows=${registryScriptBindingRows.length}`,
    },
    {
      id: 'orchestration_registry_script_names_use_canonical_token_format',
      ok: invalidRegistryScriptNameFormats.length === 0,
      detail: invalidRegistryScriptNameFormats.length === 0
        ? 'non-swarm orchestration registry scriptName values use canonical [a-z0-9_]+ tokens'
        : `invalid scriptName tokens: ${invalidRegistryScriptNameFormats.join(', ')}`,
    },
    {
      id: 'orchestration_registry_keys_use_canonical_token_format',
      ok: invalidRegistryKeyFormats.length === 0,
      detail: invalidRegistryKeyFormats.length === 0
        ? 'non-swarm orchestration registry keys use canonical [a-z0-9_]+ tokens'
        : `invalid key tokens: ${invalidRegistryKeyFormats.join(', ')}`,
    },
    {
      id: 'orchestration_registry_keys_are_unique',
      ok: duplicateRegistryKeys.length === 0,
      detail: duplicateRegistryKeys.length === 0
        ? 'non-swarm orchestration registry keys are unique (no duplicate key shadows)'
        : `duplicate registry keys: ${duplicateRegistryKeys.join(', ')}`,
    },
    {
      id: 'orchestration_registry_non_swarm_bindings_omit_kind_field',
      ok: nonSwarmBindingsWithExplicitKind.length === 0,
      detail: nonSwarmBindingsWithExplicitKind.length === 0
        ? 'non-swarm orchestration registry bindings omit kind to preserve canonical runtime-system bridge shape'
        : `non-swarm bindings with explicit kind=${nonSwarmBindingsWithExplicitKind.join(',')}`,
    },
    {
      id: 'orchestration_registry_script_names_are_unique',
      ok: duplicateRegistryScriptNames.length === 0,
      detail: duplicateRegistryScriptNames.length === 0
        ? 'non-swarm orchestration registry scriptName values are unique'
        : `duplicate_registry_script_names=${duplicateRegistryScriptNames.join(',')}`,
    },
    {
      id: 'orchestration_registry_system_ids_are_unique',
      ok: duplicateRegistrySystemIds.length === 0,
      detail: duplicateRegistrySystemIds.length === 0
        ? 'non-swarm orchestration registry systemId values are unique'
        : `duplicate_registry_system_ids=${duplicateRegistrySystemIds.join(',')}`,
    },
    {
      id: 'orchestration_registry_system_ids_use_canonical_token_format',
      ok: invalidRegistrySystemIdFormats.length === 0,
      detail: invalidRegistrySystemIdFormats.length === 0
        ? 'non-swarm orchestration registry systemId values use canonical SYSTEMS-<DOMAIN>-<NAME> token format'
        : `invalid systemId tokens: ${invalidRegistrySystemIdFormats.join(', ')}`,
    },
    {
      id: 'orchestration_registry_non_swarm_keys_are_sorted',
      ok: outOfOrderRegistryKeys.length === 0,
      detail: outOfOrderRegistryKeys.length === 0
        ? 'non-swarm orchestration registry keys are lexicographically sorted to reduce review drift'
        : `out_of_order_pairs=${outOfOrderRegistryKeys.join(',')}`,
    },
    {
      id: 'orchestration_registry_script_names_are_sorted',
      ok: outOfOrderRegistryScriptNames.length === 0,
      detail: outOfOrderRegistryScriptNames.length === 0
        ? 'non-swarm orchestration registry scriptName values are lexicographically sorted'
        : `out_of_order_script_name_pairs=${outOfOrderRegistryScriptNames.join(',')}`,
    },
    {
      id: 'orchestration_registry_system_ids_are_sorted',
      ok: outOfOrderRegistrySystemIds.length === 0,
      detail: outOfOrderRegistrySystemIds.length === 0
        ? 'non-swarm orchestration registry systemId values are lexicographically sorted'
        : `out_of_order_system_id_pairs=${outOfOrderRegistrySystemIds.join(',')}`,
    },
    {
      id: 'orchestration_registry_system_ids_align_with_script_name',
      ok: nonCanonicalRegistrySystemIdMappings.length === 0,
      detail: nonCanonicalRegistrySystemIdMappings.length === 0
        ? 'non-swarm orchestration registry systemId values end with uppercased scriptName token'
        : `non-canonical systemId/scriptName mappings: ${nonCanonicalRegistrySystemIdMappings.join(', ')}`,
    },
    {
      id: 'orchestration_registry_script_names_align_with_registry_keys',
      ok: nonCanonicalRegistryScriptMappings.length === 0,
      detail: nonCanonicalRegistryScriptMappings.length === 0
        ? 'non-swarm orchestration registry scriptName values align with canonical registry keys (or allowed aliases)'
        : `non_canonical_registry_script_mappings=${nonCanonicalRegistryScriptMappings.join(', ')}`,
    },
    {
      id: 'orchestration_registry_namespace_prefix_map_covers_audited_keys',
      ok: missingExpectedNamespacePrefixKeys.length === 0,
      detail: missingExpectedNamespacePrefixKeys.length === 0
        ? 'all audited orchestration keys have expected namespace prefix mapping contracts'
        : `missing_expected_namespace_prefix_keys=${missingExpectedNamespacePrefixKeys.join(',')}`,
    },
    {
      id: 'orchestration_registry_namespace_prefix_map_has_no_unknown_keys',
      ok: unknownExpectedNamespacePrefixKeys.length === 0,
      detail: unknownExpectedNamespacePrefixKeys.length === 0
        ? 'expected namespace prefix map does not declare unknown non-audited keys'
        : `unknown_expected_namespace_prefix_keys=${unknownExpectedNamespacePrefixKeys.join(',')}`,
    },
    {
      id: 'orchestration_registry_audited_namespace_prefixes_match_family',
      ok: auditedNamespacePrefixMismatches.length === 0,
      detail: auditedNamespacePrefixMismatches.length === 0
        ? 'audited orchestration registry keys use expected SYSTEMS-<DOMAIN>-* namespace families'
        : `audited namespace prefix mismatches: ${auditedNamespacePrefixMismatches.join(', ')}`,
    },
    {
      id: 'orchestration_registry_system_ids_use_orchestration_namespace',
      ok: invalidRegistrySystemIdNamespaces.length === 0,
      detail: invalidRegistrySystemIdNamespaces.length === 0
        ? 'non-swarm orchestration registry systemId values use approved SYSTEMS-* orchestration domain namespaces'
        : `non-approved orchestration systemId namespaces: ${invalidRegistrySystemIdNamespaces.join(', ')}`,
    },
    {
      id: 'orchestration_registry_swarm_binding_present_once',
      ok: swarmRegistryBindings.length === 1,
      detail:
        swarmRegistryBindings.length === 1
          ? 'exactly one swarm runtime shim binding is declared'
          : `swarm_binding_count=${swarmRegistryBindings.length}`,
    },
    {
      id: 'orchestration_registry_swarm_binding_shape_is_canonical',
      ok: invalidSwarmRegistryBindings.length === 0,
      detail:
        invalidSwarmRegistryBindings.length === 0
          ? 'swarm runtime shim binding remains key-only (no scriptName/systemId)'
          : `invalid_swarm_bindings=${invalidSwarmRegistryBindings.join(',')}`,
    },
    {
      id: 'orchestration_surface_shim_keys_are_unique',
      ok: duplicateOrchestrationShimKeyBindings.length === 0,
      detail: duplicateOrchestrationShimKeyBindings.length === 0
        ? 'surface orchestration shim key bindings are unique per script entrypoint'
        : `duplicate shim key bindings: ${duplicateOrchestrationShimKeyBindings.join(', ')}`,
    },
    {
      id: 'orchestration_surface_shim_keys_are_sorted',
      ok: outOfOrderOrchestrationShimKeys.length === 0,
      detail: outOfOrderOrchestrationShimKeys.length === 0
        ? 'surface orchestration shim keys are lexicographically sorted for deterministic review diffs'
        : `out_of_order_shim_key_pairs=${outOfOrderOrchestrationShimKeys.join(',')}`,
    },
    {
      id: 'orchestration_surface_shim_script_files_are_unique',
      ok: duplicateOrchestrationShimScriptFiles.length === 0,
      detail: duplicateOrchestrationShimScriptFiles.length === 0
        ? 'surface orchestration shim script file bindings are unique'
        : `duplicate_shim_script_files=${duplicateOrchestrationShimScriptFiles.join(',')}`,
    },
    {
      id: 'orchestration_surface_shim_script_names_match_registry_bindings',
      ok: shimScriptNameMismatches.length === 0,
      detail: shimScriptNameMismatches.length === 0
        ? 'surface orchestration shim file names align with adapter registry scriptName bindings'
        : `shim_script_name_mismatches=${shimScriptNameMismatches.join(',')}`,
    },
    {
      id: 'orchestration_surface_registry_script_files_exist_for_non_swarm_bindings',
      ok: missingRegistryScriptFiles.length === 0,
      detail: missingRegistryScriptFiles.length === 0
        ? 'all non-swarm orchestration registry bindings resolve to concrete surface/orchestration script files'
        : `missing_registry_script_files=${missingRegistryScriptFiles.join(',')}`,
    },
    {
      id: 'orchestration_surface_registry_script_key_bindings_match_module_contracts',
      ok: registryScriptKeyBindingMismatches.length === 0,
      detail: registryScriptKeyBindingMismatches.length === 0
        ? 'all non-swarm orchestration registry scripts bind the expected orchestration module key'
        : `registry_script_key_binding_mismatches=${registryScriptKeyBindingMismatches.join(',')}`,
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
    {
      id: 'orchestration_surface_registry_is_adapter_boundary_only',
      ok:
        orchestrationSurfaceRegistry.includes('const ORCHESTRATION_SURFACE_REGISTRY = Object.freeze(') &&
        orchestrationSurfaceRegistry.includes(
          'bindRuntimeSystemModule(__dirname, binding.scriptName, binding.systemId, currentModule, argv)',
        ) &&
        !orchestrationSurfaceRegistry.includes('client/runtime/systems/'),
      detail:
        'adapter orchestration registry remains canonical boundary and does not embed client/runtime system paths',
    },
    {
      id: 'orchestration_surface_registry_covers_audited_modules',
      ok: missingOrchestrationRegistryKeys.length === 0,
      detail:
        missingOrchestrationRegistryKeys.length === 0
          ? 'all audited orchestration wrapper modules are registered in adapters/runtime/orchestration_surface_modules.ts'
          : `missing_registry_keys=${missingOrchestrationRegistryKeys.join(',')}`,
    },
    {
      id: 'orchestration_audited_module_key_list_is_unique',
      ok: duplicateAuditedOrchestrationModuleKeys.length === 0,
      detail:
        duplicateAuditedOrchestrationModuleKeys.length === 0
          ? 'audited orchestration module key list is duplicate-free'
          : `duplicate_audited_module_keys=${duplicateAuditedOrchestrationModuleKeys.join(',')}`,
    },
    {
      id: 'orchestration_audited_module_key_list_is_sorted',
      ok: outOfOrderAuditedOrchestrationModuleKeys.length === 0,
      detail:
        outOfOrderAuditedOrchestrationModuleKeys.length === 0
          ? 'audited orchestration module key list is lexicographically sorted'
          : `out_of_order_audited_module_key_pairs=${outOfOrderAuditedOrchestrationModuleKeys.join(',')}`,
    },
    {
      id: 'orchestration_surface_registry_non_swarm_keys_match_audited_list',
      ok: nonSwarmRegistryKeysOutsideAuditedList.length === 0,
      detail:
        nonSwarmRegistryKeysOutsideAuditedList.length === 0
          ? 'all non-swarm orchestration registry keys map to audited module keys'
          : `non_swarm_registry_keys_outside_audited_list=${nonSwarmRegistryKeysOutsideAuditedList.join(',')}`,
    },
    {
      id: 'orchestration_surface_shim_keys_match_audited_list',
      ok: shimKeysOutsideAuditedList.length === 0,
      detail:
        shimKeysOutsideAuditedList.length === 0
          ? 'all parsed orchestration shim keys map to audited module keys'
          : `shim_keys_outside_audited_list=${shimKeysOutsideAuditedList.join(',')}`,
    },
    {
      id: 'orchestration_surface_registry_non_swarm_binding_count_matches_audited_list',
      ok: nonSwarmRegistryBindingCountDelta === 0,
      detail:
        nonSwarmRegistryBindingCountDelta === 0
          ? 'non-swarm orchestration registry binding count matches audited module key count'
          : `non_swarm_registry_binding_count_delta=${nonSwarmRegistryBindingCountDelta};non_swarm_registry_bindings=${nonSwarmRegistryBindings.length};audited_keys=${auditedOrchestrationModuleKeys.length}`,
    },
    {
      id: 'orchestration_surface_shim_binding_count_matches_audited_list',
      ok: shimBindingCountDelta === 0,
      detail:
        shimBindingCountDelta === 0
          ? 'orchestration shim binding key count matches audited module key count'
          : `shim_binding_count_delta=${shimBindingCountDelta};shim_keys=${orchestrationShimKeys.length};audited_keys=${auditedOrchestrationModuleKeys.length}`,
    },
    {
      id: 'orchestration_surface_registry_bindings_parseable',
      ok: orchestrationSurfaceRegistryBindings.length > 0,
      detail:
        orchestrationSurfaceRegistryBindings.length > 0
          ? `parsed_bindings=${orchestrationSurfaceRegistryBindings.length}`
          : 'no registry bindings parsed from adapters/runtime/orchestration_surface_modules.ts',
    },
    {
      id: 'orchestration_surface_registry_bindings_have_required_fields',
      ok: invalidRegistryBindings.length === 0,
      detail:
        invalidRegistryBindings.length === 0
          ? 'all non-swarm registry bindings declare scriptName + systemId'
          : `invalid_registry_bindings=${invalidRegistryBindings.join(',')}`,
    },
    {
      id: 'orchestration_surface_registry_script_name_map_is_canonical',
      ok: nonCanonicalRegistryScriptMappings.length === 0,
      detail:
        nonCanonicalRegistryScriptMappings.length === 0
          ? 'registry scriptName map is canonical (key match or approved alias)'
          : `noncanonical_script_mappings=${nonCanonicalRegistryScriptMappings.join(',')}`,
    },
    {
      id: 'orchestration_surface_registry_script_names_are_unique',
      ok: duplicateRegistryScriptNames.length === 0,
      detail:
        duplicateRegistryScriptNames.length === 0
          ? 'registry scriptName bindings are one-to-one'
          : `duplicate_script_names=${duplicateRegistryScriptNames.join('|')}`,
    },
    {
      id: 'orchestration_surface_registry_system_ids_are_unique',
      ok: duplicateRegistrySystemIds.length === 0,
      detail:
        duplicateRegistrySystemIds.length === 0
          ? 'registry systemId bindings are one-to-one'
          : `duplicate_system_ids=${duplicateRegistrySystemIds.join('|')}`,
    },
    {
      id: 'orchestration_surface_registry_covers_surface_shims',
      ok: missingRegistryKeysForShims.length === 0,
      detail:
        missingRegistryKeysForShims.length === 0
          ? 'every surface/orchestration script shim is backed by an adapter registry key'
          : `missing_registry_keys_for_shims=${missingRegistryKeysForShims.join(',')}`,
    },
    {
      id: 'orchestration_surface_registry_script_files_exist',
      ok: missingRegistryScriptFiles.length === 0,
      detail:
        missingRegistryScriptFiles.length === 0
          ? 'every non-swarm registry binding points to an existing surface/orchestration script file'
          : `missing_registry_script_files=${missingRegistryScriptFiles.join(',')}`,
    },
    {
      id: 'orchestration_surface_registry_script_key_bindings_match',
      ok: registryScriptKeyBindingMismatches.length === 0,
      detail:
        registryScriptKeyBindingMismatches.length === 0
          ? 'every non-swarm registry script file binds the expected module key'
          : `registry_script_key_binding_mismatches=${registryScriptKeyBindingMismatches.join(',')}`,
    },
    {
      id: 'audited_orchestration_modules_have_surface_shims',
      ok: missingShimKeysForAuditedModules.length === 0,
      detail:
        missingShimKeysForAuditedModules.length === 0
          ? 'all audited orchestration modules have a surface/orchestration shim entrypoint'
          : `missing_shim_keys_for_audited_modules=${missingShimKeysForAuditedModules.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_bindings_cover_audited_modules',
      ok: missingClientWrapperKeysForAuditedModules.length === 0,
      detail:
        missingClientWrapperKeysForAuditedModules.length === 0
          ? 'all audited orchestration module keys have client compatibility wrapper bindings'
          : `missing_client_wrapper_keys_for_audited_modules=${missingClientWrapperKeysForAuditedModules.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_bindings_have_no_unknown_keys',
      ok: unknownClientWrapperKeys.length === 0,
      detail:
        unknownClientWrapperKeys.length === 0
          ? 'client compatibility wrapper bindings declare only audited orchestration module keys'
          : `unknown_client_wrapper_keys=${unknownClientWrapperKeys.join(',')}`,
    },
    {
      id: 'orchestration_client_wrapper_binding_count_matches_audited_modules',
      ok: clientWrapperBindingCountDelta === 0,
      detail:
        clientWrapperBindingCountDelta === 0
          ? 'client compatibility wrapper binding count matches audited orchestration module count'
          : `client_wrapper_binding_count_delta=${clientWrapperBindingCountDelta};client_wrapper_bindings=${auditedClientWrapperBindings.length};audited_modules=${auditedOrchestrationModuleKeys.length}`,
    },
    {
      id: 'orchestration_client_wrappers_include_compatibility_marker',
      ok: clientWrapperRowsMissingCompatibilityMarker.length === 0,
      detail:
        clientWrapperRowsMissingCompatibilityMarker.length === 0
          ? 'all client compatibility wrappers include canonical thin-wrapper marker text'
          : `client_wrappers_missing_compatibility_marker=${clientWrapperRowsMissingCompatibilityMarker.join(',')}`,
    },
    {
      id: 'orchestration_client_wrappers_delegate_to_expected_surface_scripts',
      ok: clientWrapperRowsMissingExpectedSurfaceDelegation.length === 0,
      detail:
        clientWrapperRowsMissingExpectedSurfaceDelegation.length === 0
          ? 'all client compatibility wrappers delegate to expected surface/orchestration scripts'
          : `client_wrappers_missing_expected_surface_delegation=${clientWrapperRowsMissingExpectedSurfaceDelegation.join(',')}`,
    },
    {
      id: 'orchestration_client_wrappers_reference_single_surface_delegate_script',
      ok: clientWrapperRowsWithNonDeterministicDelegationCount.length === 0,
      detail:
        clientWrapperRowsWithNonDeterministicDelegationCount.length === 0
          ? 'all client compatibility wrappers reference exactly one surface/orchestration delegate script path'
          : `client_wrappers_with_nondeterministic_delegate_path_count=${clientWrapperRowsWithNonDeterministicDelegationCount.join(',')}`,
    },
    {
      id: 'orchestration_client_wrappers_avoid_nexus_or_policy_authority_tokens',
      ok: clientWrapperRowsWithAuthorityTokens.length === 0,
      detail:
        clientWrapperRowsWithAuthorityTokens.length === 0
          ? 'client compatibility wrappers avoid nexus/policy/tool-broker authority tokens'
          : `client_wrappers_with_authority_tokens=${clientWrapperRowsWithAuthorityTokens.join(',')}`,
    },
    {
      id: 'orchestration_client_wrappers_avoid_spawn_tokens',
      ok: clientWrapperRowsWithSpawnTokens.length === 0,
      detail:
        clientWrapperRowsWithSpawnTokens.length === 0
          ? 'client compatibility wrappers do not spawn subprocesses'
          : `client_wrappers_with_spawn_tokens=${clientWrapperRowsWithSpawnTokens.join(',')}`,
    },
    {
      id: 'orchestration_surface_shim_keys_use_canonical_token_format',
      ok: invalidOrchestrationShimKeyFormats.length === 0,
      detail:
        invalidOrchestrationShimKeyFormats.length === 0
          ? 'surface/orchestration shim keys use canonical [a-z0-9_]+ token format'
          : `invalid_orchestration_shim_keys=${invalidOrchestrationShimKeyFormats.join(',')}`,
    },
    {
      id: 'orchestration_surface_shim_script_filenames_use_canonical_token_format',
      ok: invalidOrchestrationShimScriptFileFormats.length === 0,
      detail:
        invalidOrchestrationShimScriptFileFormats.length === 0
          ? 'surface/orchestration shim script filenames use canonical [a-z0-9_]+.ts format'
          : `invalid_orchestration_shim_script_filenames=${invalidOrchestrationShimScriptFileFormats.join(',')}`,
    },
    {
      id: 'orchestration_surface_shim_script_filename_matches_bound_key',
      ok: orchestrationShimScriptFileKeyMismatches.length === 0,
      detail:
        orchestrationShimScriptFileKeyMismatches.length === 0
          ? 'surface/orchestration shim script filenames align with bound module keys'
          : `orchestration_shim_script_filename_key_mismatches=${orchestrationShimScriptFileKeyMismatches.join(',')}`,
    },
    {
      id: 'orchestration_surface_shims_include_bind_contract',
      ok: orchestrationShimRowsMissingBindCall.length === 0,
      detail:
        orchestrationShimRowsMissingBindCall.length === 0
          ? 'surface/orchestration shim scripts include canonical bindOrchestrationSurfaceModule contract call'
          : `orchestration_shims_missing_bind_contract=${orchestrationShimRowsMissingBindCall.join(',')}`,
    },
    {
      id: 'orchestration_surface_shims_must_not_import_client_runtime',
      ok: orchestrationShimRowsWithClientRuntimeImport.length === 0,
      detail:
        orchestrationShimRowsWithClientRuntimeImport.length === 0
          ? 'surface/orchestration shim scripts do not import client/runtime layers'
          : `orchestration_shims_with_client_runtime_import=${orchestrationShimRowsWithClientRuntimeImport.join(',')}`,
    },
    {
      id: 'orchestration_surface_shims_must_not_spawn_subprocesses',
      ok: orchestrationShimRowsWithSpawnTokens.length === 0,
      detail:
        orchestrationShimRowsWithSpawnTokens.length === 0
          ? 'surface/orchestration shim scripts do not spawn subprocesses'
          : `orchestration_shims_with_spawn_tokens=${orchestrationShimRowsWithSpawnTokens.join(',')}`,
    },
    {
      id: 'orchestration_namespace_prefix_map_values_use_canonical_prefix_format',
      ok: invalidExpectedNamespacePrefixValues.length === 0,
      detail:
        invalidExpectedNamespacePrefixValues.length === 0
          ? 'expected orchestration namespace prefix map values use canonical SYSTEMS-<DOMAIN>- format'
          : `invalid_expected_namespace_prefix_values=${invalidExpectedNamespacePrefixValues.join(',')}`,
    },
    {
      id: 'orchestration_namespace_prefix_map_values_are_allowlisted',
      ok: disallowedExpectedNamespacePrefixValues.length === 0,
      detail:
        disallowedExpectedNamespacePrefixValues.length === 0
          ? 'expected orchestration namespace prefix map values are within approved SYSTEMS namespace allowlist'
          : `disallowed_expected_namespace_prefix_values=${disallowedExpectedNamespacePrefixValues.join(',')}`,
    },
    {
      id: 'orchestration_registry_non_swarm_entries_have_script_names',
      ok: nonSwarmBindingsMissingScriptName.length === 0,
      detail:
        nonSwarmBindingsMissingScriptName.length === 0
          ? 'all non-swarm orchestration registry entries declare scriptName'
          : `non_swarm_bindings_missing_script_name=${nonSwarmBindingsMissingScriptName.join(',')}`,
    },
    {
      id: 'orchestration_registry_non_swarm_entries_have_system_ids',
      ok: nonSwarmBindingsMissingSystemId.length === 0,
      detail:
        nonSwarmBindingsMissingSystemId.length === 0
          ? 'all non-swarm orchestration registry entries declare systemId'
          : `non_swarm_bindings_missing_system_id=${nonSwarmBindingsMissingSystemId.join(',')}`,
    },
    {
      id: 'orchestration_registry_non_swarm_script_names_are_token_only',
      ok: nonSwarmBindingsWithNonCanonicalScriptPathTokens.length === 0,
      detail:
        nonSwarmBindingsWithNonCanonicalScriptPathTokens.length === 0
          ? 'all non-swarm orchestration registry scriptName values are canonical tokens (no slashes/extensions)'
          : `non_swarm_bindings_with_noncanonical_script_name_tokens=${nonSwarmBindingsWithNonCanonicalScriptPathTokens.join(',')}`,
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
