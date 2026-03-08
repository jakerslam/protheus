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
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.TOOL_NOTIFICATION_POLICY_PATH
  ? path.resolve(String(process.env.TOOL_NOTIFICATION_POLICY_PATH))
  : path.join(ROOT, 'client', 'config', 'tool_notification_policy.json');

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    channels: {
      main: { enabled: true, retry_limit: 2, escalate_after: 1 },
      background: { enabled: true, retry_limit: 1, escalate_after: 2 }
    },
    paths: {
      latest_path: 'state/tools/notification_lane/latest.json',
      history_path: 'state/tools/notification_lane/history.jsonl',
      outbox_path: 'state/tools/notification_lane/outbox.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const channelsRaw = raw && raw.channels && typeof raw.channels === 'object'
    ? raw.channels
    : {};
  const channels: Record<string, AnyObj> = {};
  const sourceKeys = Object.keys(base.channels);
  for (const key of sourceKeys) {
    const row = channelsRaw[key] && typeof channelsRaw[key] === 'object'
      ? channelsRaw[key]
      : base.channels[key];
    channels[key] = {
      enabled: toBool(row.enabled, true),
      retry_limit: clampInt(row.retry_limit, 0, 12, base.channels[key].retry_limit),
      escalate_after: clampInt(row.escalate_after, 0, 12, base.channels[key].escalate_after)
    };
  }
  const paths = raw && raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || '1.0',
    enabled: toBool(raw.enabled, true),
    channels,
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      outbox_path: resolvePath(paths.outbox_path, base.paths.outbox_path)
    }
  };
}

function deliverNotification(args: AnyObj = {}) {
  const policyPath = args.policy
    ? path.resolve(String(args.policy))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) {
    return {
      ok: false,
      type: 'tool_notification',
      error: 'notification_lane_disabled',
      policy_path: policyPath
    };
  }
  const channel = normalizeToken(args.channel || 'main', 40) || 'main';
  const channelCfg = policy.channels[channel];
  if (!channelCfg || channelCfg.enabled !== true) {
    return {
      ok: false,
      type: 'tool_notification',
      error: 'channel_disabled_or_unknown',
      channel,
      policy_path: policyPath
    };
  }

  const severity = normalizeToken(args.severity || 'info', 20) || 'info';
  const message = cleanText(args.message || '', 2000);
  if (!message) {
    return {
      ok: false,
      type: 'tool_notification',
      error: 'missing_message',
      channel,
      policy_path: policyPath
    };
  }
  const topic = cleanText(args.topic || 'general', 120) || 'general';
  const attempt = clampInt(args.attempt, 0, 64, 0);
  const apply = toBool(args.apply, true);
  const retryable = attempt < channelCfg.retry_limit;
  const escalation = ['critical', 'urgent', 'high'].includes(severity) || attempt >= channelCfg.escalate_after;
  const action = retryable ? 'queue_retry' : 'finalized';

  const receipt = {
    ok: true,
    type: 'tool_notification',
    ts: nowIso(),
    receipt_id: stableHash([
      'tool_notification',
      channel,
      severity,
      topic,
      String(attempt),
      message
    ].join('|'), 24),
    policy_version: policy.version,
    policy_path: policyPath,
    channel,
    severity,
    topic,
    attempt,
    retryable,
    escalation,
    action,
    message,
    apply
  };

  if (apply) {
    writeJsonAtomic(policy.paths.latest_path, receipt);
    appendJsonl(policy.paths.history_path, receipt);
    appendJsonl(policy.paths.outbox_path, {
      ts: receipt.ts,
      receipt_id: receipt.receipt_id,
      channel,
      severity,
      topic,
      message,
      escalation,
      action
    });
  }
  return receipt;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'notify', 40).toLowerCase();
  if (!['notify', 'status'].includes(cmd)) {
    emit({ ok: false, error: `unknown_command:${cmd}` }, 1);
  }
  const policyPath = args.policy
    ? path.resolve(String(args.policy))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (cmd === 'status') {
    emit({
      ok: true,
      type: 'tool_notification_status',
      policy_version: policy.version,
      policy_path: policyPath,
      channels: policy.channels,
      latest: readJson(policy.paths.latest_path, null),
      outbox_path: path.relative(ROOT, policy.paths.outbox_path).replace(/\\/g, '/')
    });
  }
  const receipt = deliverNotification(args);
  emit(receipt, receipt.ok === true ? 0 : 1);
}

if (require.main === module) {
  main();
}

module.exports = {
  deliverNotification,
  loadPolicy
};
