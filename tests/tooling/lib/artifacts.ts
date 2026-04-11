#!/usr/bin/env node
'use strict';

import path from 'node:path';

const ROOT = process.cwd();

export function toolingArtifactRoot(): string {
  return path.join(ROOT, 'core', 'local', 'artifacts', 'tooling_runs');
}

export function toolingHistoryPath(): string {
  return path.join(ROOT, 'local', 'state', 'ops', 'tooling_runs', 'history.jsonl');
}

export function toolingLatestPath(kind: 'gate' | 'profile', id: string): string {
  const safeId = String(id || 'unknown')
    .replace(/[^a-zA-Z0-9_.:-]+/g, '_')
    .slice(0, 120);
  return path.join(toolingArtifactRoot(), `${kind}_${safeId}_latest.json`);
}

