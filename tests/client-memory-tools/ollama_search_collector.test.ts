#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const http = require('http');

const ROOT = path.resolve(__dirname, '../..');

function startServer() {
  let flakyCalls = 0;
  const server = http.createServer((req, res) => {
    if (req.url === '/flaky-rss') {
      flakyCalls += 1;
      if (flakyCalls < 3) {
        res.writeHead(500, { 'content-type': 'text/plain' });
        res.end('temporary');
        return;
      }
      res.writeHead(200, { 'content-type': 'application/rss+xml' });
      res.end(
        '<?xml version="1.0"?>' +
          '<rss><channel>' +
          '<item><title>Alpha Signal</title><link>https://example.com/a</link><description>agent breakthrough</description></item>' +
          '</channel></rss>'
      );
      return;
    }
    if (req.url === '/json') {
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(
        JSON.stringify({
          rows: [{ title: 'Model One', url: 'https://example.com/model-one', description: 'edge ready', signal: true }]
        })
      );
      return;
    }
    if (req.url === '/ollama-tags') {
      res.writeHead(200, { 'content-type': 'application/json' });
      res.end(
        JSON.stringify({
          models: [
            {
              name: 'qwen2.5-coder:14b',
              modified_at: '2026-03-15T00:00:00Z',
              size: 1024 * 1024 * 1024,
              details: { parameter_size: '14B', family: 'qwen2.5' }
            }
          ]
        })
      );
      return;
    }
    if (req.url === '/always-500') {
      res.writeHead(500, { 'content-type': 'text/plain' });
      res.end('nope');
      return;
    }
    res.writeHead(404, { 'content-type': 'text/plain' });
    res.end('missing');
  });
  return new Promise((resolve) => {
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      resolve({ server, port: addr.port });
    });
  });
}

