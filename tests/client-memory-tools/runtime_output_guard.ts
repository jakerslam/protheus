#!/usr/bin/env node
'use strict';

const PLACEHOLDER_PATTERNS = [
  /<text response to user>/i,
  /actual concrete response text/i,
  /i don't have usable tool findings from this turn yet/i,
  /<\|begin[\s\S]*of[\s\S]*sentence\|>/i,
  /you are an expert python programmer/i,
  /\[patch\s+v\d+/i,
  /03-树2 list leaves/i,
  /<<<begin_openclaw_internal_context>>>/i,
  /<<<end_openclaw_internal_context>>>/i,
  /\[\[openclaw_internal_context_begin\]\]/i,
  /\[\[openclaw_internal_context_end\]\]/i,
  /openclaw runtime context \(internal\):/i,
  /\[internal task completion event\]/i,
  /<<<begin_untrusted_child_result>>>/i,
  /<<<end_untrusted_child_result>>>/i,
];

function collectStrings(value, out, depth, maxDepth, maxNodes, seen) {
  if (out.length >= maxNodes || depth > maxDepth) return;
  if (value == null) return;
  if (typeof value === 'string') {
    out.push(value);
    return;
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return;
  }
  if (typeof value !== 'object') {
    return;
  }
  if (seen.has(value)) {
    return;
  }
  seen.add(value);
  if (Array.isArray(value)) {
    for (const item of value) {
      collectStrings(item, out, depth + 1, maxDepth, maxNodes, seen);
      if (out.length >= maxNodes) break;
    }
    return;
  }
  for (const entry of Object.values(value)) {
    collectStrings(entry, out, depth + 1, maxDepth, maxNodes, seen);
    if (out.length >= maxNodes) break;
  }
}

function normalizedPreview(value) {
  return String(value || '')
    .replace(/[\u0000-\u001f\u007f]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, 180);
}

function assertNoPlaceholderOrPromptLeak(payload, label = 'runtime_output_guard') {
  const strings = [];
  collectStrings(payload, strings, 0, 8, 6000, new Set());
  for (const raw of strings) {
    const value = normalizedPreview(raw);
    if (!value) continue;
    for (const pattern of PLACEHOLDER_PATTERNS) {
      if (pattern.test(value)) {
        throw new Error('[' + label + '] detected placeholder/prompt-leak artifact: ' + value);
      }
    }
  }
}

function assertStableToolingEnvelope(payload, label = 'runtime_output_guard') {
  if (!payload || typeof payload !== 'object') {
    throw new Error('[' + label + '] expected object payload for tooling envelope guard');
  }
  const json = JSON.stringify(payload);
  if (!json || json === '{}' || json === '[]') {
    throw new Error('[' + label + '] empty tooling envelope payload');
  }
  const lowered = json.toLowerCase();
  const failed = lowered.includes('"ok":false') || lowered.includes('"status":"error"');
  const hasReason =
    lowered.includes('reason') ||
    lowered.includes('error') ||
    lowered.includes('status') ||
    lowered.includes('degraded');
  if (failed && !hasReason) {
    throw new Error('[' + label + '] failed envelope missing reason/status fields');
  }
}

module.exports = {
  assertNoPlaceholderOrPromptLeak,
  assertStableToolingEnvelope,
};
