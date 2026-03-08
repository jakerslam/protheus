#!/usr/bin/env node
'use strict';
export {};

const path = require('path');

const CANONICAL_PATHS = {
  client_local_root: 'client/runtime/local',
  client_state_root: 'client/runtime/local/state',
  client_internal_root: 'client/runtime/local/internal',
  core_local_root: 'core/local',
  core_state_root: 'core/local/state'
};

const LEGACY_SURFACES = ['state', 'client/state', 'local'];

function clean(v: unknown) {
  return String(v == null ? '' : v)
    .trim()
    .replace(/\\/g, '/')
    .replace(/^\/+/, '');
}

function normalizeForRoot(rootAbs: string, relPath: string) {
  const rootName = path.basename(rootAbs).toLowerCase();
  const rel = clean(relPath);
  if (!rel) return rel;
  if (rootName === 'client' && rel.startsWith('client/')) return rel.slice('client/'.length);
  if (rootName === 'core' && rel.startsWith('core/')) return rel.slice('core/'.length);
  return rel;
}

function resolveCanonical(rootAbs: string, relPath: string) {
  const normalized = normalizeForRoot(rootAbs, relPath);
  return path.join(rootAbs, normalized);
}

function resolveClientState(rootAbs: string, suffix = '') {
  return resolveCanonical(rootAbs, path.join(CANONICAL_PATHS.client_state_root, clean(suffix)));
}

function resolveCoreState(rootAbs: string, suffix = '') {
  return resolveCanonical(rootAbs, path.join(CANONICAL_PATHS.core_state_root, clean(suffix)));
}

module.exports = {
  CANONICAL_PATHS,
  LEGACY_SURFACES,
  normalizeForRoot,
  resolveCanonical,
  resolveClientState,
  resolveCoreState
};

