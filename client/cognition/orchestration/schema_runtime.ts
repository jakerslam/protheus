#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCHEMA_PATH = path.join(__dirname, 'schemas', 'finding-v1.json');

const SEVERITY_ORDER = Object.freeze({
  critical: 5,
  high: 4,
  medium: 3,
  low: 2,
  info: 1
});

const STATUS_ORDER = Object.freeze({
  confirmed: 5,
  open: 4,
  'needs-review': 3,
  resolved: 2,
  dismissed: 1
});

function loadFindingSchema() {
  return JSON.parse(fs.readFileSync(SCHEMA_PATH, 'utf8'));
}

function validateDateTime(value) {
  if (typeof value !== 'string' || !value.trim()) return false;
  const parsed = Date.parse(value);
  return Number.isFinite(parsed);
}

function validateFinding(finding) {
  if (!finding || typeof finding !== 'object' || Array.isArray(finding)) {
    return { ok: false, reason_code: 'finding_invalid_type' };
  }

  const required = ['audit_id', 'item_id', 'severity', 'status', 'location', 'evidence', 'timestamp'];
  for (const key of required) {
    if (!(key in finding)) {
      return { ok: false, reason_code: `finding_missing_${key}` };
    }
  }

  if (!SEVERITY_ORDER[String(finding.severity)]) {
    return { ok: false, reason_code: 'finding_invalid_severity' };
  }
  if (!STATUS_ORDER[String(finding.status)]) {
    return { ok: false, reason_code: 'finding_invalid_status' };
  }
  if (typeof finding.audit_id !== 'string' || !finding.audit_id.trim()) {
    return { ok: false, reason_code: 'finding_invalid_audit_id' };
  }
  if (typeof finding.item_id !== 'string' || !finding.item_id.trim()) {
    return { ok: false, reason_code: 'finding_invalid_item_id' };
  }
  if (typeof finding.location !== 'string' || !finding.location.trim()) {
    return { ok: false, reason_code: 'finding_invalid_location' };
  }
  if (!Array.isArray(finding.evidence) || finding.evidence.length < 1) {
    return { ok: false, reason_code: 'finding_invalid_evidence' };
  }
  for (const row of finding.evidence) {
    if (!row || typeof row !== 'object' || Array.isArray(row)) {
      return { ok: false, reason_code: 'finding_invalid_evidence_row' };
    }
    if (typeof row.type !== 'string' || !row.type.trim()) {
      return { ok: false, reason_code: 'finding_invalid_evidence_type' };
    }
    if (typeof row.value !== 'string' || !row.value.trim()) {
      return { ok: false, reason_code: 'finding_invalid_evidence_value' };
    }
  }
  if (!validateDateTime(finding.timestamp)) {
    return { ok: false, reason_code: 'finding_invalid_timestamp' };
  }

  return { ok: true, reason_code: 'finding_valid' };
}

function normalizeFinding(finding) {
  const cloned = Object.assign({}, finding);
  cloned.audit_id = String(cloned.audit_id || '').trim();
  cloned.item_id = String(cloned.item_id || '').trim();
  cloned.severity = String(cloned.severity || '').trim().toLowerCase();
  cloned.status = String(cloned.status || '').trim().toLowerCase();
  cloned.location = String(cloned.location || '').trim();
  cloned.timestamp = String(cloned.timestamp || '').trim() || new Date().toISOString();
  cloned.evidence = Array.isArray(cloned.evidence)
    ? cloned.evidence.map((row) => ({
      type: String(row && row.type ? row.type : '').trim(),
      value: String(row && row.value ? row.value : '').trim(),
      source: row && row.source ? String(row.source).trim() : undefined
    }))
    : [];
  return cloned;
}

module.exports = {
  ROOT,
  SCHEMA_PATH,
  SEVERITY_ORDER,
  STATUS_ORDER,
  loadFindingSchema,
  validateFinding,
  normalizeFinding
};
