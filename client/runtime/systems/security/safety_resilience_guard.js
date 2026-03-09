#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer1/security (authoritative)
const { runSecurityPlane, runSecurityPlaneCli } = require('../../lib/security_plane_bridge');

function run(args = []) {
  return runSecurityPlane('safety-resilience-guard', args);
}

function loadPolicy(policyPath = null) {
  return {
    ok: true,
    type: 'safety_resilience_policy_compat',
    lane: 'core/layer1/security',
    policy_path: policyPath ? String(policyPath) : null,
    compatibility_only: true
  };
}

function evaluateSafetyResilience(input = {}, opts = {}) {
  const args = [
    'evaluate',
    `--sentinel-json=${JSON.stringify(input && input.sentinel && typeof input.sentinel === 'object' ? input.sentinel : {})}`,
    `--signals-json=${JSON.stringify(input && input.signals && typeof input.signals === 'object' ? input.signals : {})}`,
    `--apply=${opts && opts.apply === false ? 0 : 1}`
  ];
  if (opts && opts.policy_path) {
    args.push(`--policy=${String(opts.policy_path)}`);
  }
  const out = run(args);
  const payload = out && out.payload && typeof out.payload === 'object' ? out.payload : {};
  if (payload && typeof payload === 'object' && payload.adjusted_sentinel) {
    return payload;
  }
  return {
    ok: Boolean(out && out.ok),
    type: 'safety_resilience_compat',
    lane: 'core/layer1/security',
    compatibility_only: true,
    adjusted_sentinel: input && input.sentinel && typeof input.sentinel === 'object'
      ? input.sentinel
      : {},
    bridge_payload: payload
  };
}

if (require.main === module) {
  runSecurityPlaneCli('safety-resilience-guard', process.argv.slice(2));
}

module.exports = {
  run,
  loadPolicy,
  evaluateSafetyResilience
};
