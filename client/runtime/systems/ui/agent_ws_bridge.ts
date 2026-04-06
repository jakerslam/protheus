#!/usr/bin/env tsx

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops (authoritative transport + receipts); this file is UI bridge/wrapper.

let wsDependencyWarned = false;
function resolveWebSocketServerCtor() {
  try {
    const runtime = require('ws');
    if (runtime && typeof runtime.WebSocketServer === 'function') return runtime.WebSocketServer;
  } catch {}
  return null;
}

function createAgentWsBridge({ flags, cleanText, fetchBackend, fetchBackendJson }) {
  const WebSocketServerCtor = resolveWebSocketServerCtor();
  if (!WebSocketServerCtor) {
    if (!wsDependencyWarned) {
      wsDependencyWarned = true;
      console.warn('[infring dashboard] ws module unavailable; disabling local agent websocket bridge and falling back to HTTP transport.');
    }
    return {
      ws_enabled: false,
      ws_error: 'ws_module_missing',
      tryHandle() { return false; },
    };
  }
  const wss = new WebSocketServerCtor({ noServer: true, clientTracking: false, perMessageDeflate: false });
  const route = /^\/api\/agents\/([^/]+)\/ws$/;
  const enc = (agentId) => encodeURIComponent(String(agentId || '').trim());
  const send = (ws, payload) => {
    try { if (ws && ws.readyState === 1) ws.send(JSON.stringify(payload)); } catch {}
  };
  const parseJson = (raw) => { try { return JSON.parse(raw); } catch { return null; } };
  const toNum = (value, fallback = 0) => Number.isFinite(Number(value)) ? Number(value) : fallback;
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, Math.max(0, Number(ms) || 0)));
  const splitThoughtSentences = (raw, maxItems = 6) => {
    const normalized = String(raw || '').replace(/\s+/g, ' ').trim();
    if (!normalized) return [];
    const chunks = normalized.match(/[^.!?]+[.!?]+|[^.!?]+$/g) || [normalized];
    const out = [];
    for (let i = 0; i < chunks.length; i++) {
      const sentence = cleanText(chunks[i] || '', 280);
      if (!sentence) continue;
      out.push(sentence);
      if (out.length >= maxItems) break;
    }
    return out;
  };
  const normalizeToolRows = (rawTools) => {
    if (!Array.isArray(rawTools)) return [];
    const out = [];
    for (let i = 0; i < rawTools.length; i++) {
      const row = rawTools[i] && typeof rawTools[i] === 'object' ? rawTools[i] : {};
      const name = cleanText(row.name || row.tool || 'tool', 120).toLowerCase() || 'tool';
      const input = cleanText(row.input || row.arguments || row.args || '', 16000);
      const result = cleanText(row.result || row.output || row.summary || '', 24000);
      const isError = !!(row.is_error || row.error || row.blocked);
      out.push({
        id: cleanText(row.id || `${name}-${Date.now()}-${i}`, 160),
        name,
        input,
        result,
        is_error: isError,
      });
      if (out.length >= 16) break;
    }
    return out;
  };
  const replayToolTimeline = async (ws, agentId, tools) => {
    if (!Array.isArray(tools) || tools.length < 1) return;
    const replayCount = Math.min(tools.length, 8);
    const basePauseMs = replayCount > 4 ? 42 : 55;
    for (let i = 0; i < replayCount; i++) {
      const tool = tools[i] || {};
      const toolName = cleanText(tool.name || 'tool', 120).toLowerCase() || 'tool';
      const toolInput = cleanText(tool.input || '', 16000);
      const toolResult = cleanText(tool.result || '', 24000);
      const toolError = !!tool.is_error;
      send(ws, {
        type: 'tool_start',
        agent_id: agentId,
        tool: toolName,
        input: toolInput,
      });
      const thoughtLines =
        toolName === 'thought_process'
          ? splitThoughtSentences(toolInput || toolResult, 3)
          : [];
      for (let ti = 0; ti < thoughtLines.length; ti++) {
        send(ws, {
          type: 'phase',
          agent_id: agentId,
          phase: 'reasoning',
          detail: thoughtLines[ti],
        });
        await sleep(basePauseMs);
      }
      send(ws, {
        type: 'tool_result',
        agent_id: agentId,
        tool: toolName,
        input: toolInput,
        result: toolResult,
        is_error: toolError,
      });
      send(ws, {
        type: 'tool_end',
        agent_id: agentId,
        tool: toolName,
        input: toolInput,
      });
      await sleep(basePauseMs);
    }
  };
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
          const phaseCycle = [
            'Analyzing request',
            'Planning next step',
            'Working through tools',
          ];
          let phaseCursor = 0;
          send(ws, {
            type: 'phase',
            agent_id: targetAgent,
            phase: 'thinking',
            detail: phaseCycle[phaseCursor],
          });
          const phaseTimer = setInterval(() => {
            phaseCursor = (phaseCursor + 1) % phaseCycle.length;
            send(ws, {
              type: 'phase',
              agent_id: targetAgent,
              phase: 'thinking',
              detail: phaseCycle[phaseCursor],
            });
          }, 2300);
          let res = null;
          let out = {};
          try {
            res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/message`, {
              method: 'POST',
              headers: { 'content-type': 'application/json' },
              body: JSON.stringify({ message: content, attachments: Array.isArray(payload.attachments) ? payload.attachments : [] }),
            }, 180000);
            out = await res.json().catch(() => ({}));
          } finally {
            clearInterval(phaseTimer);
          }
          if (!res.ok || out.ok === false) {
            send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(out.error || `backend_http_${res.status}`, 260) });
            return;
          }
          const toolRows = normalizeToolRows(out.tools);
          if (toolRows.length > 0) {
            send(ws, {
              type: 'phase',
              agent_id: targetAgent,
              phase: 'planning',
              detail: 'Preparing tool findings',
            });
            await replayToolTimeline(ws, targetAgent, toolRows);
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
            tools: toolRows,
            response_finalization: out.response_finalization || null,
            turn_transaction: out.turn_transaction || null,
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
          if (!res.ok) {
            send(ws, { type: 'terminal_error', agent_id: targetAgent, message: cleanText(out.error || out.message || `backend_http_${res.status}`, 260) });
            return;
          }
          const stdout = String(out.stdout || '');
          const stderrBase = String(out.stderr || '');
          const stderr = stderrBase || (out.ok === false ? cleanText(out.message || out.error || '', 4000) : '');
          send(ws, {
            type: 'terminal_output',
            agent_id: targetAgent,
            stdout,
            stderr,
            exit_code: toNum(out.exit_code || (out.ok === false ? 1 : 0), 0),
            duration_ms: toNum(out.duration_ms || 0, 0),
            cwd: cleanText(out.cwd || '', 4000),
            requested_command: cleanText(out.requested_command || command, 16000),
            executed_command: cleanText(out.executed_command || command, 16000),
            command_translated: !!out.command_translated,
            translation_reason: cleanText(out.translation_reason || '', 240),
            suggestions: Array.isArray(out.suggestions) ? out.suggestions : [],
            permission_gate: out.permission_gate || null,
            filter_events: Array.isArray(out.filter_events) ? out.filter_events : [],
            low_signal_output: !!out.low_signal_output,
            recovery_hints: Array.isArray(out.recovery_hints) ? out.recovery_hints : [],
            tool_summary: out.tool_summary || null,
            tracking: out.tracking || null,
          });
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
            context_tokens: toNum(
              out.context_tokens ||
              out.context_used_tokens ||
              (out.context_pool && out.context_pool.active_tokens) ||
              0,
              0
            ),
            context_ratio: toNum(
              out.context_ratio ||
              (out.context_pool && out.context_pool.context_ratio) ||
              0,
              0
            ),
            context_pressure: cleanText(
              out.context_pressure ||
              (out.context_pool && out.context_pool.context_pressure) ||
              '',
              32
            ),
          });
          return;
        }
      }).catch((error) => send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(error && error.message ? error.message : 'ws_bridge_failed', 260) }));
    });
    ws.on('error', () => {});
  });
  return {
    ws_enabled: true,
    ws_error: '',
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
