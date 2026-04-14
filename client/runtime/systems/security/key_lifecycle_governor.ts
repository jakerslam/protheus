#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/security (authoritative)
// Thin TypeScript wrapper only.

const { runSecurityPlane } = require('../../lib/security_plane_bridge.ts');
const { createLegacyRetiredModuleForFile } = require('../../lib/legacy_retired_wrapper.ts');

const SYSTEM_ID = 'SYSTEMS-SECURITY-KEY_LIFECYCLE_GOVERNOR';
const TOOL = 'key-lifecycle-governor';
const DEFAULT_KEY_ID = 'runtime-default';
const DEFAULT_ACTION = 'status';
const legacy = createLegacyRetiredModuleForFile(__filename);

function normalizeArgs(args = []) {
  return Array.isArray(args) ? args.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function hasFlag(args, key) {
  const list = normalizeArgs(args);
  return list.some((token) => token === key || token.startsWith(`${key}=`));
}

function stripLeadingCommand(args = []) {
  const list = normalizeArgs(args);
  if (list.length === 0) return [];
  if (list[0].startsWith('-')) return list;
  return list.slice(1);
}

function ensureStrict(args = []) {
  const list = normalizeArgs(args);
  if (!hasFlag(list, '--strict')) list.push('--strict=1');
  return list;
}

function ensureRequiredFlags(args = []) {
  const list = normalizeArgs(args);
  if (!hasFlag(list, '--key-id')) list.push(`--key-id=${DEFAULT_KEY_ID}`);
  if (!hasFlag(list, '--action')) list.push(`--action=${DEFAULT_ACTION}`);
  return list;
}

function outputResult(out) {
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  }
  return Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
}

function bridgeLooksUnavailable(out) {
  const payloadError = out && out.payload && out.payload.error ? String(out.payload.error) : '';
  const stderr = out && out.stderr ? String(out.stderr) : '';
  const combined = `${payloadError} ${stderr}`.toLowerCase();
  return combined.includes('bridge_failed') || combined.includes('kernel_bridge_failed');
}

function run(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const command = (args[0] || 'run').toLowerCase();
  if (command === 'status') {
    return outputResult(runSecurityPlane('status', []));
  }

  const laneArgs = ensureRequiredFlags(ensureStrict(stripLeadingCommand(args)));
  const out = runSecurityPlane(TOOL, laneArgs);
  if (bridgeLooksUnavailable(out)) {
    return outputResult(legacy.run(args));
  }
  return outputResult(out);
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  systemId: SYSTEM_ID,
  tool: TOOL,
  defaultKeyId: DEFAULT_KEY_ID,
  defaultAction: DEFAULT_ACTION,
  run
};
