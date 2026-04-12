#!/usr/bin/env tsx

// Layer ownership: adapters/runtime::agent-ws-bridge (dashboard websocket surface adapter).

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
  const toolIdentity = (row, idx, prefix = 'tool') => {
    const source = row && typeof row === 'object' ? row : {};
    const receipt = source.tool_attempt_receipt && typeof source.tool_attempt_receipt === 'object'
      ? source.tool_attempt_receipt
      : {};
    const name = cleanText(source.name || source.tool || receipt.tool_name || 'tool', 120).toLowerCase() || 'tool';
    const attemptId = cleanText(source.attempt_id || source.tool_attempt_id || receipt.attempt_id || '', 160);
    const attemptSequence = toNum(source.attempt_sequence || source.tool_attempt_sequence || idx + 1, idx + 1);
    const fallbackId = cleanText(source.id || `${prefix}-${name}-${attemptSequence}`, 160);
    return {
      id: attemptId || fallbackId,
      attemptId,
      attemptSequence,
      identityKey: attemptId || `${name}#${attemptSequence}`,
    };
  };
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
  const stringifyStructuredValue = (value, max = 16000) => {
    if (typeof value === 'string') return cleanText(value, max);
    if (value == null) return '';
    try {
      return cleanText(JSON.stringify(value), max);
    } catch {
      return cleanText(String(value), max);
    }
  };
  const normalizeToolContentType = (value) =>
    typeof value === 'string' ? value.toLowerCase() : '';
  const isToolCallContentType = (value) => {
    const type = normalizeToolContentType(value);
    return type === 'toolcall' || type === 'tool_call' || type === 'tooluse' || type === 'tool_use';
  };
  const isToolResultContentType = (value) => {
    const type = normalizeToolContentType(value);
    return type === 'toolresult' || type === 'tool_result' || type === 'tool_result_error';
  };
  const resolveToolBlockArgs = (block) => {
    if (!block || typeof block !== 'object') return '';
    return block.args ?? block.arguments ?? block.input ?? '';
  };
  const resolveToolUseId = (block) => {
    if (!block || typeof block !== 'object') return '';
    const id =
      (typeof block.id === 'string' && block.id.trim()) ||
      (typeof block.tool_use_id === 'string' && block.tool_use_id.trim()) ||
      (typeof block.toolUseId === 'string' && block.toolUseId.trim()) ||
      '';
    return cleanText(id, 160);
  };
  const structuredContentBlocks = (payload) => {
    const data = payload && typeof payload === 'object' ? payload : {};
    const out = [];
    const pushBlocks = (value) => {
      if (!Array.isArray(value)) return;
      for (let i = 0; i < value.length; i++) out.push(value[i]);
    };
    pushBlocks(data.content);
    pushBlocks(data.response);
    if (data.message && typeof data.message === 'object') {
      pushBlocks(data.message.content);
    }
    return out;
  };
  const assistantTextFromPayload = (payload) => {
    const data = payload && typeof payload === 'object' ? payload : {};
    if (typeof data.response === 'string') return String(data.response || '');
    if (typeof data.content === 'string') return String(data.content || '');
    const blocks = structuredContentBlocks(data);
    if (!blocks.length) return '';
    const parts = [];
    for (let i = 0; i < blocks.length; i++) {
      const entry = blocks[i];
      if (typeof entry === 'string') {
        const text = cleanText(entry, 12000);
        if (text) parts.push(text);
        continue;
      }
      if (!entry || typeof entry !== 'object') continue;
      if (isToolCallContentType(entry.type) || isToolResultContentType(entry.type)) continue;
      const text =
        typeof entry.text === 'string'
          ? entry.text
          : (typeof entry.content === 'string' ? entry.content : '');
      const cleaned = cleanText(text, 12000);
      if (cleaned) parts.push(cleaned);
    }
    return cleanText(parts.join('\n\n'), 24000);
  };
  const normalizeToolRows = (rawTools) => {
    if (!Array.isArray(rawTools)) return [];
    const out = [];
    for (let i = 0; i < rawTools.length; i++) {
      const row = rawTools[i] && typeof rawTools[i] === 'object' ? rawTools[i] : {};
      const name = cleanText(row.name || row.tool || 'tool', 120).toLowerCase() || 'tool';
      const identity = toolIdentity({ ...row, name }, i, 'tool');
      const input = stringifyStructuredValue(row.input || row.arguments || row.args || '', 16000);
      const result = stringifyStructuredValue(row.result || row.output || row.summary || '', 24000);
      const isError = !!(row.is_error || row.error || row.blocked);
      out.push({
        id: identity.id,
        name,
        input,
        result,
        is_error: isError,
        blocked: row.blocked === true || String(row.status || '').toLowerCase() === 'blocked',
        status: cleanText(row.status || '', 40).toLowerCase(),
        attempt_id: identity.attemptId,
        attempt_sequence: identity.attemptSequence,
        identity_key: identity.identityKey,
        tool_attempt_receipt: row.tool_attempt_receipt || null,
      });
      if (out.length >= 16) break;
    }
    return out;
  };
  const structuredToolRows = (payload) => {
    const blocks = structuredContentBlocks(payload);
    if (!blocks.length) return [];
    const out = [];
    const byIdentity = new Map();
    const ensureRow = (row, idx) => {
      const identity = toolIdentity(row, idx, 'content');
      const key = identity.identityKey;
      let current = byIdentity.get(key) || null;
      if (!current) {
        current = {
          id: identity.id,
          name: cleanText(row.name || row.tool || 'tool', 120).toLowerCase() || 'tool',
          input: '',
          result: '',
          is_error: false,
          blocked: false,
          status: '',
          attempt_id: identity.attemptId,
          attempt_sequence: identity.attemptSequence,
          identity_key: identity.identityKey,
          tool_attempt_receipt: null,
        };
        byIdentity.set(key, current);
        out.push(current);
      }
      return current;
    };
    for (let i = 0; i < blocks.length; i++) {
      const entry = blocks[i];
      if (!entry || typeof entry !== 'object') continue;
      const block = entry;
      if (isToolCallContentType(block.type)) {
        const toolName = cleanText(block.name || block.tool || 'tool', 120).toLowerCase() || 'tool';
        const row = ensureRow({
          name: toolName,
          attempt_id: resolveToolUseId(block),
          attempt_sequence: out.length + 1,
        }, out.length);
        if (!row.input) row.input = stringifyStructuredValue(resolveToolBlockArgs(block), 16000);
        continue;
      }
      if (!isToolResultContentType(block.type)) continue;
      const toolUseId = resolveToolUseId(block);
      const toolName = cleanText(block.name || block.tool || 'tool', 120).toLowerCase() || 'tool';
      const row = ensureRow({
        name: toolName,
        attempt_id: toolUseId,
        attempt_sequence: out.length + 1,
      }, out.length);
      const result = stringifyStructuredValue(
        block.result ?? block.output ?? block.content ?? block.text ?? block.error ?? '',
        24000
      );
      if (!row.result && result) row.result = result;
      if (!row.name || row.name === 'tool') row.name = toolName;
      const rawStatus = cleanText(block.status || '', 40).toLowerCase();
      const blocked = block.blocked === true || rawStatus === 'blocked' || rawStatus === 'policy_denied';
      const isError =
        block.is_error === true ||
        normalizeToolContentType(block.type) === 'tool_result_error' ||
        (!!rawStatus && rawStatus !== 'ok' && !blocked);
      if (blocked) row.blocked = true;
      if (isError) row.is_error = true;
      if (rawStatus) row.status = rawStatus;
    }
    return out.slice(0, 16);
  };
  const toolCardFromAttempt = (rawAttempt, idx) => {
    const envelope = rawAttempt && typeof rawAttempt === 'object' ? rawAttempt : {};
    const attempt = envelope.attempt && typeof envelope.attempt === 'object' ? envelope.attempt : envelope;
    const toolName = cleanText(attempt.tool_name || attempt.tool || 'tool', 120).toLowerCase() || 'tool';
    const rawStatus = cleanText(attempt.status || attempt.outcome || '', 40).toLowerCase();
    const blocked = rawStatus === 'blocked' || rawStatus === 'policy_denied';
    const isError = !blocked && !!rawStatus && rawStatus !== 'ok';
    const identity = toolIdentity({
      name: toolName,
      attempt_id: attempt.attempt_id || '',
      attempt_sequence: idx + 1,
      tool_attempt_receipt: attempt,
    }, idx, 'attempt');
    let input = '';
    try {
      if (envelope.normalized_result && envelope.normalized_result.normalized_args) {
        input = cleanText(JSON.stringify(envelope.normalized_result.normalized_args), 16000);
      }
    } catch {}
    const result = cleanText(
      envelope.error || attempt.reason || (attempt.backend ? ('Attempted via ' + String(attempt.backend).replace(/_/g, ' ')) : '') || rawStatus || 'attempt recorded',
      24000
    );
    return {
      id: identity.id,
      name: toolName,
      input,
      result,
      is_error: isError,
      blocked,
      status: blocked ? 'blocked' : (rawStatus || (isError ? 'error' : 'ok')),
      attempt_id: identity.attemptId,
      attempt_sequence: identity.attemptSequence,
      identity_key: identity.identityKey,
      tool_attempt_receipt: attempt,
    };
  };
  const mergeToolRowSets = (baseRows, extraRows) => {
    const merged = Array.isArray(baseRows) ? baseRows.slice() : [];
    const incoming = Array.isArray(extraRows) ? extraRows : [];
    const claimedBaseIndexes = new Set();
    for (let i = 0; i < incoming.length; i++) {
      const candidate = incoming[i] || {};
      let matched = false;
      for (let j = 0; j < merged.length; j++) {
        const current = merged[j] || {};
        const sameAttempt = candidate.attempt_id && String(current.attempt_id || '').trim() === String(candidate.attempt_id || '').trim();
        const sameUnnamedTool = !candidate.attempt_id && String(current.name || '').toLowerCase() === String(candidate.name || '').toLowerCase();
        const adoptUnnamedBase = !sameAttempt && !current.attempt_id && !claimedBaseIndexes.has(j) && String(current.name || '').toLowerCase() === String(candidate.name || '').toLowerCase();
        if (!sameAttempt && !sameUnnamedTool && !adoptUnnamedBase) continue;
        if (!current.input && candidate.input) current.input = candidate.input;
        if (!current.result && candidate.result) current.result = candidate.result;
        if (candidate.blocked) current.blocked = true;
        if (candidate.status) current.status = candidate.status;
        if (candidate.is_error) current.is_error = true;
        if (candidate.id) current.id = candidate.id;
        if (candidate.attempt_id) current.attempt_id = candidate.attempt_id;
        if (candidate.attempt_sequence) current.attempt_sequence = candidate.attempt_sequence;
        if (candidate.identity_key) current.identity_key = candidate.identity_key;
        if (candidate.tool_attempt_receipt) current.tool_attempt_receipt = candidate.tool_attempt_receipt;
        claimedBaseIndexes.add(j);
        matched = true;
        break;
      }
      if (!matched) merged.push(candidate);
    }
    return merged.slice(0, 16);
  };
  const mergeResponseToolRows = (payload) => {
    const data = payload && typeof payload === 'object' ? payload : {};
    const base = mergeToolRowSets(
      normalizeToolRows(data.tools),
      structuredToolRows(data)
    );
    const completion =
      data &&
      data.response_finalization &&
      data.response_finalization.tool_completion &&
      typeof data.response_finalization.tool_completion === 'object'
        ? data.response_finalization.tool_completion
        : null;
    const attempts = Array.isArray(completion && completion.tool_attempts)
      ? completion.tool_attempts
      : [];
    if (!attempts.length) return base;
    const merged = base.slice();
    for (let i = 0; i < attempts.length; i++) {
      const attemptCard = toolCardFromAttempt(attempts[i], i);
      const nextMerged = mergeToolRowSets(merged, [attemptCard]);
      merged.length = 0;
      Array.prototype.push.apply(merged, nextMerged);
    }
    return merged.slice(0, 16);
  };
  const normalizeToolCompletionSteps = (rawCompletion) => {
    if (!rawCompletion || typeof rawCompletion !== 'object') return [];
    const rows = Array.isArray(rawCompletion.live_tool_steps)
      ? rawCompletion.live_tool_steps
      : [];
    const out = [];
    for (let i = 0; i < rows.length; i++) {
      const row = rows[i] && typeof rows[i] === 'object' ? rows[i] : {};
      const tool = cleanText(row.tool || row.name || '', 120).toLowerCase();
      const status = cleanText(row.status || row.tool_status || '', 220);
      if (!tool || !status) continue;
      out.push({
        tool,
        status,
        is_error: !!row.is_error,
      });
      if (out.length >= 16) break;
    }
    return out;
  };
  const toolStatusFromCompletion = (toolName, toolIndex, completionSteps, fallbackStatus) => {
    if (Array.isArray(completionSteps) && completionSteps.length > 0) {
      const byIndex = completionSteps[toolIndex] || null;
      if (
        byIndex &&
        String(byIndex.tool || '').toLowerCase() === String(toolName || '').toLowerCase() &&
        byIndex.status
      ) {
        return cleanText(byIndex.status, 220);
      }
      for (let i = 0; i < completionSteps.length; i++) {
        const row = completionSteps[i] || {};
        if (
          String(row.tool || '').toLowerCase() === String(toolName || '').toLowerCase() &&
          row.status
        ) {
          return cleanText(row.status, 220);
        }
      }
    }
    return cleanText(fallbackStatus || '', 220);
  };
  const replayToolTimeline = async (ws, agentId, tools, toolCompletion) => {
    if (!Array.isArray(tools) || tools.length < 1) return;
    const replayCount = Math.min(tools.length, 8);
    const completionSteps = normalizeToolCompletionSteps(toolCompletion);
    const fallbackToolStatus = cleanText(
      toolCompletion && typeof toolCompletion === 'object'
        ? toolCompletion.live_tool_status || ''
        : '',
      220
    );
    const basePauseMs = replayCount > 4 ? 90 : 120;
    for (let i = 0; i < replayCount; i++) {
      const tool = tools[i] || {};
      const toolName = cleanText(tool.name || 'tool', 120).toLowerCase() || 'tool';
      const toolInput = cleanText(tool.input || '', 16000);
      const toolResult = cleanText(tool.result || '', 24000);
      const toolError = !!tool.is_error;
      const attemptId = cleanText(tool.attempt_id || (tool.tool_attempt_receipt && tool.tool_attempt_receipt.attempt_id) || '', 160);
      const attemptSequence = toNum(tool.attempt_sequence || i + 1, i + 1);
      const toolStatus = toolStatusFromCompletion(
        toolName,
        i,
        completionSteps,
        i === 0 ? fallbackToolStatus : ''
      );
      send(ws, {
        type: 'tool_start',
        agent_id: agentId,
        tool: toolName,
        input: toolInput,
        attempt_id: attemptId,
        attempt_sequence: attemptSequence,
        tool_status: toolStatus,
      });
      if (toolStatus) {
        send(ws, {
          type: 'phase',
          agent_id: agentId,
          phase: 'tool_running',
          detail: toolStatus,
          tool: toolName,
          source: 'tool_completion_receipt',
          progress_percent: Math.round(((i + 1) / replayCount) * 100),
          tool_step_index: i + 1,
          tool_step_total: replayCount,
        });
      }
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
        attempt_id: attemptId,
        attempt_sequence: attemptSequence,
        result: toolResult,
        is_error: toolError,
        tool_status: toolStatus,
      });
      send(ws, {
        type: 'tool_end',
        agent_id: agentId,
        tool: toolName,
        input: toolInput,
        attempt_id: attemptId,
        attempt_sequence: attemptSequence,
        tool_status: toolStatus,
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
          let res = null;
          let out = {};
          res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/message`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ message: content, attachments: Array.isArray(payload.attachments) ? payload.attachments : [] }),
          }, 180000);
          out = await res.json().catch(() => ({}));
          const hasStructuredResponse =
            !!(out && typeof out === 'object' && (
              typeof out.response === 'string' ||
              Array.isArray(out.response) ||
              Array.isArray(out.content) ||
              (out.response_finalization &&
                out.response_finalization.tool_completion &&
                Array.isArray(out.response_finalization.tool_completion.tool_attempts)) ||
              Array.isArray(out.tools)
            ));
          if ((!res.ok || out.ok === false) && !hasStructuredResponse) {
            send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(out.error || `backend_http_${res.status}`, 260) });
            return;
          }
          const toolRows = mergeResponseToolRows(out);
          const toolCompletion =
            out &&
            out.response_finalization &&
            out.response_finalization.tool_completion &&
            typeof out.response_finalization.tool_completion === 'object'
              ? out.response_finalization.tool_completion
              : null;
          if (toolRows.length > 0) {
            await replayToolTimeline(ws, targetAgent, toolRows, toolCompletion);
          }
          send(ws, {
            type: 'response',
            agent_id: targetAgent,
            agent_name: agentName || cleanText(out.agent_name || '', 120) || '',
            content: assistantTextFromPayload(out),
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
