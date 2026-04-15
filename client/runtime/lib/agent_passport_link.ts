'use strict';
export {};

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { recordIterationStep } = require('./passport_iteration_chain.ts');

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function resolveAutoLinkEnabled() {
  const preferred = String(process.env.INFRING_AGENT_PASSPORT_AUTOLINK || '').trim();
  const legacy = String(process.env.PROTHEUS_AGENT_PASSPORT_AUTOLINK || '').trim();
  if (!preferred && legacy) {
    process.env.INFRING_AGENT_PASSPORT_AUTOLINK = legacy;
  } else if (preferred && !legacy) {
    process.env.PROTHEUS_AGENT_PASSPORT_AUTOLINK = preferred;
  }
  const finalValue = preferred || legacy || String(process.env.AGENT_PASSPORT_AUTOLINK || '1').trim();
  return finalValue !== '0';
}

function linkReceiptToPassport(filePath, receiptRecord) {
  const autoLink = resolveAutoLinkEnabled();
  if (!autoLink) return null;
  try {
    return recordIterationStep({
      lane: 'action_receipts',
      step: 'receipt_link',
      target_path: cleanText(filePath, 360) || null,
      metadata: {
        receipt_path: cleanText(filePath, 360) || null,
        receipt_record: receiptRecord && typeof receiptRecord === 'object' ? receiptRecord : null
      }
    });
  } catch {
    return null;
  }
}

module.exports = {
  linkReceiptToPassport
};
