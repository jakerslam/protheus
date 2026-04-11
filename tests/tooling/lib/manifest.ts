#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();

export type ToolingGate = {
  id: string;
  owner: string;
  description: string;
  timeout_sec?: number;
  timeout_env?: string;
  defer_host_stall?: boolean;
  script?: string;
  command?: string[];
  artifact_paths?: string[];
};

export type ToolingGateRegistry = {
  version: string;
  gates: Record<string, Omit<ToolingGate, 'id'>>;
};

export type ToolingProfileManifest = {
  version: string;
  profiles: Record<
    string,
    {
      description: string;
      gate_ids: string[];
    }
  >;
};

export function readJson<T>(filePath: string): T {
  return JSON.parse(fs.readFileSync(path.resolve(ROOT, filePath), 'utf8')) as T;
}

export function loadGateRegistry(filePath: string): ToolingGateRegistry {
  return readJson<ToolingGateRegistry>(filePath);
}

export function loadVerifyProfiles(filePath: string): ToolingProfileManifest {
  return readJson<ToolingProfileManifest>(filePath);
}

