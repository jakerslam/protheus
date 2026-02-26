#!/usr/bin/env node
'use strict';
export {};

const crypto = require('crypto');

type AnyObj = Record<string, any>;

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v: unknown, maxLen = 220) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function planReweave(sentinel: AnyObj = {}, verifier: AnyObj = {}, policy: AnyObj = {}, opts: AnyObj = {}) {
  const tier = String(sentinel && sentinel.tier || 'clear');
  const shadowOnly = policy && policy.shadow_only !== false;
  const mismatches = Array.isArray(verifier && verifier.mismatches) ? verifier.mismatches : [];
  const changedFiles = mismatches
    .map((row: AnyObj) => String(row && row.file || '').trim())
    .filter(Boolean);
  const planId = `rwv_${crypto.randomBytes(6).toString('hex')}`;
  const strategy = tier === 'confirmed_malice'
    ? 'full_restore_from_last_good_manifest'
    : (
      changedFiles.length
        ? 'targeted_strand_reweave'
        : 'noop_verify_only'
    );
  const steps: AnyObj[] = [];
  if (strategy === 'targeted_strand_reweave') {
    steps.push({ step: 'freeze_affected_lanes', apply: !shadowOnly, shadow: shadowOnly });
    steps.push({ step: 'restore_changed_files_from_signed_source', apply: !shadowOnly, shadow: shadowOnly });
    steps.push({ step: 'rebuild_helix_manifest', apply: !shadowOnly, shadow: shadowOnly });
    steps.push({ step: 'verify_attestation', apply: !shadowOnly, shadow: shadowOnly });
  } else if (strategy === 'full_restore_from_last_good_manifest') {
    steps.push({ step: 'global_actuation_freeze', apply: !shadowOnly, shadow: shadowOnly });
    steps.push({ step: 'restore_full_protected_scope', apply: !shadowOnly, shadow: shadowOnly });
    steps.push({ step: 'reseed_codex_chain', apply: !shadowOnly, shadow: shadowOnly });
    steps.push({ step: 'reverify_and_resume_by_policy', apply: !shadowOnly, shadow: shadowOnly });
  } else {
    steps.push({ step: 'verify_only', apply: false, shadow: true });
  }
  return {
    ok: true,
    type: 'helix_reweave_plan',
    ts: nowIso(),
    plan_id: planId,
    strategy,
    tier,
    shadow_only: shadowOnly,
    reason: cleanText(opts.reason || '', 180) || null,
    changed_files: changedFiles.slice(0, 5000),
    steps
  };
}

module.exports = {
  planReweave
};
