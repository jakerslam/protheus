#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');

// PARTS_LOADER: split oversized file into <=1000-line wrapper + parts.
const PARTS_DIR = `${__filename}.parts`;
const source = fs
  .readdirSync(PARTS_DIR, { withFileTypes: true })
  .filter((entry) => entry.isFile() && /\.ts$/i.test(entry.name))
  .map((entry) => entry.name)
  .sort((a, b) => a.localeCompare(b, 'en'))
  .map((name) => fs.readFileSync(path.join(PARTS_DIR, name), 'utf8'))
  .join('\n');

module._compile(source, __filename);
