#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');

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

function resolveApiBase() {
  const base = cleanText(process.env.MOLTBOOK_API_BASE || 'https://api.moltbook.com', 300);
  return base.replace(/\/+$/, '');
}

function buildHeaders(options = {}) {
  const headers = {
    accept: 'application/json',
    'user-agent': 'infring-moltbook-hot/1.0'
  };
  const directApiKey = cleanText(options.apiKey || process.env.MOLTBOOK_API_KEY || '', 256);
  if (directApiKey) headers.authorization = `Bearer ${directApiKey}`;
  const apiKeyHandle = cleanText(options.apiKeyHandle || '', 200);
  if (apiKeyHandle) headers['x-secret-handle'] = apiKeyHandle;
  return headers;
}

async function fetchJson(url, options = {}) {
  const timeoutMs = Math.max(2000, Math.min(30000, Number(options.timeoutMs || process.env.MOLTBOOK_HTTP_TIMEOUT_MS || 12000) || 12000));
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(new Error('timeout')), timeoutMs);
  try {
    const response = await fetch(url, {
      method: 'GET',
      headers: buildHeaders(options),
      signal: controller.signal
    });
    const text = await response.text();
    let payload = null;
    try {
      payload = text ? JSON.parse(text) : null;
    } catch {
      payload = null;
    }
    if (!response.ok) {
      const error = new Error(`moltbook_http_${response.status}`);
      error.code = response.status === 429 ? 'rate_limited' : response.status >= 500 ? 'http_5xx' : 'http_error';
      error.http_status = response.status;
      error.payload = payload;
      throw error;
    }
    return payload;
  } catch (error) {
    if (error && String(error.name) === 'AbortError') {
      const timeoutError = new Error('moltbook_timeout');
      timeoutError.code = 'timeout';
      throw timeoutError;
    }
    throw error;
  } finally {
    clearTimeout(timer);
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
  const url = `${resolveApiBase()}/v1/posts/hot?limit=${boundedLimit}`;
  const payload = await fetchJson(url, options);
  return normalizeHotPosts(payload).slice(0, boundedLimit);
}

module.exports = {
  moltbook_getHotPosts
};
