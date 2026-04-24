#!/usr/bin/env node
/* eslint-disable no-console */
import { loadGateRegistry } from '../../lib/manifest.ts';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';
const DEFAULT_REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const DEFAULT_OUT = 'core/local/artifacts/tooling_registry_contract_guard_current.json';
const ARTIFACT_FLAG_PREFIXES = ['--out=', '--out-json=', '--out-markdown='];

function resolveArgs(argv: string[]) {
  return {
    strict: argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false),
    registry: readFlag(argv, 'registry') || DEFAULT_REGISTRY,
    out: readFlag(argv, 'out') || DEFAULT_OUT,
  };
}

function containsPlaceholder(value: string): boolean {
  return value.includes('${');
}

function duplicateValues(values: string[]): string[] {
  const counts = new Map<string, number>();
  for (const value of values) counts.set(value, (counts.get(value) || 0) + 1);
  return [...counts.entries()]
    .filter(([, count]) => count > 1)
    .map(([value]) => value)
    .sort();
}

function casefoldDuplicateValues(values: string[]): string[] {
  const folded = values
    .map((value) => String(value || '').trim().toLowerCase())
    .filter((value) => value.length > 0);
  return duplicateValues(folded);
}

function isCanonicalRelativePath(value: string): boolean {
  if (value.trim() !== value) return false;
  if (value.length === 0) return false;
  if (value.includes('\\')) return false;
  if (value.startsWith('/') || value.startsWith('./') || value.startsWith('../')) return false;
  if (value.includes('//')) return false;
  const segments = value.split('/');
  if (segments.some((segment) => segment.length === 0 || segment === '.' || segment === '..')) return false;
  return true;
}

function isCanonicalGateId(value: string): boolean {
  return /^[a-z0-9][a-z0-9:_-]*$/.test(value);
}

function isCanonicalOwner(value: string): boolean {
  return /^[a-z][a-z0-9_-]*$/.test(value);
}

function isCanonicalTimeoutEnv(value: string): boolean {
  return /^VERIFY_[A-Z0-9_]+$/.test(value);
}

function isCanonicalScriptToken(value: string): boolean {
  return /^[a-z0-9][a-z0-9:_-]*$/.test(value);
}

function isArtifactFlagToken(value: string): boolean {
  return ARTIFACT_FLAG_PREFIXES.some((prefix) => value.startsWith(prefix));
}

function extractArtifactFlagPath(value: string): string | null {
  for (const prefix of ARTIFACT_FLAG_PREFIXES) {
    if (value.startsWith(prefix)) {
      return value.slice(prefix.length).trim();
    }
  }
  return null;
}

function looksLikeArtifactFlag(value: string): boolean {
  return (
    isArtifactFlagToken(value) ||
    value.startsWith('--scope=') ||
    value.startsWith('--boundary=') ||
    value.startsWith('--disposition=')
  );
}

function hasCaseInsensitiveSuffix(value: string, suffix: string): boolean {
  return String(value || '').toLowerCase().endsWith(String(suffix || '').toLowerCase());
}

function isCanonicalFailureId(value: string): boolean {
  return /^(_[a-z0-9_]+|[a-z0-9][a-z0-9:_-]*)$/.test(String(value || '').trim());
}

