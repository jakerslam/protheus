#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';

function cleanText(value: unknown, maxLen = 2000): string {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
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
  const policy = JSON.parse(
    fs.readFileSync(path.resolve(root, args.policyPath), 'utf8')
  ) as {
    promotion_rules?: Array<{ from: string; to: string }>;
  };
  const parsed = parseTag(args.sourceTag);
  const rules = Array.isArray(policy.promotion_rules) ? policy.promotion_rules : [];
  let ok = true;
  const errors: string[] = [];
  if (!parsed) {
    ok = false;
    errors.push('invalid_source_tag');
  }
  if (!args.targetChannel) {
    ok = false;
    errors.push('target_channel_required');
  }
  if (parsed) {
    const allowed = rules.some(
      (row) =>
        cleanText(row?.from ?? '', 40).toLowerCase() === parsed.channel &&
        cleanText(row?.to ?? '', 40).toLowerCase() === args.targetChannel
    );
    if (!allowed) {
      ok = false;
      errors.push(`promotion_not_allowed:${parsed.channel}->${args.targetChannel}`);
    }
  }
  const promotedTag = parsed ? nextTag(parsed.version, args.targetChannel || parsed.channel) : '';
  const report = {
    ok,
    type: 'release_channel_promote',
    source_tag: args.sourceTag,
    source_channel: parsed?.channel || '',
    target_channel: args.targetChannel,
    promoted_tag: promotedTag,
    errors,
  };
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
  if (args.strict && !ok) process.exitCode = 1;
}

main();
