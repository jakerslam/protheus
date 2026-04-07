#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

function nowIso() {
  return new Date().toISOString();
}

function stableHash(v, len = 16) {
  return crypto.createHash('sha256').update(String(v == null ? '' : v), 'utf8').digest('hex').slice(0, len);
}

function appendJsonl(filePath, row) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function mintTitheReceipt(policy, payload) {
  const receipt = {
    ts: nowIso(),
    type: 'tithe_chain_receipt',
    receipt_id: `chain_${stableHash(JSON.stringify(payload), 18)}`,
    donor_id: payload.donor_id,
    contribution_id: payload.contribution_id,
    effective_tithe_rate: payload.effective_tithe_rate,
    discount_rate: payload.discount_rate,
    gpu_hours: payload.validated_gpu_hours,
    chain: 'sovereign_bridge_stub'
  };
  appendJsonl(policy.paths.chain_receipts_path, receipt);
  return receipt;
}

module.exports = {
  mintTitheReceipt
};
