#!/usr/bin/env node
'use strict';
export {};

const {
  nowIso,
  cleanText,
  stableHash,
  readJson,
  writeJsonAtomic
} = require('./_shared');

function loadContributions(policy: Record<string, any>) {
  const rows = readJson(policy.paths.contributions_path, []);
  return Array.isArray(rows) ? rows : [];
}

function saveContributions(policy: Record<string, any>, rows: any[]) {
  writeJsonAtomic(policy.paths.contributions_path, rows.slice(-5000));
}

function recordContribution(policy: Record<string, any>, input: Record<string, any>) {
  const donorId = cleanText(input.donor_id || input.donor || '', 120) || 'anonymous';
  const gpuHours = Math.max(0, Number(input.gpu_hours || input.hours || 0));
  const proofRef = cleanText(input.proof_ref || input.proof || '', 320) || 'unspecified';
  const contributionId = `gpu_${stableHash(`${donorId}|${gpuHours}|${proofRef}|${Date.now()}`, 18)}`;
  const row = {
    contribution_id: contributionId,
    donor_id: donorId,
    gpu_hours: Number(gpuHours.toFixed(6)),
    proof_ref: proofRef,
    received_at: nowIso(),
    status: 'received'
  };
  const rows = loadContributions(policy);
  rows.push(row);
  saveContributions(policy, rows);
  return row;
}

function updateContributionStatus(policy: Record<string, any>, contributionId: string, status: string, details: Record<string, any> = {}) {
  const rows = loadContributions(policy);
  let updated = null;
  const next = rows.map((row: any) => {
    if (String(row.contribution_id || '') !== String(contributionId || '')) return row;
    updated = {
      ...row,
      status: cleanText(status || 'unknown', 40) || 'unknown',
      status_updated_at: nowIso(),
      ...details
    };
    return updated;
  });
  saveContributions(policy, next);
  return updated;
}

module.exports = {
  loadContributions,
  recordContribution,
  updateContributionStatus
};