function run(argv: string[]) {
  const args = resolveArgs(argv);
  const registry = loadGateRegistry(args.registry);
  const policyFailures: Array<{ id: string; detail: string }> = [];
  const failures: Array<{ id: string; detail: string }> = [];
  const gates = Object.entries(registry.gates || {});

  if (!isCanonicalRelativePath(args.registry)) {
    policyFailures.push({
      id: '_policy',
      detail: `registry_path_noncanonical:${args.registry}`,
    });
  }
  if (!isCanonicalRelativePath(args.out)) {
    policyFailures.push({
      id: '_policy',
      detail: `output_path_noncanonical:${args.out}`,
    });
  }
  const version = String(registry.version || '').trim();
  if (!/^\d+\.\d+$/.test(version)) {
    policyFailures.push({
      id: '_policy',
      detail: `registry_version_invalid:${version || 'missing'}`,
    });
  }
  if (gates.length === 0) {
    policyFailures.push({
      id: '_policy',
      detail: 'gate_registry_empty',
    });
  }
  const gateIds = gates.map(([gateId]) => gateId);
  const duplicateGateIds = duplicateValues(gateIds);
  if (duplicateGateIds.length > 0) {
    policyFailures.push({
      id: '_policy',
      detail: `gate_id_duplicate:${duplicateGateIds.join(',')}`,
    });
  }
  const duplicateGateIdsCasefold = casefoldDuplicateValues(gateIds);
  if (duplicateGateIdsCasefold.length > 0) {
    policyFailures.push({
      id: '_policy',
      detail: `gate_id_duplicate_casefold:${duplicateGateIdsCasefold.join(',')}`,
    });
  }
  if (!gateIds.every((gateId) => gateId === gateId.toLowerCase())) {
    policyFailures.push({
      id: '_policy',
      detail: 'gate_id_lowercase_contract_invalid',
    });
  }
  const sortedGateIds = [...gateIds].sort((left, right) => left.localeCompare(right));
  if (sortedGateIds.join('|') !== gateIds.join('|')) {
    policyFailures.push({
      id: '_policy',
      detail: 'gate_id_order_drift',
    });
  }
  if (args.registry !== DEFAULT_REGISTRY) {
    policyFailures.push({
      id: '_policy',
      detail: `registry_path_exact_default_contract_invalid:${args.registry}`,
    });
  }
  if (args.out !== DEFAULT_OUT) {
    policyFailures.push({
      id: '_policy',
      detail: `output_path_exact_default_contract_invalid:${args.out}`,
    });
  }
  if (!hasCaseInsensitiveSuffix(args.out, '_current.json')) {
    policyFailures.push({
      id: '_policy',
      detail: `output_path_current_json_suffix_contract_invalid:${args.out}`,
    });
  }
  if (!String(args.out).startsWith('core/local/artifacts/')) {
    policyFailures.push({
      id: '_policy',
      detail: `output_path_artifacts_prefix_contract_invalid:${args.out}`,
    });
  }
  if (/\s/.test(String(args.out || ''))) {
    policyFailures.push({
      id: '_policy',
      detail: `output_path_whitespace_contract_invalid:${args.out}`,
    });
  }
  if (!String(args.registry).startsWith('tests/tooling/config/')) {
    policyFailures.push({
      id: '_policy',
      detail: `registry_path_config_prefix_contract_invalid:${args.registry}`,
    });
  }
  if (!hasCaseInsensitiveSuffix(args.registry, '.json')) {
    policyFailures.push({
      id: '_policy',
      detail: `registry_path_json_suffix_contract_invalid:${args.registry}`,
    });
  }
  if (/\s/.test(String(args.registry || ''))) {
    policyFailures.push({
      id: '_policy',
      detail: `registry_path_whitespace_contract_invalid:${args.registry}`,
    });
  }
  if (containsPlaceholder(String(args.registry || ''))) {
    policyFailures.push({
      id: '_policy',
      detail: `registry_path_placeholder_contract_invalid:${args.registry}`,
    });
  }
  if (containsPlaceholder(String(args.out || ''))) {
    policyFailures.push({
      id: '_policy',
      detail: `output_path_placeholder_contract_invalid:${args.out}`,
    });
  }
  const duplicateArtifactFlagPrefixes = duplicateValues(ARTIFACT_FLAG_PREFIXES);
  if (ARTIFACT_FLAG_PREFIXES.length === 0 || duplicateArtifactFlagPrefixes.length > 0) {
    policyFailures.push({
      id: '_policy',
      detail: `artifact_flag_prefixes_nonempty_unique_contract_invalid:${ARTIFACT_FLAG_PREFIXES.join(',')}`,
    });
  }
  if (!ARTIFACT_FLAG_PREFIXES.every((prefix) => /^--[a-z0-9-]+=/.test(prefix))) {
    policyFailures.push({
      id: '_policy',
      detail: `artifact_flag_prefixes_canonical_contract_invalid:${ARTIFACT_FLAG_PREFIXES.join(',')}`,
    });
  }

  for (const [gateId, gate] of gates) {
    if (!isCanonicalGateId(gateId)) {
      failures.push({
        id: gateId,
        detail: `gate_id_noncanonical:${gateId}`,
      });
    }
    const owner = String(gate.owner || '').trim();
    if (!owner || !isCanonicalOwner(owner)) {
      failures.push({
        id: gateId,
        detail: `gate_owner_invalid:${owner || 'missing'}`,
      });
    }
    const description = String(gate.description || '').trim();
    if (!description || containsPlaceholder(description) || /^(todo|tbd|pending)$/i.test(description)) {
      failures.push({
        id: gateId,
        detail: `gate_description_invalid:${description || 'missing'}`,
      });
    }
    const timeoutSec = (gate as any).timeout_sec;
    const hasTimeoutSec = timeoutSec !== undefined && timeoutSec !== null;
    if (hasTimeoutSec && (!Number.isInteger(timeoutSec) || Number(timeoutSec) <= 0)) {
      failures.push({
        id: gateId,
        detail: `timeout_sec_invalid:${String(timeoutSec)}`,
      });
    }
    const timeoutEnv = String((gate as any).timeout_env || '').trim();
    if (hasTimeoutSec && !timeoutEnv) {
      failures.push({
        id: gateId,
        detail: 'timeout_env_missing',
      });
    }
    if (timeoutEnv && !isCanonicalTimeoutEnv(timeoutEnv)) {
      failures.push({
        id: gateId,
        detail: `timeout_env_noncanonical:${timeoutEnv}`,
      });
    }
    if (!hasTimeoutSec && timeoutEnv) {
      failures.push({
        id: gateId,
        detail: `timeout_env_without_timeout_sec:${timeoutEnv}`,
      });
    }
    if (
      (gate as any).defer_host_stall !== undefined &&
      typeof (gate as any).defer_host_stall !== 'boolean'
    ) {
      failures.push({
        id: gateId,
        detail: `defer_host_stall_nonboolean:${String((gate as any).defer_host_stall)}`,
      });
    }

    const script = String((gate as any).script || '').trim();
    const command = Array.isArray(gate.command) ? gate.command : [];
    const hasScript = script.length > 0;
    const hasCommand = command.length > 0;
    if ((hasScript && hasCommand) || (!hasScript && !hasCommand)) {
      failures.push({
        id: gateId,
        detail: `execution_selector_invalid:script=${hasScript ? '1' : '0'}:command=${hasCommand ? '1' : '0'}`,
      });
    }
    if (hasScript && (!isCanonicalScriptToken(script) || containsPlaceholder(script))) {
      failures.push({
        id: gateId,
        detail: `script_token_invalid:${script}`,
      });
    }

    const commandTokens = command.map((part) => String(part || ''));
    if (commandTokens.some((token) => token.trim() !== token)) {
      failures.push({
        id: gateId,
        detail: 'command_token_trimmed_contract_invalid',
      });
    }
    const duplicateCommandTokens = duplicateValues(commandTokens.filter((part) => part.trim().length > 0));
    if (duplicateCommandTokens.length > 0 && hasCommand) {
      failures.push({
        id: gateId,
        detail: `command_token_duplicate:${duplicateCommandTokens.join(',')}`,
      });
    }
    for (const part of commandTokens) {
      if (!part.trim()) {
        failures.push({
          id: gateId,
          detail: 'command_token_empty',
        });
        continue;
      }
      if (containsPlaceholder(part)) {
        failures.push({
          id: gateId,
          detail: `command_token_placeholder:${part}`,
        });
      }
    }

    const artifactPaths = Array.isArray(gate.artifact_paths) ? gate.artifact_paths : [];
    const artifactTokens = artifactPaths.map((artifactPath) => String(artifactPath || '').trim());
    const duplicateArtifactPaths = duplicateValues(artifactTokens.filter(Boolean));
    if (duplicateArtifactPaths.length > 0) {
      failures.push({
        id: gateId,
        detail: `artifact_path_duplicate:${duplicateArtifactPaths.join(',')}`,
      });
    }
    const duplicateArtifactPathsCasefold = casefoldDuplicateValues(artifactTokens.filter(Boolean));
    if (duplicateArtifactPathsCasefold.length > 0) {
      failures.push({
        id: gateId,
        detail: `artifact_path_duplicate_casefold:${duplicateArtifactPathsCasefold.join(',')}`,
      });
    }

    for (const part of command) {
      const value = String(part || '');
      if (looksLikeArtifactFlag(value) && containsPlaceholder(value)) {
        failures.push({
          id: gateId,
          detail: `placeholder_artifact_flag:${value}`,
        });
      }
    }

    for (const value of artifactTokens) {
      if (!value) {
        failures.push({
          id: gateId,
          detail: 'empty_artifact_path',
        });
        continue;
      }
      if (containsPlaceholder(value)) {
        failures.push({
          id: gateId,
          detail: `placeholder_artifact_path:${value}`,
        });
      }
      if (!isCanonicalRelativePath(value)) {
        failures.push({
          id: gateId,
          detail: `artifact_path_noncanonical:${value}`,
        });
      }
      if (!/\.(json|md)$/i.test(value)) {
        failures.push({
          id: gateId,
          detail: `artifact_path_suffix_invalid:${value}`,
        });
      }
    }

    const artifactFlagPaths = commandTokens
      .map((token) => extractArtifactFlagPath(token))
      .filter((token): token is string => token !== null)
      .map((token) => token.trim())
      .filter(Boolean);
    const duplicateArtifactFlagPaths = duplicateValues(artifactFlagPaths);
    if (duplicateArtifactFlagPaths.length > 0) {
      failures.push({
        id: gateId,
        detail: `artifact_flag_path_duplicate:${duplicateArtifactFlagPaths.join(',')}`,
      });
    }
    for (const flagPath of artifactFlagPaths) {
      if (!isCanonicalRelativePath(flagPath)) {
        failures.push({
          id: gateId,
          detail: `artifact_flag_path_noncanonical:${flagPath}`,
        });
        continue;
      }
      if (!artifactTokens.includes(flagPath)) {
        failures.push({
          id: gateId,
          detail: `artifact_flag_path_missing_registry_binding:${flagPath}`,
        });
      }
    }
  }

  const mergedFailures = [...policyFailures, ...failures];
  if (!policyFailures.every((row) => isCanonicalFailureId(row.id) && String(row.detail || '').trim().length > 0)) {
    policyFailures.push({
      id: '_policy',
      detail: 'policy_failure_shape_contract_invalid',
    });
  }
  if (!mergedFailures.every((row) => isCanonicalFailureId(row.id) && String(row.detail || '').trim().length > 0)) {
    failures.push({
      id: '_policy',
      detail: 'failure_shape_contract_invalid',
    });
  }
  if (!mergedFailures.every((row) => String(row.detail || '') === String(row.detail || '').trim())) {
    failures.push({
      id: '_policy',
      detail: 'failure_detail_trimmed_contract_invalid',
    });
  }
  if (!mergedFailures.every((row) => !containsPlaceholder(String(row.detail || '')))) {
    failures.push({
      id: '_policy',
      detail: 'failure_detail_placeholder_contract_invalid',
    });
  }
  if (!mergedFailures.every((row) => isCanonicalFailureId(row.id))) {
    failures.push({
      id: '_policy',
      detail: 'failure_id_token_contract_invalid',
    });
  }
  const mergedFailuresFinal = [...policyFailures, ...failures];
  const payload = {
    ok: mergedFailuresFinal.length === 0,
    type: 'tooling_registry_contract_guard',
    generated_at: new Date().toISOString(),
    inputs: {
      registry_path: args.registry,
      strict: args.strict,
    },
    summary: {
      gate_count: gates.length,
      policy_failure_count: policyFailures.length,
      failure_count: failures.length,
      total_issue_count: mergedFailuresFinal.length,
      pass: mergedFailuresFinal.length === 0,
    },
    policy_failures: policyFailures,
    failures: mergedFailuresFinal,
  };

  return emitStructuredResult(payload, {
    outPath: args.out,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
