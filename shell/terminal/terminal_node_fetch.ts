#!/usr/bin/env node
import http from 'node:http';
import https from 'node:https';

type FetchInit = Record<string, unknown>;

function headerValue(headers: unknown, key: string): string {
  if (!headers || typeof headers !== 'object') return '';
  const direct = (headers as Record<string, unknown>)[key] || (headers as Record<string, unknown>)[key.toLowerCase()];
  return String(direct == null ? '' : direct);
}

export function terminalNodeFetch(input: string, init: FetchInit = {}): Promise<any> {
  return new Promise((resolve, reject) => {
    const url = new URL(input);
    const body = String(init.body == null ? '' : init.body);
    const headers = {
      accept: headerValue(init.headers, 'accept') || 'application/json',
      'content-type': headerValue(init.headers, 'content-type') || 'application/json',
      'content-length': Buffer.byteLength(body),
      connection: 'close',
    };
    const request = (url.protocol === 'https:' ? https : http).request({
      method: String(init.method || 'GET'),
      hostname: url.hostname,
      port: url.port,
      path: `${url.pathname}${url.search}`,
      headers,
      agent: false,
    }, (response) => {
      const chunks: Buffer[] = [];
      response.on('data', (chunk) => chunks.push(Buffer.from(chunk)));
      response.on('end', () => {
        const text = Buffer.concat(chunks).toString('utf8');
        resolve({ ok: (response.statusCode || 0) < 400, status: response.statusCode || 0, text: async () => text });
      });
    });
    request.on('error', reject);
    if (body) request.write(body);
    request.end();
  });
}
