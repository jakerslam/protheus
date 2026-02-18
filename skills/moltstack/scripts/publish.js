#!/usr/bin/env node
/**
 * MoltStack Publisher
 * Publishes a post to The Protheus Codex
 * Usage: node publish.js '{"title":"...","content":"..."}'
 */

const fs = require('fs');
const path = require('path');
const https = require('https');

// Load credentials
const credentialsPath = path.join(process.env.HOME, '.config', 'moltstack', 'credentials.json');

function loadCredentials() {
  try {
    const data = fs.readFileSync(credentialsPath, 'utf8');
    return JSON.parse(data);
  } catch (err) {
    console.error('Error loading credentials:', err.message);
    console.error('Expected at:', credentialsPath);
    process.exit(1);
  }
}

function publishPost(credentials, postData) {
  return new Promise((resolve, reject) => {
    const payload = JSON.stringify({
      title: postData.title,
      content: postData.content,
      publishNow: true
    });

    const options = {
      hostname: 'moltstack.net',
      port: 443,
      path: '/api/posts',
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${credentials.api_key}`,
        'Content-Type': 'application/json',
        'Content-Length': Buffer.byteLength(payload)
      }
    };

    const req = https.request(options, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        try {
          const response = JSON.parse(data);
          if (response.success) {
            resolve(response);
          } else {
            reject(new Error(response.error || 'Unknown error'));
          }
        } catch (e) {
          reject(new Error('Invalid JSON response: ' + data));
        }
      });
    });

    req.on('error', (err) => reject(err));
    req.write(payload);
    req.end();
  });
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

  // Load credentials
  const credentials = loadCredentials();

  // Publish
  try {
    console.log(`Publishing: ${postData.title}`);
    const result = await publishPost(credentials, postData);
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
