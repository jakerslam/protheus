#!/usr/bin/env node
'use strict';

const path = require('path');
const {
  createLegacyRetiredModule,
  normalizeLaneId,
  runAsMain
} = require('../../lib/legacy_retired_wrapper.ts');

const RUNTIME_ROOT = path.resolve(__dirname, '..', '..');
const DEFAULT_LANE = 'RUNTIME-LEGACY-ALIAS';

function parseArgs(argv = []) {
  const args = Array.isArray(argv) ? [...argv] : [];
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
  const raw = String(scriptPath || '').trim();
  if (!raw) return '';
  const abs = path.resolve(raw);
  const rel = path.relative(RUNTIME_ROOT, abs).replace(/\\/g, '/').replace(/\.[^.]+$/, '');
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

function run(argv = []) {
  const parsed = parseArgs(argv);
  const laneId = resolveLane(parsed.laneId, parsed.scriptPath);
  const mod = createLegacyRetiredModule(__dirname, 'legacy_alias_adapter', laneId);
  return mod.run(parsed.passthrough);
}

module.exports = {
  parseArgs,
  laneFromScript,
  resolveLane,
  run
};

if (require.main === module) {
  runAsMain({ run }, process.argv.slice(2));
}
