#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type Band = {
  id: string;
  action: string;
  min_utilization?: number;
  max_utilization?: number;
};

type ReceiptContract = {
  band: string;
  receipt_type: string;
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/queue_backpressure_policy_gate_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    policyPath: cleanText(
      readFlag(argv, 'policy') || 'client/runtime/config/queue_backpressure_policy.json',
      400,
    ),
  };
}

function safeNumber(value: unknown, fallback = NaN): number {
  const num = Number(value);
  return Number.isFinite(num) ? num : fallback;
}

function toBand(raw: any): Band {
  return {
    id: cleanText(String(raw?.id || ''), 80),
    action: cleanText(String(raw?.action || ''), 120),
    min_utilization:
      raw?.min_utilization == null ? undefined : safeNumber(raw?.min_utilization),
    max_utilization:
      raw?.max_utilization == null ? undefined : safeNumber(raw?.max_utilization),
  };
}

function resolveBand(bands: Band[], utilization: number): Band | null {
  for (const band of bands) {
    const minOk =
      band.min_utilization == null || Number.isNaN(band.min_utilization)
        ? true
        : utilization >= band.min_utilization;
    const maxOk =
      band.max_utilization == null || Number.isNaN(band.max_utilization)
        ? true
        : utilization <= band.max_utilization;
    if (minOk && maxOk) return band;
  }
  return null;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const policyAbs = path.resolve(root, args.policyPath);
  let policy: any = null;
  try {
    policy = JSON.parse(fs.readFileSync(policyAbs, 'utf8'));
  } catch (error) {
    const payload = {
      ok: false,
      type: 'queue_backpressure_policy_gate',
      error: 'queue_backpressure_policy_unavailable',
      detail: cleanText(error instanceof Error ? error.message : String(error), 320),
      policy_path: args.policyPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const rawBands = Array.isArray(policy?.utilization_bands) ? policy.utilization_bands : [];
  const bands = rawBands.map(toBand).filter((band) => band.id && band.action);
  const receiptContracts: ReceiptContract[] = Array.isArray(policy?.receipt_contracts)
    ? policy.receipt_contracts
        .map((row: any) => ({
          band: cleanText(String(row?.band || ''), 80),
          receipt_type: cleanText(String(row?.receipt_type || ''), 160),
        }))
        .filter((row: ReceiptContract) => row.band && row.receipt_type)
    : [];
  const receiptTypeByBand = new Map<string, string>();
  for (const row of receiptContracts) {
    if (!receiptTypeByBand.has(row.band)) {
      receiptTypeByBand.set(row.band, row.receipt_type);
    }
  }
  const failures: Array<{ id: string; detail: string }> = [];

  const requiredBandIds = ['healthy', 'defer', 'shed', 'quarantine'];
  const requiredActions = ['none', 'defer_noncritical', 'shed_noncritical', 'quarantine_new_ingress'];
  const requiredReceiptsByBand: Record<string, string> = {
    healthy: 'queue_backpressure_healthy_receipt',
    defer: 'queue_backpressure_defer_receipt',
    shed: 'queue_backpressure_shed_receipt',
    quarantine: 'queue_backpressure_quarantine_receipt',
  };
  for (const id of requiredBandIds) {
    if (!bands.some((band) => band.id === id)) {
      failures.push({ id: 'queue_backpressure_missing_band', detail: id });
    }
    const expectedReceipt = requiredReceiptsByBand[id];
    const actualReceipt = cleanText(receiptTypeByBand.get(id) || '', 160);
    if (!actualReceipt) {
      failures.push({
        id: 'queue_backpressure_missing_receipt_contract',
        detail: id,
      });
    } else if (actualReceipt !== expectedReceipt) {
      failures.push({
        id: 'queue_backpressure_receipt_contract_mismatch',
        detail: `${id}:expected=${expectedReceipt};actual=${actualReceipt}`,
      });
    }
  }
  for (const action of requiredActions) {
    if (!bands.some((band) => band.action === action)) {
      failures.push({ id: 'queue_backpressure_missing_action', detail: action });
    }
  }

  for (const band of bands) {
    if (
      band.min_utilization != null &&
      band.max_utilization != null &&
      !Number.isNaN(band.min_utilization) &&
      !Number.isNaN(band.max_utilization) &&
      band.min_utilization > band.max_utilization
    ) {
      failures.push({
        id: 'queue_backpressure_band_range_invalid',
        detail: `${band.id}:${band.min_utilization}>${band.max_utilization}`,
      });
    }
  }

  const expectations = Array.isArray(policy?.deterministic_expectations)
    ? policy.deterministic_expectations
    : [];
  const expectationChecks = expectations.map((row: any) => {
    const utilization = safeNumber(row?.utilization, NaN);
    const expectedBand = cleanText(String(row?.expected_band || ''), 80);
    const expectedAction = cleanText(String(row?.expected_action || ''), 120);
    const expectedReceiptType = cleanText(String(row?.expected_receipt_type || ''), 160);
    const resolved = resolveBand(bands, utilization);
    const actualReceiptType = resolved ? cleanText(receiptTypeByBand.get(resolved.id) || '', 160) : '';
    const ok =
      Number.isFinite(utilization) &&
      !!resolved &&
      resolved.id === expectedBand &&
      resolved.action === expectedAction &&
      (!!expectedReceiptType ? actualReceiptType === expectedReceiptType : !!actualReceiptType);
    if (!ok) {
      failures.push({
        id: 'queue_backpressure_expectation_failed',
        detail: `u=${utilization};expected=${expectedBand}/${expectedAction}/${expectedReceiptType || 'missing'};actual=${resolved?.id || 'none'}/${resolved?.action || 'none'}/${actualReceiptType || 'missing'}`,
      });
    }
    return {
      utilization,
      expected_band: expectedBand,
      expected_action: expectedAction,
      expected_receipt_type: expectedReceiptType,
      actual_band: resolved?.id || '',
      actual_action: resolved?.action || '',
      actual_receipt_type: actualReceiptType,
      ok,
    };
  });
  for (const id of ['defer', 'shed', 'quarantine']) {
    const hasExpectation = expectationChecks.some(
      (row: any) =>
        row.expected_band === id &&
        !!cleanText(String(row.expected_receipt_type || ''), 160),
    );
    if (!hasExpectation) {
      failures.push({
        id: 'queue_backpressure_expectation_missing_receipt_coverage',
        detail: id,
      });
    }
  }

  const report = {
    ok: failures.length === 0,
    type: 'queue_backpressure_policy_gate',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    policy_path: args.policyPath,
    summary: {
      band_count: bands.length,
      receipt_contract_count: receiptContracts.length,
      expectation_count: expectationChecks.length,
      failed_count: failures.length,
      pass: failures.length === 0,
    },
    bands,
    receipt_contracts: receiptContracts,
    expectation_checks: expectationChecks,
    failures,
  };

  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
