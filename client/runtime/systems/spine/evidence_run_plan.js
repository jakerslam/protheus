#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer2/spine::evidence_run_plan (authoritative)
// Thin wrapper only; authority logic lives in core/layer2/spine.
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge');

process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS || '1200';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS =
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '1500';

const bridge = createOpsLaneBridge(__dirname, 'evidence_run_plan', 'spine');
const COMMAND = 'evidence-run-plan';

function toPressure(raw) {
  const s = String(raw == null ? '' : raw).trim().toLowerCase();
  return s === 'soft' || s === 'hard' ? s : 'none';
}

function runCore(args = []) {
  const out = bridge.run([COMMAND, ...(Array.isArray(args) ? args : [])]);
  if (out && out.stdout) process.stdout.write(out.stdout);
  if (out && out.stderr) process.stderr.write(out.stderr);
  if (out && out.payload && !out.stdout) process.stdout.write(`${JSON.stringify(out.payload)}\n`);
  return out;
}

function computeEvidenceRunPlan(configuredRunsRaw, budgetPressureRaw, projectedPressureRaw) {
  const args = [
    COMMAND,
    `--configured-runs=${String(configuredRunsRaw == null ? '' : configuredRunsRaw)}`,
    `--budget-pressure=${toPressure(budgetPressureRaw)}`,
    `--projected-pressure=${toPressure(projectedPressureRaw)}`
  ];
  const out = bridge.run(args);
  return out && out.payload && out.payload.plan ? out.payload.plan : null;
}

if (require.main === module) {
  const out = runCore(process.argv.slice(2));
  process.exit(Number.isFinite(out && out.status) ? Number(out.status) : 1);
}

module.exports = {
  lane: bridge.lane,
  run: (args = []) => bridge.run([COMMAND, ...(Array.isArray(args) ? args : [])]),
  computeEvidenceRunPlan
};
