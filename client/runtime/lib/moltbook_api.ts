#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');

const adapterApi = require(path.resolve(
  __dirname,
  '..',
  '..',
  '..',
  'adapters',
  'cognition',
  'skills',
  'moltbook',
  'moltbook_api.ts'
));

function cleanText(value, maxLen = 240) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function clampLimit(value, fallback = 20) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(1, Math.min(100, Math.floor(parsed)));
}

function readFixturePayload() {
  const fixturePath = cleanText(process.env.MOLTBOOK_HOT_POSTS_FIXTURE || '', 600);
  if (!fixturePath) return null;
  const resolved = path.resolve(fixturePath);
  if (!fs.existsSync(resolved)) return null;
  try {
    return JSON.parse(fs.readFileSync(resolved, 'utf8'));
  } catch {
    return null;
  }
}

function normalizeHotPosts(payload) {
  if (Array.isArray(payload)) return payload;
  if (payload && Array.isArray(payload.posts)) return payload.posts;
  if (payload && payload.data && Array.isArray(payload.data.posts)) return payload.data.posts;
  return [];
}

async function moltbook_getHotPosts(limit = 20, options = {}) {
  const boundedLimit = clampLimit(limit, 20);
  const fixture = readFixturePayload();
  if (fixture) return normalizeHotPosts(fixture).slice(0, boundedLimit);

  const rawOptions =
    options && typeof options === 'object' && !Array.isArray(options)
      ? { ...options }
      : options;
  if (rawOptions && typeof rawOptions === 'object' && !Array.isArray(rawOptions)) {
    if (!rawOptions.apiKey) {
      const envApiKey = cleanText(process.env.MOLTBOOK_API_KEY || '', 256);
      if (envApiKey) rawOptions.apiKey = envApiKey;
    }
    if (!rawOptions.apiKeyHandle) {
      const envHandle = cleanText(process.env.MOLTBOOK_API_KEY_HANDLE || '', 200);
      if (envHandle) rawOptions.apiKeyHandle = envHandle;
    }
  }

  return adapterApi.moltbook_getHotPosts(boundedLimit, rawOptions);
}

module.exports = {
  ...adapterApi,
  moltbook_getHotPosts
};
