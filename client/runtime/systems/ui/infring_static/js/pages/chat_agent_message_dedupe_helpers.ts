// Chat agent-message signature and duplicate merge helpers.
'use strict';

function infringChatAgentMessageDedupeMethods() {
  return {
    agentMessageSignature(message) {
      if (!message || typeof message !== 'object') return '';
      var text = this.messageVisiblePreviewText(message).replace(/\s+/g, ' ').trim().toLowerCase();
      var tools = Array.isArray(message.tools) ? message.tools : [];
      var toolParts = [];
      for (var i = 0; i < tools.length && i < 8; i += 1) {
        var tool = tools[i] || {};
        var name = String(tool.name || '').trim().toLowerCase();
        var result = String(tool.summary || tool.display_text || tool.result_ref || '').replace(/\s+/g, ' ').trim().toLowerCase();
        if (result.length > 180) result = result.slice(0, 180);
        var state = tool && tool.is_error ? 'error' : (tool && tool.running ? 'running' : 'ok');
        if (name || result) toolParts.push(name + ':' + state + ':' + result);
      }
      return (text || '') + '||' + toolParts.join('||');
    },
    assistantTurnStartTimestamp(message) {
      if (!message || typeof message !== 'object') return 0;
      var turn = message.turn_transaction && typeof message.turn_transaction === 'object'
        ? message.turn_transaction
        : null;
      var raw = Number(
        message._turn_started_at ||
        (turn && (turn.started_at || turn.request_started_at || turn.created_at || turn.ts)) ||
        0
      );
      if (!Number.isFinite(raw) || raw <= 0) return 0;
      return raw;
    },
    findRecentDuplicateAgentMessage(candidate, dedupeWindowMs) {
      if (!candidate || typeof candidate !== 'object') return null;
      var rows = Array.isArray(this.messages) ? this.messages : [];
      if (!rows.length) return null;
      var signature = this.agentMessageSignature(candidate);
      if (!signature) return null;
      var candidateTurnStart = this.assistantTurnStartTimestamp(candidate);
      var nowTs = Number(candidate.ts || Date.now());
      var maxAge = Number(dedupeWindowMs || 70000);
      if (!Number.isFinite(maxAge) || maxAge < 5000) maxAge = 70000;
      var checked = 0;
      for (var i = rows.length - 1; i >= 0; i -= 1) {
        var row = rows[i];
        if (!row || row.thinking || row.streaming) continue;
        var role = String(row.role || '').toLowerCase();
        if (role !== 'agent' && role !== 'assistant') continue;
        checked += 1;
        var rowTs = Number(row.ts || 0);
        var ageMs = rowTs > 0 ? Math.abs(nowTs - rowTs) : 0;
        if (ageMs > maxAge && checked > 3) break;
        var rowSignature = this.agentMessageSignature(row);
        if (rowSignature === signature && (!rowTs || ageMs <= maxAge)) return row;
        if (candidateTurnStart > 0) {
          var rowTurnStart = this.assistantTurnStartTimestamp(row);
          if (!(rowTurnStart > 0 && Math.abs(rowTurnStart - candidateTurnStart) <= 1200)) {
            continue;
          }
        }
        if (rowSignature === signature) return row;
        if (checked >= 16) break;
      }
      return null;
    },
    pushAgentMessageDeduped(message, options) {
      var payload = message && typeof message === 'object' ? message : null;
      if (!payload) return null;
      var opts = options && typeof options === 'object' ? options : {};
      var dedupeWindowMs = Number(opts.dedupe_window_ms || opts.dedupeWindowMs || 70000);
      var duplicate = this.findRecentDuplicateAgentMessage(payload, dedupeWindowMs);
      if (!duplicate) {
        return this.appendActiveChatMessage(payload);
      }
      var mergeToolCards = function(existingTools, incomingTools) {
        var base = Array.isArray(existingTools) ? existingTools.slice() : [];
        var incoming = Array.isArray(incomingTools) ? incomingTools : [];
        if (!incoming.length) return base;
        var keyFor = function(tool) {
          if (!tool || typeof tool !== 'object') return '';
          var id = String(tool.id || '').trim();
          if (id) return 'id:' + id;
          var name = String(tool.name || '').trim().toLowerCase();
          var inputRef = String(tool.input_ref || tool.detail_ref || '').trim();
          return 'sig:' + name + '::' + inputRef;
        };
        var index = Object.create(null);
        for (var i = 0; i < base.length; i++) {
          var baseKey = keyFor(base[i]);
          if (!baseKey) continue;
          index[baseKey] = i;
        }
        for (var j = 0; j < incoming.length; j++) {
          var next = incoming[j];
          if (!next || typeof next !== 'object') continue;
          var nextKey = keyFor(next);
          var pos = (nextKey && Object.prototype.hasOwnProperty.call(index, nextKey))
            ? Number(index[nextKey])
            : -1;
          if (pos < 0 || pos >= base.length) {
            base.push(next);
            if (nextKey) index[nextKey] = base.length - 1;
            continue;
          }
          var prior = base[pos];
          if (!prior || typeof prior !== 'object') {
            base[pos] = next;
            continue;
          }
          if (!String(prior.summary || '').trim() && String(next.summary || '').trim()) prior.summary = next.summary;
          if (!String(prior.result_ref || '').trim() && String(next.result_ref || '').trim()) prior.result_ref = next.result_ref;
          if (!String(prior.input_ref || '').trim() && String(next.input_ref || '').trim()) prior.input_ref = next.input_ref;
          if (!String(prior.id || '').trim() && String(next.id || '').trim()) prior.id = next.id;
          if (next.is_error) prior.is_error = true;
          if (prior.running && next.running === false) prior.running = false;
        }
        return base;
      };
      if (duplicate._auto_fallback && !payload._auto_fallback) {
        duplicate.text = payload.text;
        duplicate.tools = Array.isArray(payload.tools) ? payload.tools : [];
        duplicate._auto_fallback = false;
      } else if ((!String(duplicate.text || '').trim()) && String(payload.text || '').trim()) {
        duplicate.text = payload.text;
      }
      if (Array.isArray(payload.tools) && payload.tools.length) {
        duplicate.tools = mergeToolCards(duplicate.tools, payload.tools);
      }
      if (payload.response_finalization && typeof payload.response_finalization === 'object') {
        duplicate.response_finalization = payload.response_finalization;
      }
      if (payload.turn_transaction && typeof payload.turn_transaction === 'object') {
        duplicate.turn_transaction = payload.turn_transaction;
      }
      if (Array.isArray(payload.terminal_transcript) && payload.terminal_transcript.length) {
        duplicate.terminal_transcript = payload.terminal_transcript;
      }
      if (payload.attention_queue && typeof payload.attention_queue === 'object') {
        duplicate.attention_queue = payload.attention_queue;
      }
      if (String(payload.tool_failure_summary || '').trim()) {
        duplicate.tool_failure_summary = String(payload.tool_failure_summary || '').trim();
      }
      var nextMeta = String(payload.meta || '').trim();
      if (nextMeta) {
        var priorMeta = String(duplicate.meta || '').trim();
        duplicate.meta = priorMeta ? priorMeta : nextMeta;
      }
      duplicate.ts = Number(payload.ts || Date.now());
      duplicate.agent_id = payload.agent_id || duplicate.agent_id;
      duplicate.agent_name = payload.agent_name || duplicate.agent_name;
      if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
      this.scheduleConversationPersist();
      return duplicate;
    },
  };
}
