#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1 + layer2 runtime state (authoritative).
// This bridge is intentionally thin and read-only so reminder surfaces can
// check readiness before sending noisy pings.

const fs = require('fs');
const path = require('path');

function cleanText(value, maxLen = 280) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '');
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const idx = token.indexOf('=');
    if (idx >= 0) {
      out[token.slice(2, idx)] = token.slice(idx + 1);
      continue;
    }
    const key = token.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = true;
  }
  return out;
}

function repoRoot() {
  const override = cleanText(process.env.PROTHEUS_REMINDER_ROOT || '', 500);
  if (override) return path.resolve(override);
  return path.resolve(__dirname, '..', '..', '..', '..');
}

function readJson(filePath) {
  try {
    if (!fs.existsSync(filePath)) return null;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function readJsonLinesCount(filePath) {
  try {
    if (!fs.existsSync(filePath)) return 0;
    const lines = fs
      .readFileSync(filePath, 'utf8')
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean);
    return lines.length;
  } catch {
    return 0;
  }
}

function listJsonFiles(dirPath) {
  try {
    return fs
      .readdirSync(dirPath, { withFileTypes: true })
      .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
      .map((entry) => entry.name)
      .sort();
  } catch {
    return [];
  }
}

function lastFileByName(dirPath) {
  const files = listJsonFiles(dirPath);
  if (!files.length) return null;
  return path.join(dirPath, files[files.length - 1]);
}

function hasCredentialFile(root) {
  const candidates = [
    path.join(root, 'client', 'runtime', 'config', 'moltbook', 'credentials.json'),
    path.join(root, 'client', 'runtime', 'config', 'credentials.json')
  ];
  return candidates.some((candidate) => fs.existsSync(candidate));
}

function hasBrowserRuntime(root) {
  const candidates = [
    path.join(root, 'client', 'runtime', 'patches', 'websocket-server-patch.js'),
    path.join(root, 'client', 'runtime', 'patches', 'websocket-heartbeat.js')
  ];
  return candidates.some((candidate) => fs.existsSync(candidate));
}

function buildSlackStatusSnapshot(root) {
  const stateRoot = path.join(root, 'client', 'runtime', 'local', 'state');
  const spineLatest = readJson(path.join(stateRoot, 'spine', 'runs', 'latest.json'));
  const queueDepth = readJsonLinesCount(path.join(stateRoot, 'attention', 'queue.jsonl'));
  const eyeLatest = readJson(path.join(stateRoot, 'eye', 'latest.json'));
  const crossSignalPath = lastFileByName(path.join(stateRoot, 'sensory', 'cross_signal', 'hypotheses'));
  const crossSignalLatest = crossSignalPath ? readJson(crossSignalPath) : null;
  const dopamineLatest = readJson(path.join(stateRoot, 'dopamine', 'ambient', 'latest.json'));

  const missing = [];
  if (!spineLatest) missing.push('spine_status');
  if (!eyeLatest) missing.push('eye_health');
  if (!dopamineLatest) missing.push('dopamine_status');
  if (!crossSignalLatest) missing.push('cross_signal');

  const ready = missing.length === 0;
  return {
    ok: true,
    type: 'slack_status_reminder_snapshot',
    ts: new Date().toISOString(),
    ready,
    cadence_hours: ready ? 6 : 24,
    missing,
    summary: {
      spine: {
        ts: spineLatest && (spineLatest.ts || spineLatest.completed_at || null),
        result: spineLatest && (spineLatest.result || spineLatest.type || null)
      },
      proposal_queue_depth: queueDepth,
      eye: {
        ts: eyeLatest && (eyeLatest.ts || eyeLatest.updated_at || null),
        status: eyeLatest && (eyeLatest.status || eyeLatest.mode || null)
      },
      cross_signal: {
        source: crossSignalPath ? path.relative(root, crossSignalPath) : null,
        hypothesis_count: Array.isArray(crossSignalLatest && crossSignalLatest.hypotheses)
          ? crossSignalLatest.hypotheses.length
          : null
      },
      dopamine: {
        ts: dopamineLatest && (dopamineLatest.ts || null),
        sds: dopamineLatest && dopamineLatest.summary && dopamineLatest.summary.sds != null
          ? Number(dopamineLatest.summary.sds)
          : null
      }
    },
    recommendation: ready
      ? 'send_status_report'
      : 'skip_or_reduce_reminder_until_state_is_healthy'
  };
}

function buildMoltcheckSnapshot(root) {
  const collectorPath = path.join(
    root,
    'client',
    'cognition',
    'adaptive',
    'sensory',
    'eyes',
    'collectors',
    'moltbook_hot.ts'
  );
  const skillPath = path.join(root, 'client', 'cognition', 'skills', 'moltbook', 'moltbook_api.js');
  const apiKeyEnvPresent = !!cleanText(process.env.MOLTBOOK_API_KEY || '', 400);
  const credentialFilePresent = hasCredentialFile(root);
  const browserRuntimePresent = hasBrowserRuntime(root);

  const checks = {
    collector_present: fs.existsSync(collectorPath),
    skill_present: fs.existsSync(skillPath),
    credential_present: apiKeyEnvPresent || credentialFilePresent,
    browser_runtime_present: browserRuntimePresent
  };
  const missing = Object.keys(checks).filter((key) => !checks[key]);
  const automationReady =
    checks.collector_present && checks.skill_present && checks.credential_present;

  return {
    ok: true,
    type: 'moltcheck_readiness_snapshot',
    ts: new Date().toISOString(),
    mode: automationReady ? 'automation_ready' : 'manual_only',
    cadence_hours: automationReady ? 4 : 24,
    checks,
    missing,
    recommendation: automationReady
      ? 'run_collector_then_engage_if_inactive'
      : 'manual_reminder_only_reduce_noise'
  };
}

function main(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const command = cleanText(args._[0] || 'status', 80).toLowerCase();
  const root = repoRoot();
  let payload;
  if (command === 'slack-status' || command === 'status') {
    payload = buildSlackStatusSnapshot(root);
  } else if (command === 'moltcheck-status' || command === 'moltcheck') {
    payload = buildMoltcheckSnapshot(root);
  } else {
    payload = {
      ok: false,
      type: 'reminder_data_bridge_error',
      reason: `unknown_command:${command || 'none'}`
    };
  }
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(payload.ok ? 0 : 2);
}

if (require.main === module) {
  main();
}

module.exports = {
  cleanText,
  parseArgs,
  repoRoot,
  buildSlackStatusSnapshot,
  buildMoltcheckSnapshot,
  main
};
