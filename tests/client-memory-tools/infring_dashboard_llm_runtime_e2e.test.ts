#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

// PARTS_LOADER: split oversized file into <=1000-line wrapper + parts.
const PARTS_DIR = [
  `${__filename}.parts`,
  `${__filename.replace(/\.js$/i, '.ts')}.parts`,
].find((candidate) => fs.existsSync(candidate)) || `${__filename}.parts`;
const source = fs
  .readdirSync(PARTS_DIR, { withFileTypes: true })
  .filter((entry) => entry.isFile() && /\.ts$/i.test(entry.name))
  .map((entry) => entry.name)
  .sort((a, b) => a.localeCompare(b, 'en'))
  .map((name) => fs.readFileSync(path.join(PARTS_DIR, name), 'utf8'))
  .join('\n');
assertNoPlaceholderOrPromptLeak({ source }, 'infring_dashboard_llm_runtime_e2e_test');
assertStableToolingEnvelope(
  { status: 'ok', source_length: source.length, parts_dir: PARTS_DIR },
  'infring_dashboard_llm_runtime_e2e_test'
);

module._compile(source, __filename);
