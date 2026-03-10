#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

function cleanText(value, maxLen = 200) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseJson(stdout) {
  const text = String(stdout || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {}
  const lines = text.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function ensureDir(filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function nowIso() {
  return new Date().toISOString();
}

function rootDir() {
  const override = cleanText(process.env.PROTHEUS_HEARTBEAT_ROOT || '', 500);
  if (override) return path.resolve(override);
  return path.resolve(__dirname, '..', '..');
}

function outputPath(root) {
  const override = cleanText(process.env.PROTHEUS_HEARTBEAT_KPI_OUT || '', 500);
  if (override) return path.resolve(override);
  return path.join(root, 'artifacts', 'heartbeat', 'heartbeat_kpi_latest.json');
}

function historyPath(root) {
  const override = cleanText(process.env.PROTHEUS_HEARTBEAT_KPI_HISTORY || '', 500);
  if (override) return path.resolve(override);
  return path.join(root, 'artifacts', 'heartbeat', 'heartbeat_kpi_history.jsonl');
}

function runReminderSnapshot(root, mode) {
  const bridge = path.join(root, 'client', 'runtime', 'systems', 'ops', 'reminder_data_bridge.js');
  const run = spawnSync(process.execPath, [bridge, mode], {
    cwd: root,
    encoding: 'utf8',
    env: process.env,
    timeout: 10000,
    maxBuffer: 1024 * 1024
  });
  const payload = parseJson(run.stdout);
  return {
    ok: run.status === 0 && !!payload && payload.ok !== false,
    status: Number.isFinite(run.status) ? Number(run.status) : 1,
    payload,
    stderr: cleanText(run.stderr || '', 500)
  };
}

function runDeploymentHealth(root) {
  if (String(process.env.PROTHEUS_HEARTBEAT_KPI_SKIP_DEPLOY || '0').trim() === '1') {
    return {
      ok: true,
      skipped: true,
      status: 0,
      output_preview: 'deployment_health_skipped'
    };
  }
  const script = path.join(root, 'scripts', 'utils', 'health-check-deployment.sh');
  if (!fs.existsSync(script)) {
    return {
      ok: false,
      skipped: false,
      status: 2,
      output_preview: 'deployment_health_script_missing'
    };
  }
  const run = spawnSync('bash', [script], {
    cwd: root,
    encoding: 'utf8',
    env: process.env,
    timeout: 120000,
    maxBuffer: 1024 * 1024 * 4
  });
  const merged = `${run.stdout || ''}\n${run.stderr || ''}`;
  return {
    ok: run.status === 0,
    skipped: false,
    status: Number.isFinite(run.status) ? Number(run.status) : 1,
    output_preview: cleanText(merged, 500)
  };
}

function toRatio(numerator, denominator) {
  if (!Number.isFinite(numerator) || !Number.isFinite(denominator) || denominator <= 0) return 0;
  return Number((numerator / denominator).toFixed(4));
}

function buildReport(root) {
  const ts = nowIso();
  const slack = runReminderSnapshot(root, 'slack-status');
  const moltcheck = runReminderSnapshot(root, 'moltcheck-status');
  const deploy = runDeploymentHealth(root);

  const slackReady = !!(slack.payload && slack.payload.ready);
  const moltcheckReady = !!(moltcheck.payload && moltcheck.payload.mode === 'automation_ready');
  const deploymentReady = !!deploy.ok;

  const checksTotal = 3;
  const checksPassed = Number(slackReady) + Number(moltcheckReady) + Number(deploymentReady);
  const completionRate = toRatio(checksPassed, checksTotal);

  return {
    ok: true,
    type: 'heartbeat_kpi_report',
    ts,
    kpi: {
      checks_total: checksTotal,
      checks_passed: checksPassed,
      completion_rate: completionRate
    },
    checks: {
      slack_status: {
        ready: slackReady,
        status: slack.status,
        missing: Array.isArray(slack.payload && slack.payload.missing) ? slack.payload.missing : []
      },
      moltcheck_status: {
        ready: moltcheckReady,
        status: moltcheck.status,
        mode: (moltcheck.payload && moltcheck.payload.mode) || null,
        missing: Array.isArray(moltcheck.payload && moltcheck.payload.missing) ? moltcheck.payload.missing : []
      },
      deployment_health: {
        ready: deploymentReady,
        status: deploy.status,
        skipped: !!deploy.skipped,
        output_preview: deploy.output_preview
      }
    },
    recommendation: completionRate >= 0.67
      ? 'heartbeat_lane_operational'
      : 'prioritize_bridge_or_deployment_remediation'
  };
}

function main() {
  const root = rootDir();
  const report = buildReport(root);
  const out = outputPath(root);
  const history = historyPath(root);

  ensureDir(out);
  ensureDir(history);

  fs.writeFileSync(out, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  fs.appendFileSync(history, `${JSON.stringify(report)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(report)}\n`);
}

if (require.main === module) {
  main();
}

module.exports = {
  buildReport,
  parseJson
};
