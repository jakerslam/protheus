#!/usr/bin/env tsx

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops (authoritative transport + receipts); this file is UI bridge/wrapper.

const { WebSocketServer } = require('ws');

function createAgentWsBridge({ flags, cleanText, fetchBackend, fetchBackendJson }) {
  const wss = new WebSocketServer({ noServer: true, clientTracking: false, perMessageDeflate: false });
  const route = /^\/api\/agents\/([^/]+)\/ws$/;
  const enc = (agentId) => encodeURIComponent(String(agentId || '').trim());
  const send = (ws, payload) => {
    try { if (ws && ws.readyState === 1) ws.send(JSON.stringify(payload)); } catch {}
  };
  const parseJson = (raw) => { try { return JSON.parse(raw); } catch { return null; } };
  const toNum = (value, fallback = 0) => Number.isFinite(Number(value)) ? Number(value) : fallback;
  const sendContext = async (ws, agentId) => {
    const agent = await fetchBackendJson(flags, `/api/agents/${enc(agentId)}`, 8000).catch(() => ({}));
    const contextWindow = toNum(agent.context_window || agent.context_window_tokens || 0, 0);
    send(ws, { type: 'context_state', agent_id: agentId, context_tokens: 0, context_window: contextWindow, context_ratio: 0, context_pressure: contextWindow > 0 ? 'normal' : '' });
    return agent;
  };
  wss.on('connection', (ws, _req, agentId) => {
    const targetAgent = cleanText(agentId || '', 180);
    let agentName = '';
    let chain = Promise.resolve();
    chain = chain.then(async () => {
      const agent = await sendContext(ws, targetAgent);
      agentName = cleanText(agent.name || '', 120);
      send(ws, { type: 'connected', agent_id: targetAgent, agent_name: agentName || '' });
    }).catch((error) => send(ws, { type: 'error', content: cleanText(error && error.message ? error.message : 'ws_connect_failed', 260), agent_id: targetAgent }));
    ws.on('message', (chunk) => {
      const raw = Buffer.isBuffer(chunk) ? chunk.toString('utf8') : String(chunk || '');
      chain = chain.then(async () => {
        const payload = parseJson(raw);
        if (!payload || typeof payload !== 'object') return;
        const msgType = cleanText(payload.type || '', 40).toLowerCase();
        if (!msgType || msgType === 'ping') { send(ws, { type: 'pong' }); return; }
        if (msgType === 'message') {
          const content = String(payload.content == null ? '' : payload.content).slice(0, 12000);
          if (!content.trim()) { send(ws, { type: 'error', content: 'message_required', agent_id: targetAgent }); return; }
          send(ws, { type: 'typing', state: 'start', agent_id: targetAgent });
          const res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/message`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ message: content, attachments: Array.isArray(payload.attachments) ? payload.attachments : [] }),
          }, 180000);
          const out = await res.json().catch(() => ({}));
          if (!res.ok || out.ok === false) {
            send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(out.error || `backend_http_${res.status}`, 260) });
            return;
          }
          send(ws, {
            type: 'response',
            agent_id: targetAgent,
            agent_name: agentName || cleanText(out.agent_name || '', 120) || '',
            content: String(out.response || out.content || ''),
            input_tokens: toNum(out.input_tokens || 0, 0),
            output_tokens: toNum(out.output_tokens || 0, 0),
            cost_usd: toNum(out.cost_usd || 0, 0),
            iterations: toNum(out.iterations || 1, 1),
            duration_ms: toNum(out.duration_ms || out.latency_ms || 0, 0),
            context_tokens: toNum(out.context_tokens || out.context_used_tokens || out.context_total_tokens || 0, 0),
            context_window: toNum(out.context_window || out.context_window_tokens || 0, 0),
            context_ratio: toNum(out.context_ratio || 0, 0),
            context_pressure: cleanText(out.context_pressure || '', 32),
            auto_route: out.auto_route || null,
          });
          return;
        }
        if (msgType === 'terminal') {
          const command = String(payload.command == null ? '' : payload.command).slice(0, 16000);
          if (!command.trim()) { send(ws, { type: 'terminal_error', agent_id: targetAgent, message: 'command_required' }); return; }
          const res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/terminal`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ command, cwd: cleanText(payload.cwd || '', 4000) }),
          }, 120000);
          const out = await res.json().catch(() => ({}));
          if (!res.ok || out.ok === false) {
            send(ws, { type: 'terminal_error', agent_id: targetAgent, message: cleanText(out.error || out.message || `backend_http_${res.status}`, 260) });
            return;
          }
          send(ws, { type: 'terminal_output', agent_id: targetAgent, stdout: String(out.stdout || ''), stderr: String(out.stderr || ''), exit_code: toNum(out.exit_code || 0, 0), duration_ms: toNum(out.duration_ms || 0, 0), cwd: cleanText(out.cwd || '', 4000) });
          return;
        }
        if (msgType === 'command') {
          const command = cleanText(payload.command || '', 80).toLowerCase();
          const silent = !!payload.silent;
          const res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/command`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ command, silent }),
          }, 12000);
          const out = await res.json().catch(() => ({}));
          if (!res.ok || out.ok === false) {
            const commandUnavailable = res.status === 404 || res.status === 405 || res.status === 501;
            if (commandUnavailable) {
              const ctx = await sendContext(ws, targetAgent).catch(() => null);
              const contextWindow = toNum(
                (ctx && (ctx.context_window || ctx.context_window_tokens)) ||
                  out.context_window ||
                  out.context_window_tokens ||
                  0,
                0
              );
              send(ws, {
                type: 'command_result',
                silent,
                agent_id: targetAgent,
                command: cleanText(command || 'unknown', 80),
                message: silent ? '' : 'Command unavailable on this runtime surface.',
                runtime_sync: null,
                context_window: contextWindow,
              });
              return;
            }
            send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(out.error || `backend_http_${res.status}`, 260) });
            return;
          }
          send(ws, {
            type: 'command_result',
            silent,
            agent_id: targetAgent,
            command: cleanText(out.command || command || 'unknown', 80),
            message: cleanText(out.message || `Command '${command || 'unknown'}' acknowledged.`, 320),
            runtime_sync: out.runtime_sync || null,
            context_window: toNum(out.context_window || 0, 0),
          });
          return;
        }
      }).catch((error) => send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(error && error.message ? error.message : 'ws_bridge_failed', 260) }));
    });
    ws.on('error', () => {});
  });
  return {
    tryHandle(req, socket, head) {
      const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
      const match = pathname.match(route);
      if (!match) return false;
      const agentId = cleanText(decodeURIComponent(match[1] || ''), 180);
      if (!agentId) { try { socket.destroy(); } catch {} return true; }
      wss.handleUpgrade(req, socket, head, (ws) => wss.emit('connection', ws, req, agentId));
      return true;
    },
  };
}

module.exports = { createAgentWsBridge };
