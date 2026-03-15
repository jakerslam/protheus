#!/usr/bin/env node
'use strict';

const crypto = require('node:crypto');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function sha16(value) {
  return crypto.createHash('sha256').update(String(value == null ? '' : value), 'utf8').digest('hex').slice(0, 16);
}

function normalizeTags(rawTags = []) {
  const out = [];
  const defaults = ['conversation', 'decision', 'insight', 'directive', 't1'];
  for (const raw of defaults.concat(Array.isArray(rawTags) ? rawTags : [])) {
    const tag = cleanText(raw, 32).toLowerCase();
    if (!tag || out.includes(tag)) continue;
    out.push(tag);
  }
  return out.slice(0, 12);
}

function inferLevel(input = {}) {
  if (Number.isFinite(Number(input.level))) {
    return Math.max(1, Math.min(3, Number(input.level)));
  }
  const priority = cleanText(input.priority || input.severity, 16).toLowerCase();
  if (priority === 'high' || priority === 'critical') return 1;
  if (priority === 'medium') return 2;
  return 3;
}

function levelToken(level) {
  if (level <= 1) return 'jot1';
  if (level === 2) return 'jot2';
  return 'jot3';
}

function synthesizeEnvelope(row = {}) {
  const base = row && typeof row === 'object' ? row : {};
  const date = cleanText(base.date || base.ts || base.timestamp || nowIso(), 32) || nowIso();
  const title = cleanText(base.title || base.subject || base.topic || 'Conversation Eye insight', 180);
  const preview = cleanText(base.preview || base.summary || base.message || base.content || title, 320);
  const nodeKind = cleanText(base.node_kind || base.kind || 'insight', 32).toLowerCase() || 'insight';
  const level = inferLevel(base);
  const nodeTags = normalizeTags(base.node_tags || base.tags || []);

  const stableSeed = JSON.stringify({
    date,
    title,
    preview,
    nodeKind,
    nodeTags
  });
  const nodeId = cleanText(base.node_id, 120) || `conversation-eye-${sha16(stableSeed)}`;
  const hexId = cleanText(base.hex_id, 24) || sha16(`${nodeId}|${date}`);

  return {
    ts: cleanText(base.ts || nowIso(), 32) || nowIso(),
    date: cleanText(date, 20),
    node_id: nodeId,
    hex_id: hexId,
    node_kind: nodeKind,
    level,
    level_token: cleanText(base.level_token || levelToken(level), 16) || levelToken(level),
    node_tags: nodeTags,
    edges_to: Array.isArray(base.edges_to) ? base.edges_to.slice(0, 12) : [],
    title,
    preview,
    xml: cleanText(
      base.xml || `<conversation-node id="${nodeId}" kind="${nodeKind}" level="${level}"><title>${title}</title><preview>${preview}</preview></conversation-node>`,
      1600
    )
  };
}

module.exports = {
  synthesizeEnvelope
};
