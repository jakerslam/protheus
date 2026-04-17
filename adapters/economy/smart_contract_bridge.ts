#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const MAX_ID_LEN = 128;
const MAX_RATE = 1;
const MIN_RATE = 0;

function nowIso() {
  return new Date().toISOString();
}

function stableHash(v, len = 16) {
  return crypto.createHash('sha256').update(String(v == null ? '' : v), 'utf8').digest('hex').slice(0, len);
}

function stableStringify(value) {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(',')}]`;
  }
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(',')}}`;
}

function sanitizeToken(value, maxLen = MAX_ID_LEN) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, maxLen);
}

function parseFiniteNumber(value, fallback = 0) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function clampRate(value) {
  return Math.max(MIN_RATE, Math.min(MAX_RATE, parseFiniteNumber(value, 0)));
}

function normalizeContributionPayload(payload = {}) {
  const donorId = sanitizeToken(payload.donor_id || 'unknown_donor');
  const contributionId = sanitizeToken(payload.contribution_id || `contrib_${stableHash(nowIso(), 12)}`);
  const validatedGpuHours = Math.max(0, parseFiniteNumber(payload.validated_gpu_hours, 0));
  return {
    donor_id: donorId || 'unknown_donor',
    contribution_id: contributionId || `contrib_${stableHash(nowIso(), 12)}`,
    effective_tithe_rate: clampRate(payload.effective_tithe_rate),
    discount_rate: clampRate(payload.discount_rate),
    validated_gpu_hours: validatedGpuHours
  };
}

function appendJsonl(filePath, row) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function mintTitheReceipt(policy, payload) {
  const normalized = normalizeContributionPayload(payload);
  const receiptBody = {
    donor_id: normalized.donor_id,
    contribution_id: normalized.contribution_id,
    effective_tithe_rate: normalized.effective_tithe_rate,
    discount_rate: normalized.discount_rate,
    gpu_hours: normalized.validated_gpu_hours,
    chain: 'sovereign_bridge_stub'
  };
  const receipt = {
    ts: nowIso(),
    type: 'tithe_chain_receipt',
    receipt_id: `chain_${stableHash(stableStringify(receiptBody), 18)}`,
    receipt_hash: stableHash(stableStringify(receiptBody), 32),
    ...receiptBody
  };
  appendJsonl(policy.paths.chain_receipts_path, receipt);
  return receipt;
}

module.exports = {
  mintTitheReceipt,
  normalizeContributionPayload
};
