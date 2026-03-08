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

const DEFAULT_POLICY_PATH = process.env.ADAPTATION_REVIEW_BRIDGE_POLICY_PATH
  ? path.resolve(String(process.env.ADAPTATION_REVIEW_BRIDGE_POLICY_PATH))
  : path.join(ROOT, 'client', 'config', 'adaptation_review_bridge_policy.json');

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    high_impact_threshold: 0.7,
    require_shadow_persona_for_high_impact: true,
    paths: {
      latest_path: 'state/adaptive/review_bridge/latest.json',
      history_path: 'state/adaptive/review_bridge/history.jsonl',
      queue_path: 'state/adaptive/review_bridge/review_queue.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const paths = raw && raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 40) || '1.0',
    enabled: toBool(raw.enabled, true),
    high_impact_threshold: Number(clampInt(Math.round(Number(raw.high_impact_threshold) * 1000), 0, 1000, Math.round(base.high_impact_threshold * 1000)) / 1000),
    require_shadow_persona_for_high_impact: toBool(raw.require_shadow_persona_for_high_impact, true),
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      queue_path: resolvePath(paths.queue_path, base.paths.queue_path)
    }
  };
}

function submitReview(args: AnyObj = {}) {
  const policyPath = args.policy
    ? path.resolve(String(args.policy))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) {
    return {
      ok: false,
      type: 'adaptation_review_bridge',
      error: 'review_bridge_disabled',
      policy_path: policyPath
    };
  }

  const riskScore = Number(clampInt(Math.round(Number(args['risk-score'] ?? args.risk_score ?? 0) * 1000), 0, 1000, 0) / 1000);
  const shadow = normalizeToken(args.shadow || '', 120);
  const persona = normalizeToken(args.persona || '', 120);
  const adaptationReceiptId = cleanText(args['adaptation-receipt-id'] || args.adaptation_receipt_id || '', 180)
    || cleanText(args['receipt-id'] || args.receipt_id || '', 180)
    || null;
  const summary = cleanText(args.summary || '', 500) || null;
  const apply = toBool(args.apply, true);

  const highImpact = riskScore >= policy.high_impact_threshold;
  if (highImpact && policy.require_shadow_persona_for_high_impact && (!shadow || !persona)) {
    return {
      ok: false,
      type: 'adaptation_review_bridge',
      error: 'missing_shadow_or_persona_for_high_impact',
      risk_score: riskScore,
      threshold: policy.high_impact_threshold,
      policy_path: policyPath
    };
  }

  const reviewStatus = highImpact ? 'pending_review' : 'auto_approved';
  const receipt = {
    ok: true,
    type: 'adaptation_review_bridge',
    ts: nowIso(),
    receipt_id: stableHash([
      'adaptation_review_bridge',
      String(adaptationReceiptId || ''),
      String(riskScore),
      String(reviewStatus),
      String(shadow || ''),
      String(persona || '')
    ].join('|'), 24),
    policy_version: policy.version,
    policy_path: policyPath,
    adaptation_receipt_id: adaptationReceiptId,
    risk_score: riskScore,
    high_impact_threshold: policy.high_impact_threshold,
    high_impact: highImpact,
    review_status: reviewStatus,
    shadow: shadow || null,
    persona: persona || null,
    summary,
    apply
  };

  if (apply) {
    writeJsonAtomic(policy.paths.latest_path, receipt);
    appendJsonl(policy.paths.history_path, receipt);
    if (highImpact) {
      appendJsonl(policy.paths.queue_path, {
        ts: receipt.ts,
        receipt_id: receipt.receipt_id,
        adaptation_receipt_id: adaptationReceiptId,
        shadow: receipt.shadow,
        persona: receipt.persona,
        risk_score: riskScore,
        status: reviewStatus,
        summary
      });
    }
  }
  return receipt;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'submit', 40).toLowerCase();
  if (!['submit', 'status'].includes(cmd)) {
    emit({ ok: false, error: `unknown_command:${cmd}` }, 1);
  }
  const policyPath = args.policy
    ? path.resolve(String(args.policy))
    : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (cmd === 'status') {
    emit({
      ok: true,
      type: 'adaptation_review_bridge_status',
      policy_version: policy.version,
      policy_path: policyPath,
      latest: readJson(policy.paths.latest_path, null)
    });
  }
  const receipt = submitReview(args);
  emit(receipt, receipt.ok === true ? 0 : 1);
}

if (require.main === module) {
  main();
}

module.exports = {
  submitReview,
  loadPolicy
};
