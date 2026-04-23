#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';

function cleanText(value: unknown, maxLen = 2000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function isCanonicalChannelToken(value: string): boolean {
  return /^[a-z][a-z0-9-]{0,31}$/.test(value);
}

function isCanonicalVersionToken(value: string): boolean {
  return /^\d+\.\d+$/.test(value);
}

function toRepoRelative(root: string, target: string): string | null {
  const rel = path.relative(root, target).replace(/\\/g, '/');
  if (!rel || rel.startsWith('../') || rel === '..') return null;
  return rel;
}

function parseArgs(argv: string[]) {
  const out = {
    sourceTag: '',
    targetChannel: '',
    policyPath: 'client/runtime/config/release_channel_policy.json',
    strict: false,
  };
  for (const tokenRaw of argv) {
    const token = cleanText(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--source-tag=')) out.sourceTag = cleanText(token.slice(13), 120);
    else if (token.startsWith('--target-channel=')) out.targetChannel = cleanText(token.slice(17), 40).toLowerCase();
    else if (token.startsWith('--policy=')) out.policyPath = cleanText(token.slice(9), 400);
    else if (token.startsWith('--strict=')) {
      const raw = cleanText(token.slice(9), 20).toLowerCase();
      out.strict = raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
    }
  }
  return out;
}

function parseTag(tag: string): { version: string; channel: string } | null {
  const raw = cleanText(tag, 120).replace(/^v/i, '');
  const match = raw.match(/^(\d+\.\d+\.\d+)(?:-([a-z][a-z0-9.-]*))?$/i);
  if (!match) return null;
  return {
    version: match[1],
    channel: cleanText(match[2] || 'stable', 40).toLowerCase(),
  };
}

function nextTag(version: string, channel: string): string {
  if (channel === 'stable') return `v${version}`;
  return `v${version}-${channel}`;
}

function main() {
  const root = path.resolve(__dirname, '../../../..');
  const args = parseArgs(process.argv.slice(2));
  const policyFailures: string[] = [];
  const validationFailures: string[] = [];
  const policyPathResolved = path.resolve(root, args.policyPath);
  const policyPathRel = toRepoRelative(root, policyPathResolved);

  if (!args.policyPath) {
    policyFailures.push('policy_path_required');
  }
  if (!policyPathRel) {
    policyFailures.push('policy_path_outside_repo');
  }
  if (!policyPathResolved.toLowerCase().endsWith('.json')) {
    policyFailures.push('policy_path_must_be_json');
  }
  if (!fs.existsSync(policyPathResolved)) {
    policyFailures.push('policy_path_missing');
  }

  let policyRaw: unknown = {};
  if (policyFailures.length === 0) {
    try {
      policyRaw = JSON.parse(fs.readFileSync(policyPathResolved, 'utf8'));
    } catch {
      policyFailures.push('policy_json_parse_failed');
    }
  }

  const policyObj = policyRaw && typeof policyRaw === 'object' ? (policyRaw as Record<string, unknown>) : {};
  const schemaId = cleanText(policyObj.schema_id ?? '', 120);
  const schemaVersion = cleanText(policyObj.schema_version ?? '', 40);
  const defaultChannel = cleanText(policyObj.default_channel ?? '', 40).toLowerCase();
  const channelsRaw = Array.isArray(policyObj.channels) ? policyObj.channels : [];
  const rulesRaw = Array.isArray(policyObj.promotion_rules) ? policyObj.promotion_rules : [];

  if (schemaId !== 'release_channel_policy') {
    policyFailures.push('schema_id_invalid');
  }
  if (!isCanonicalVersionToken(schemaVersion)) {
    policyFailures.push('schema_version_invalid');
  }
  if (!isCanonicalChannelToken(defaultChannel)) {
    policyFailures.push('default_channel_invalid_token');
  }
  if (!Array.isArray(policyObj.channels) || channelsRaw.length === 0) {
    policyFailures.push('channels_missing_or_empty');
  }
  if (!Array.isArray(policyObj.promotion_rules) || rulesRaw.length === 0) {
    policyFailures.push('promotion_rules_missing_or_empty');
  }

  const channels = channelsRaw.map((entry) => cleanText(entry, 40).toLowerCase()).filter(Boolean);
  const channelSeen = new Set<string>();
  for (const channel of channels) {
    if (!isCanonicalChannelToken(channel)) {
      policyFailures.push(`channel_token_invalid:${channel}`);
      continue;
    }
    if (channelSeen.has(channel)) {
      policyFailures.push(`channel_duplicate:${channel}`);
      continue;
    }
    channelSeen.add(channel);
  }
  for (const required of ['alpha', 'beta', 'stable']) {
    if (!channelSeen.has(required)) {
      policyFailures.push(`required_channel_missing:${required}`);
    }
  }
  if (defaultChannel && !channelSeen.has(defaultChannel)) {
    policyFailures.push('default_channel_not_in_channels');
  }

  const rules = rulesRaw.map((row) =>
    row && typeof row === 'object' ? (row as Record<string, unknown>) : {}
  );
  const rulePairs = new Set<string>();
  for (let index = 0; index < rules.length; index += 1) {
    const row = rules[index];
    const from = cleanText(row.from ?? '', 40).toLowerCase();
    const to = cleanText(row.to ?? '', 40).toLowerCase();
    if (!isCanonicalChannelToken(from) || !isCanonicalChannelToken(to)) {
      policyFailures.push(`promotion_rule_token_invalid:${index}`);
      continue;
    }
    if (!channelSeen.has(from) || !channelSeen.has(to)) {
      policyFailures.push(`promotion_rule_channel_not_declared:${from}->${to}`);
      continue;
    }
    if (from === to) {
      policyFailures.push(`promotion_rule_self_promotion_forbidden:${from}`);
      continue;
    }
    const pair = `${from}->${to}`;
    if (rulePairs.has(pair)) {
      policyFailures.push(`promotion_rule_duplicate:${pair}`);
      continue;
    }
    if (from === 'stable') {
      policyFailures.push(`promotion_rule_stable_source_forbidden:${pair}`);
      continue;
    }
    rulePairs.add(pair);
  }
  for (const requiredPair of ['alpha->beta', 'beta->stable', 'alpha->stable']) {
    if (!rulePairs.has(requiredPair)) {
      policyFailures.push(`required_promotion_rule_missing:${requiredPair}`);
    }
  }

  const parsed = parseTag(args.sourceTag);
  if (!args.sourceTag) {
    validationFailures.push('source_tag_required');
  }
  let ok = true;
  if (!parsed) {
    ok = false;
    validationFailures.push('invalid_source_tag');
  }
  if (!args.targetChannel) {
    ok = false;
    validationFailures.push('target_channel_required');
  }
  if (args.targetChannel && !isCanonicalChannelToken(args.targetChannel)) {
    ok = false;
    validationFailures.push('target_channel_invalid_token');
  }
  if (args.targetChannel && channelSeen.size > 0 && !channelSeen.has(args.targetChannel)) {
    ok = false;
    validationFailures.push(`target_channel_not_declared:${args.targetChannel}`);
  }
  if (parsed && channelSeen.size > 0 && !channelSeen.has(parsed.channel)) {
    ok = false;
    validationFailures.push(`source_channel_not_declared:${parsed.channel}`);
  }
  if (parsed && args.targetChannel && parsed.channel === args.targetChannel) {
    ok = false;
    validationFailures.push('promotion_noop_forbidden');
  }
  if (parsed) {
    const allowed = rulePairs.has(`${parsed.channel}->${args.targetChannel}`);
    if (!allowed) {
      ok = false;
      validationFailures.push(`promotion_not_allowed:${parsed.channel}->${args.targetChannel}`);
    }
  }
  const promotedTag = parsed ? nextTag(parsed.version, args.targetChannel || parsed.channel) : '';
  if (promotedTag && !parseTag(promotedTag)) {
    ok = false;
    validationFailures.push('promoted_tag_invalid');
  }
  if (policyFailures.length > 0 || validationFailures.length > 0) {
    ok = false;
  }
  const errors = [...policyFailures, ...validationFailures];
  const totalIssueCount = errors.length;
  const report = {
    ok,
    type: 'release_channel_promote',
    strict_mode: args.strict,
    policy_path: policyPathRel || '',
    policy_schema_id: schemaId,
    policy_schema_version: schemaVersion,
    default_channel: defaultChannel,
    channels,
    allowed_promotion_pairs: Array.from(rulePairs).sort(),
    source_tag: args.sourceTag,
    source_channel: parsed?.channel || '',
    target_channel: args.targetChannel,
    promoted_tag: promotedTag,
    policy_failures: Array.from(new Set(policyFailures)),
    validation_failures: Array.from(new Set(validationFailures)),
    policy_failure_count: Array.from(new Set(policyFailures)).length,
    validation_failure_count: Array.from(new Set(validationFailures)).length,
    total_issue_count: Array.from(new Set(errors)).length,
    errors,
  };
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if ((args.strict || totalIssueCount > 0) && !ok) process.exitCode = 1;
}

main();
