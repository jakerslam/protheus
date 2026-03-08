#!/usr/bin/env node
'use strict';
export {};

const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../../lib/queued_backlog_runtime');
const { routeTool } = require('./tool_context_router');
const { deliverNotification } = require('./tool_notification_lane');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.SHADOW_TOOL_BRIDGE_POLICY_PATH
  ? path.resolve(String(process.env.SHADOW_TOOL_BRIDGE_POLICY_PATH))
  : path.join(ROOT, 'client', 'runtime', 'config', 'shadow_tool_notification_bridge_policy.json');

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    allowlist: {
      shadows: ['*'],
      personas: ['*']
    },
    notification: {
      default_channel: 'main',
      default_severity: 'info'
    },
    paths: {
      latest_path: 'state/tools/shadow_tool_bridge/latest.json',
      history_path: 'state/tools/shadow_tool_bridge/history.jsonl'
    }
  };
}

function listContainsWildcard(list: string[], value: string) {
  return list.includes('*') || list.includes(value);
}

function normalizeAllowToken(v: unknown) {
  const raw = cleanText(v || '', 80);
  if (raw === '*') return '*';
  return normalizeToken(raw, 80);
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const allowlist = raw && raw.allowlist && typeof raw.allowlist === 'object'
    ? raw.allowlist
    : {};
  const notification = raw && raw.notification && typeof raw.notification === 'object'
    ? raw.notification
    : {};
  const paths = raw && raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || '1.0',
    enabled: toBool(raw.enabled, true),
    allowlist: {
      shadows: Array.isArray(allowlist.shadows) ? allowlist.shadows.map((v: unknown) => normalizeAllowToken(v)).filter(Boolean) : base.allowlist.shadows,
      personas: Array.isArray(allowlist.personas) ? allowlist.personas.map((v: unknown) => normalizeAllowToken(v)).filter(Boolean) : base.allowlist.personas
    },
    notification: {
      default_channel: normalizeToken(notification.default_channel || base.notification.default_channel, 40) || 'main',
      default_severity: normalizeToken(notification.default_severity || base.notification.default_severity, 20) || 'info'
    },
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path)
    }
  };
}

function bridge(args: AnyObj = {}) {
  const policyPath = args.policy
    ? path.resolve(String(args.policy))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) {
    return {
      ok: false,
      type: 'shadow_tool_notification_bridge',
      error: 'bridge_disabled',
      policy_path: policyPath
    };
  }

  const shadow = normalizeToken(args.shadow || args['shadow-id'] || '', 120);
  const persona = normalizeToken(args.persona || '', 120);
  if (!shadow || !persona) {
    return {
      ok: false,
      type: 'shadow_tool_notification_bridge',
      error: 'missing_shadow_or_persona',
      policy_path: policyPath
    };
  }
  if (!listContainsWildcard(policy.allowlist.shadows, shadow)) {
    return {
      ok: false,
      type: 'shadow_tool_notification_bridge',
      error: 'shadow_not_allowlisted',
      shadow,
      policy_path: policyPath
    };
  }
  if (!listContainsWildcard(policy.allowlist.personas, persona)) {
    return {
      ok: false,
      type: 'shadow_tool_notification_bridge',
      error: 'persona_not_allowlisted',
      persona,
      policy_path: policyPath
    };
  }

  const contextJson = args['context-json'] || args.context_json || '{}';
  const routeReceipt = routeTool({
    'context-json': contextJson,
    policy: args['router-policy'] || args.router_policy || process.env.TOOL_CONTEXT_ROUTER_POLICY_PATH || '',
    apply: 0
  });
  if (!routeReceipt || routeReceipt.ok !== true || !routeReceipt.selected_tool) {
    return {
      ok: false,
      type: 'shadow_tool_notification_bridge',
      error: 'tool_route_failed',
      route: routeReceipt || null,
      policy_path: policyPath
    };
  }

  const message = cleanText(
    args.message
      || `shadow=${shadow} persona=${persona} selected_tool=${routeReceipt.selected_tool} objective=${routeReceipt.context?.objective || 'n/a'}`,
    2000
  );
  const severity = normalizeToken(args.severity || policy.notification.default_severity, 20) || 'info';
  const channel = normalizeToken(args.channel || policy.notification.default_channel, 40) || 'main';
  const topic = cleanText(args.topic || `shadow:${shadow}`, 120) || `shadow:${shadow}`;
  const apply = toBool(args.apply, true);
  const notifyReceipt = deliverNotification({
    policy: args['notify-policy'] || args.notify_policy || process.env.TOOL_NOTIFICATION_POLICY_PATH || '',
    channel,
    severity,
    topic,
    message,
    attempt: args.attempt || 0,
    apply: apply ? 1 : 0
  });
  if (!notifyReceipt || notifyReceipt.ok !== true) {
    return {
      ok: false,
      type: 'shadow_tool_notification_bridge',
      error: 'notification_lane_failed',
      route: routeReceipt,
      notification: notifyReceipt || null,
      policy_path: policyPath
    };
  }

  const receipt = {
    ok: true,
    type: 'shadow_tool_notification_bridge',
    ts: nowIso(),
    receipt_id: stableHash([
      'shadow_tool_notification_bridge',
      shadow,
      persona,
      routeReceipt.selected_tool,
      notifyReceipt.receipt_id
    ].join('|'), 24),
    policy_version: policy.version,
    policy_path: policyPath,
    shadow,
    persona,
    selected_tool: routeReceipt.selected_tool,
    route_receipt_id: routeReceipt.receipt_id,
    notification_receipt_id: notifyReceipt.receipt_id,
    escalation: notifyReceipt.escalation === true,
    apply
  };

  if (apply) {
    writeJsonAtomic(policy.paths.latest_path, receipt);
    appendJsonl(policy.paths.history_path, receipt);
  }
  return receipt;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'bridge', 40).toLowerCase();
  if (!['bridge', 'status'].includes(cmd)) {
    emit({ ok: false, error: `unknown_command:${cmd}` }, 1);
  }
  const policyPath = args.policy
    ? path.resolve(String(args.policy))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (cmd === 'status') {
    emit({
      ok: true,
      type: 'shadow_tool_notification_bridge_status',
      policy_version: policy.version,
      policy_path: policyPath,
      latest: readJson(policy.paths.latest_path, null),
      allowlist: policy.allowlist
    });
  }
  const receipt = bridge(args);
  emit(receipt, receipt.ok === true ? 0 : 1);
}

if (require.main === module) {
  main();
}

module.exports = {
  bridge,
  loadPolicy
};
