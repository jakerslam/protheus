#!/usr/bin/env node
'use strict';

const path = require('path');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const DEFAULT_LANE = 'RUNTIME-LEGACY-ALIAS';
const MAX_LANE_LEN = 128;
const MAX_ARG_LEN = 512;
const bridge = createOpsLaneBridge(__dirname, 'legacy_alias_adapter', 'runtime-systems', {
  inheritStdio: true
});

function sanitizeArg(value) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, MAX_ARG_LEN);
}

function normalizeLaneId(raw, fallback = DEFAULT_LANE) {
  const v = sanitizeArg(raw || '')
    .toUpperCase()
    .replace(/[^A-Z0-9_.-]+/g, '-')
    .replace(/^-+|-+$/g, '');
  return (v.slice(0, MAX_LANE_LEN) || fallback).slice(0, MAX_LANE_LEN);
}

function parseArgs(argv = []) {
  const args = Array.isArray(argv) ? argv.map((row) => sanitizeArg(row)).filter(Boolean) : [];
  let laneId = '';
  let scriptPath = '';
  const passthrough = [];

  for (let i = 0; i < args.length; i += 1) {
    const token = String(args[i] || '');
    if (token.startsWith('--lane-id=')) {
      laneId = token.slice('--lane-id='.length).trim();
      continue;
    }
    if (token === '--lane-id') {
      laneId = String(args[i + 1] || '').trim();
      i += 1;
      continue;
    }
    if (token.startsWith('--script=')) {
      scriptPath = token.slice('--script='.length).trim();
      continue;
    }
    if (token === '--script') {
      scriptPath = String(args[i + 1] || '').trim();
      i += 1;
      continue;
    }
    passthrough.push(token);
  }

  return { laneId, scriptPath, passthrough };
}

function laneFromScript(scriptPath) {
  const raw = sanitizeArg(scriptPath || '');
  if (!raw) return '';
  const runtimeRoot = path.resolve(__dirname, '..', '..');
  const abs = path.resolve(raw);
  const rel = path.relative(runtimeRoot, abs).replace(/\\/g, '/').replace(/\.[^.]+$/, '');
  if (!rel || rel.startsWith('..')) return '';
  return normalizeLaneId(`RUNTIME-${rel}`, DEFAULT_LANE);
}

function laneFromAliasRel(aliasRel) {
  const rel = sanitizeArg(aliasRel || '')
    .replace(/\\/g, '/')
    .replace(/^\.\//, '')
    .replace(/\.[^.]+$/, '');
  if (!rel || rel.startsWith('..')) return '';
  return normalizeLaneId(`RUNTIME-${rel}`, DEFAULT_LANE);
}

function resolveLane(inputLaneId, scriptPath) {
  const lane = normalizeLaneId(String(inputLaneId || '').trim(), '');
  if (lane) return lane;
  const fromScript = laneFromScript(scriptPath);
  if (fromScript) return fromScript;
  return DEFAULT_LANE;
}

function runBridge(laneId, argv = []) {
  const args = Array.isArray(argv) ? argv.map((v) => sanitizeArg(v)).filter(Boolean) : [];
  const out = bridge.run([`--lane-id=${laneId}`].concat(args));
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) {
    process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  } else if (!out || (!out.stdout && !out.stderr)) {
    process.stdout.write(
      `${JSON.stringify({
        ok: false,
        type: 'legacy_alias_adapter',
        error: 'bridge_no_output',
        lane_id: laneId,
        status: Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1
      })}\n`
    );
  }
  return out;
}

function runLegacyAlias(spec = {}, argv = process.argv.slice(2)) {
  const explicitLane = normalizeLaneId(spec.lane_id || spec.laneId || '', '');
  const laneFromScriptPath = laneFromScript(spec.script || spec.scriptPath || '');
  const laneFromAlias = laneFromAliasRel(spec.alias_rel || spec.aliasRel || '');
  const laneId = normalizeLaneId(
    explicitLane || laneFromScriptPath || laneFromAlias || DEFAULT_LANE,
    DEFAULT_LANE
  );
  return runBridge(laneId, argv);
}

function run(argv = []) {
  const parsed = parseArgs(argv);
  const laneId = resolveLane(parsed.laneId, parsed.scriptPath);
  return runBridge(laneId, parsed.passthrough);
}

module.exports = {
  parseArgs,
  laneFromScript,
  laneFromAliasRel,
  resolveLane,
  runLegacyAlias,
  run
};

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.exit(Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1);
}