async function main() {
  const bridgeOnly = process.argv.includes('--collector-bridge-only=1');
  const stateDir = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-collector-runtime-test-'));
  process.env.EYES_STATE_DIR = stateDir;
  process.env.EYES_COLLECTOR_ALLOW_DIRECT_FETCH_FALLBACK = '1';
  process.env.EYES_COLLECTOR_BACKOFF_BASE_MS = '5';
  process.env.EYES_COLLECTOR_BACKOFF_MAX_MS = '25';
  process.env.EYES_COLLECTOR_CIRCUIT_MS = '100';
  process.env.EYES_COLLECTOR_CIRCUIT_AFTER = '2';
  process.env.EYES_COLLECTOR_MIN_INTERVAL_MS = '1';

  // Set env before requiring the runtime so it uses isolated local state.
  const runtime = require(path.join(
    ROOT,
    'client/cognition/shared/adaptive/sensory/eyes/collectors/collector_runtime.ts'
  ));
  const ollama = require(path.join(
    ROOT,
    'client/cognition/shared/adaptive/sensory/eyes/collectors/ollama_search.ts'
  ));
  const github = require(path.join(
    ROOT,
    'client/cognition/shared/adaptive/sensory/eyes/collectors/github_repo.ts'
  ));
  const ollamaAdapter = require(path.join(
    ROOT,
    'adapters/cognition/collectors/ollama_search.ts'
  ));
  const ollamaShim = require(path.join(
    ROOT,
    'client/runtime/systems/sensory/eyes_collectors/ollama_search.ts'
  ));

  const { server, port } = await startServer();
  const flakyRss = `http://127.0.0.1:${port}/flaky-rss`;
  const jsonUrl = `http://127.0.0.1:${port}/json`;
  const ollamaTagsUrl = `http://127.0.0.1:${port}/ollama-tags`;
  const always500 = `http://127.0.0.1:${port}/always-500`;

  try {
    const extracted = ollama.extractOllamaModels({
      models: [{ name: 'qwen2.5-coder:14b', modified_at: '2026-03-15T00:00:00Z', size: 1024 * 1024 * 1024 }]
    });
    assert.strictEqual(Array.isArray(extracted), true);
    assert.strictEqual(extracted.length, 1);
    assert.strictEqual(extracted[0].title, 'qwen2.5-coder:14b');
    assert.strictEqual(extracted[0].signal, true);
    assert.strictEqual(typeof ollamaAdapter.run, 'function');
    assert.strictEqual(typeof ollamaAdapter.parseArgs, 'function');
    assert.strictEqual(typeof ollamaAdapter.extractOllamaModels, 'function');
    assert.strictEqual(typeof ollamaShim.parseArgs, 'function');
    assert.strictEqual(typeof ollamaShim.extractOllamaModels, 'function');
    assert.deepStrictEqual(
      ollamaAdapter.extractOllamaModels({
        models: [{ name: 'qwen2.5-coder:14b', modified_at: '2026-03-15T00:00:00Z', size: 1024 * 1024 * 1024 }]
      }),
      extracted
    );
    const parsedArgs = ollamaAdapter.parseArgs([
      '--force',
      '--max=5',
      '--min-hours=0',
      '--attempts=1',
      `--url=${ollamaTagsUrl}`,
    ]);
    assert.deepStrictEqual(parsedArgs, {
      force: true,
      maxItems: 5,
      minHours: 0,
      attempts: 1,
      url: ollamaTagsUrl,
    });

    if (bridgeOnly) {
      console.log(JSON.stringify({
        ok: true,
        type: 'ollama_search_collector_bridge_test',
        status: 'pass'
      }));
      return;
    }

    const fetched = await runtime.fetchTextWithAdaptiveControls('flaky_feed', flakyRss, {
      scope: 'sensory.collector.dynamic',
      caller: 'tests/ollama_search_collector',
      attempts: 3,
      baseBackoffMs: 5,
      maxBackoffMs: 20,
      minIntervalMs: 1,
      circuitAfterFailures: 3,
      circuitOpenMs: 100
    });
    assert.strictEqual(fetched.status, 200);
    assert.strictEqual(fetched.attempt, 3, 'flaky feed should recover on third attempt');
    assert.ok(String(fetched.text).includes('Alpha Signal'));

    const feedRun = await runtime.runFeedCollector({
      collectorId: 'test_feed',
      scope: 'sensory.collector.dynamic',
      caller: 'tests/ollama_search_collector',
      feedCandidates: [flakyRss],
      maxItems: 5,
      minHours: 0,
      force: true,
      attempts: 1,
      signalRegex: /alpha|agent/i,
      topics: ['ai', 'signals']
    });
    assert.strictEqual(feedRun.ok, true, JSON.stringify(feedRun));
    assert.ok(Array.isArray(feedRun.items));
    assert.ok(feedRun.items.length >= 1, 'expected at least one RSS item');
    assert.strictEqual(feedRun.items[0].signal, true);

    const cadenceRun = await runtime.runFeedCollector({
      collectorId: 'test_feed',
      scope: 'sensory.collector.dynamic',
      caller: 'tests/ollama_search_collector',
      feedCandidates: [flakyRss],
      maxItems: 5,
      minHours: 24,
      force: false
    });
    assert.strictEqual(cadenceRun.ok, true);
    assert.strictEqual(cadenceRun.skipped, true, 'second run should honor cadence gate');

    const jsonRun = await runtime.runJsonCollector({
      collectorId: 'json_feed',
      scope: 'sensory.collector.dynamic',
      caller: 'tests/ollama_search_collector',
      url: jsonUrl,
      maxItems: 5,
      minHours: 0,
      force: true,
      extractor: (payload) => payload.rows || []
    });
    assert.strictEqual(jsonRun.ok, true, JSON.stringify(jsonRun));
    assert.ok(Array.isArray(jsonRun.items));
    assert.strictEqual(jsonRun.items.length, 1);
    assert.strictEqual(jsonRun.items[0].title, 'Model One');

    let firstErr = null;
    try {
      await runtime.fetchTextWithAdaptiveControls('breaker_feed', always500, {
        scope: 'sensory.collector.dynamic',
        caller: 'tests/ollama_search_collector',
        attempts: 1,
        baseBackoffMs: 5,
        maxBackoffMs: 15,
        minIntervalMs: 1,
        circuitAfterFailures: 1,
        circuitOpenMs: 5000
      });
    } catch (err) {
      firstErr = err;
    }
    assert.ok(firstErr, 'first breaker request should fail');

    let secondErr = null;
    try {
      await runtime.fetchTextWithAdaptiveControls('breaker_feed', always500, {
        scope: 'sensory.collector.dynamic',
        caller: 'tests/ollama_search_collector',
        attempts: 1,
        baseBackoffMs: 5,
        maxBackoffMs: 15,
        minIntervalMs: 1,
        circuitAfterFailures: 1,
        circuitOpenMs: 5000
      });
    } catch (err) {
      secondErr = err;
    }
    assert.ok(secondErr, 'second breaker request should fail due open circuit');
    assert.strictEqual(secondErr.code, 'rate_limited');

    const rateStatePath = path.join(stateDir, 'collector_rate_state.json');
    assert.ok(fs.existsSync(rateStatePath), 'rate state should persist to disk');

    // GitHub runtime: mock network to prove PR review + repo activity lanes.
    const previousFetch = global.fetch;
    global.fetch = async (url) => {
      const target = String(url || '');
      if (target.includes('/pulls/7/files')) {
        return {
          status: 200,
          text: async () => JSON.stringify([
            { filename: 'src/security/auth.rs', status: 'modified', additions: 40, deletions: 10, changes: 50 },
            { filename: 'schema/migrations/20260315.sql', status: 'added', additions: 20, deletions: 0, changes: 20 }
          ])
        };
      }
      if (target.endsWith('/pulls/7')) {
        return {
          status: 200,
          text: async () => JSON.stringify({
            number: 7,
            title: 'Harden token policy',
            html_url: 'https://github.com/acme/demo/pull/7',
            state: 'open',
            draft: false,
            additions: 60,
            deletions: 10,
            changed_files: 2,
            user: { login: 'alice' },
            head: { sha: 'abc123' }
          })
        };
      }
      if (target.endsWith('/releases/latest')) {
        return {
          status: 200,
          text: async () => JSON.stringify({
            tag_name: 'v1.2.3',
            name: 'Release 1.2.3',
            html_url: 'https://github.com/acme/demo/releases/tag/v1.2.3',
            body: 'security fix',
            published_at: '2026-03-15T00:00:00Z',
            author: { login: 'bot' }
          })
        };
      }
      if (target.includes('/commits')) {
        return {
          status: 200,
          text: async () => JSON.stringify([
            {
              sha: 'c0ffee',
              html_url: 'https://github.com/acme/demo/commit/c0ffee',
              commit: { message: 'refactor auth checks', author: { name: 'bob', date: '2026-03-15T01:00:00Z' } }
            }
          ])
        };
      }
      if (target.includes('/pulls?state=open')) {
        return {
          status: 200,
          text: async () => JSON.stringify([
            { number: 11, title: 'Improve sandbox policy', html_url: 'https://github.com/acme/demo/pull/11', user: { login: 'eve' }, updated_at: '2026-03-15T02:00:00Z', draft: false }
          ])
        };
      }
      return { status: 404, text: async () => 'not_found' };
    };

    const prReview = await github.run({ owner: 'acme', repo: 'demo', pr: 7, githubAppInstallationToken: 'ghs_test' });
    assert.strictEqual(prReview.ok, true, JSON.stringify(prReview));
    assert.strictEqual(prReview.mode, 'pr_review');
    assert.strictEqual(prReview.auth_mode, 'github_app_installation_token');
    assert.ok(Array.isArray(prReview.review.risk_flags));
    assert.ok(prReview.review.risk_flags.includes('security_sensitive_paths'));
    assert.ok(prReview.review.risk_flags.includes('schema_or_data_migration'));

    const repoActivity = await github.run({ owner: 'acme', repo: 'demo', force: true, maxItems: 5, githubToken: 'ghp_test' });
    assert.strictEqual(repoActivity.ok, true, JSON.stringify(repoActivity));
    assert.strictEqual(repoActivity.mode, 'repo_activity');
    assert.strictEqual(repoActivity.auth_mode, 'pat');
    assert.ok(Array.isArray(repoActivity.items));
    assert.ok(repoActivity.items.some((row) => row.type === 'pull_request'));

    global.fetch = previousFetch;

    console.log(JSON.stringify({
      ok: true,
      type: 'collector_runtime_test',
      status: 'pass'
    }));
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
}

if (require.main === module) {
  main().catch((err) => {
    console.error(JSON.stringify({ ok: false, error: String(err && err.message || err) }));
    process.exit(1);
  });
}

module.exports = { main };
