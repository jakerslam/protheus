#!/usr/bin/env node
'use strict';

const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

function cleanText(v, maxLen = 260) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function toStatus(value, fallback = 1) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function buildFailurePayload(out, fallbackReason = 'conduit_domain_failed') {
  const payload =
    out && out.payload && typeof out.payload === 'object'
      ? out.payload
      : null;
  const reason = cleanText(
    (payload && payload.reason) || (out && (out.stderr || out.stdout)) || fallbackReason,
    300,
  );
  return {
    ok: false,
    engine: 'conduit',
    payload,
    status: toStatus(out && out.status, 1),
    routed_via: cleanText((payload && payload.routed_via) || (out && out.routed_via) || 'conduit', 120),
    error: reason || fallbackReason,
  };
}

function createDomainProxy(scriptDir, lane, domain) {
  const bridge = createOpsLaneBridge(scriptDir, lane, domain);
  return function run(args = []) {
    const out = bridge.run(Array.isArray(args) ? args : []);
    if (out && out.ok === true && out.payload && typeof out.payload === 'object') {
      return {
        ok: true,
        engine: 'conduit',
        payload: out.payload,
        status: toStatus(out.status, 0),
        routed_via: cleanText(out.payload.routed_via || out.routed_via || 'conduit', 120)
      };
    }
    return buildFailurePayload(out);
  };
}

module.exports = {
  createDomainProxy
};
