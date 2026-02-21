#!/usr/bin/env node
/**
 * MoltStack Publisher
 * Publishes a post to The Protheus Codex
 * Usage: node publish.js '{"title":"...","content":"..."}'
 */

const { egressFetch, EgressGatewayError } = require('../../../lib/egress_gateway');
const { issueSecretHandle, resolveSecretHandle } = require('../../../lib/secret_broker');

function issueMoltstackHandle() {
  const issued = issueSecretHandle({
    secret_id: 'moltstack_api_key',
    scope: 'skill.moltstack.publish',
    caller: 'skills/moltstack/scripts/publish.js',
    ttl_sec: 600,
    reason: 'publish_post'
  });
  if (!issued || issued.ok !== true || !issued.handle) {
    throw new Error(`Missing MoltStack api_key (${issued && issued.error ? issued.error : 'unknown'})`);
  }
  return issued.handle;
}

function resolveMoltstackApiKey(handle) {
  const resolved = resolveSecretHandle(handle, {
    scope: 'skill.moltstack.publish',
    caller: 'skills/moltstack/scripts/publish.js'
  });
  if (!resolved || resolved.ok !== true || !resolved.value) {
    throw new Error(`MoltStack api_key handle resolve failed (${resolved && resolved.error ? resolved.error : 'unknown'})`);
  }
  return String(resolved.value).trim();
}

async function publishPost(apiKeyHandle, postData) {
  const payload = JSON.stringify({
    title: postData.title,
    content: postData.content,
    publishNow: true
  });
  const apiKey = resolveMoltstackApiKey(apiKeyHandle);
  let res;
  try {
    res = await egressFetch('https://moltstack.net/api/posts', {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${apiKey}`,
        'Content-Type': 'application/json',
      },
      body: payload
    }, {
      scope: 'skill.moltstack.publish',
      caller: 'skills/moltstack/scripts/publish.js',
      runtime_allowlist: ['moltstack.net'],
      timeout_ms: Number(process.env.MOLTSTACK_API_TIMEOUT_MS || 20000),
      meta: { action: 'publish_post' }
    });
  } catch (err) {
    if (err instanceof EgressGatewayError) {
      throw new Error(`Egress denied: ${err.details && err.details.code ? err.details.code : 'policy'}`);
    }
    throw err;
  }

  const data = await res.text();
  let response = null;
  try {
    response = data ? JSON.parse(data) : null;
  } catch {
    throw new Error('Invalid JSON response: ' + data);
  }
  if (!res.ok) {
    throw new Error((response && (response.error || response.message)) || `HTTP ${res.status}`);
  }
  if (!response || response.success !== true) {
    throw new Error((response && response.error) || 'Unknown error');
  }
  return response;
}

async function main() {
  // Parse arguments
  const args = process.argv.slice(2);
  if (args.length === 0) {
    console.error('Usage: node publish.js \'{"title":"...","content":"..."}\'');
    process.exit(1);
  }

  let postData;
  try {
    postData = JSON.parse(args[0]);
  } catch (err) {
    console.error('Error parsing post data:', err.message);
    process.exit(1);
  }

  if (!postData.title || !postData.content) {
    console.error('Error: title and content are required');
    process.exit(1);
  }

  const apiKeyHandle = issueMoltstackHandle();

  // Publish
  try {
    console.log(`Publishing: ${postData.title}`);
    const result = await publishPost(apiKeyHandle, postData);
    console.log('✓ Published successfully');
    console.log(`URL: ${result.post.url}`);
    console.log(`Published at: ${result.post.published_at}`);
    process.exit(0);
  } catch (err) {
    console.error('✗ Publish failed:', err.message);
    process.exit(1);
  }
}

main();
