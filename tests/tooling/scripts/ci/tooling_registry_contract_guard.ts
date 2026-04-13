#!/usr/bin/env node
/* eslint-disable no-console */
import { loadGateRegistry } from '../../lib/manifest.ts';
import { emitStructuredResult } from '../../lib/result.ts';
const DEFAULT_REGISTRY = 'tests/tooling/config/tooling_gate_registry.json';
const DEFAULT_OUT = 'core/local/artifacts/tooling_registry_contract_guard_current.json';

function parseArgs(argv: string[]) {
  const out = {
    strict: false,
    registry: DEFAULT_REGISTRY,
    out: DEFAULT_OUT,
  };
  for (const raw of argv) {
    const arg = String(raw || '').trim();
    if (!arg) continue;
    if (arg === '--strict' || arg === '--strict=1') out.strict = true;
    else if (arg.startsWith('--strict=')) {
      const value = arg.slice('--strict='.length).trim().toLowerCase();
      out.strict = ['1', 'true', 'yes', 'on'].includes(value);
    } else if (arg.startsWith('--registry=')) {
      out.registry = arg.slice('--registry='.length).trim() || out.registry;
    } else if (arg.startsWith('--out=')) {
      out.out = arg.slice('--out='.length).trim() || out.out;
    }
  }
  return out;
}

function containsPlaceholder(value: string): boolean {
  return value.includes('${');
}

function looksLikeArtifactFlag(value: string): boolean {
  return (
    value.startsWith('--out=') ||
    value.startsWith('--out-json=') ||
    value.startsWith('--out-markdown=') ||
    value.startsWith('--scope=') ||
    value.startsWith('--boundary=') ||
    value.startsWith('--disposition=')
  );
}

function run(argv: string[]) {
  const args = parseArgs(argv);
  const registry = loadGateRegistry(args.registry);
  const failures: Array<{ id: string; detail: string }> = [];
  const gates = Object.entries(registry.gates || {});

  for (const [gateId, gate] of gates) {
    const command = Array.isArray(gate.command) ? gate.command : [];
    const artifactPaths = Array.isArray(gate.artifact_paths) ? gate.artifact_paths : [];

    for (const part of command) {
      const value = String(part || '');
      if (looksLikeArtifactFlag(value) && containsPlaceholder(value)) {
        failures.push({
          id: gateId,
          detail: `placeholder_artifact_flag:${value}`,
        });
      }
    }

    for (const artifactPath of artifactPaths) {
      const value = String(artifactPath || '').trim();
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
    }
  }

  const payload = {
    ok: failures.length === 0,
    type: 'tooling_registry_contract_guard',
    generated_at: new Date().toISOString(),
    inputs: {
      registry_path: args.registry,
      strict: args.strict,
    },
    summary: {
      gate_count: gates.length,
      failure_count: failures.length,
      pass: failures.length === 0,
    },
    failures,
  };

  return emitStructuredResult(payload, {
    outPath: args.out,
    strict: args.strict,
    ok: payload.ok,
  });
}

process.exit(run(process.argv.slice(2)));
