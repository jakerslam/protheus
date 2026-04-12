    toolAttemptIdentity: function(tool, idx, prefix) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var receipt = row.tool_attempt_receipt && typeof row.tool_attempt_receipt === 'object'
        ? row.tool_attempt_receipt
        : {};
      var toolName = String(row.name || row.tool || receipt.tool_name || 'tool').trim() || 'tool';
      var attemptId = String(row.attempt_id || row.tool_attempt_id || receipt.attempt_id || '').trim();
      var attemptSequence = Number(row.attempt_sequence || row.tool_attempt_sequence || idx + 1);
      if (!Number.isFinite(attemptSequence) || attemptSequence < 1) attemptSequence = idx + 1;
      var fallbackId = String(row.id || ((prefix || 'tool') + '-' + toolName + '-' + attemptSequence)).trim();
      return {
        id: attemptId || fallbackId,
        attempt_id: attemptId,
        attempt_sequence: attemptSequence,
        identity_key: attemptId || (toolName.toLowerCase() + '#' + attemptSequence)
      };
    },
    stringifyStructuredToolValue: function(value, maxLen) {
      var limit = Number(maxLen || 16000);
      if (!Number.isFinite(limit) || limit < 1) limit = 16000;
      if (typeof value === 'string') return String(value).slice(0, limit);
      if (value == null) return '';
      try {
        return JSON.stringify(value).slice(0, limit);
      } catch (_) {
        return String(value).slice(0, limit);
      }
    },
    normalizeToolContentType: function(value) {
      return typeof value === 'string' ? String(value).toLowerCase() : '';
    },
    isToolCallContentType: function(value) {
      var type = this.normalizeToolContentType(value);
      return type === 'toolcall' || type === 'tool_call' || type === 'tooluse' || type === 'tool_use';
    },
    isToolResultContentType: function(value) {
      var type = this.normalizeToolContentType(value);
      return type === 'toolresult' || type === 'tool_result' || type === 'tool_result_error';
    },
    resolveToolBlockArgs: function(block) {
      if (!block || typeof block !== 'object') return '';
      return block.args != null ? block.args : (block.arguments != null ? block.arguments : (block.input != null ? block.input : ''));
    },
    resolveToolUseId: function(block) {
      if (!block || typeof block !== 'object') return '';
      var id = '';
      if (typeof block.id === 'string' && block.id.trim()) id = block.id;
      else if (typeof block.tool_use_id === 'string' && block.tool_use_id.trim()) id = block.tool_use_id;
      else if (typeof block.toolUseId === 'string' && block.toolUseId.trim()) id = block.toolUseId;
      return String(id || '').trim();
    },
    structuredContentBlocksFromPayload: function(payload) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var out = [];
      var pushBlocks = function(value) {
        if (!Array.isArray(value)) return;
        for (var i = 0; i < value.length; i++) out.push(value[i]);
      };
      pushBlocks(data.content);
      pushBlocks(data.response);
      if (data.message && typeof data.message === 'object') pushBlocks(data.message.content);
      return out;
    },
    assistantTextFromPayload: function(payload) {
      var data = payload && typeof payload === 'object' ? payload : {};
      if (typeof data.response === 'string') return String(data.response || '');
      if (typeof data.content === 'string') return String(data.content || '');
      var blocks = this.structuredContentBlocksFromPayload(data);
      if (!blocks.length) return '';
      var parts = [];
      for (var i = 0; i < blocks.length; i++) {
        var entry = blocks[i];
        if (typeof entry === 'string') {
          if (entry.trim()) parts.push(entry);
          continue;
        }
        if (!entry || typeof entry !== 'object') continue;
        if (this.isToolCallContentType(entry.type) || this.isToolResultContentType(entry.type)) continue;
        var text = typeof entry.text === 'string'
          ? entry.text
          : (typeof entry.content === 'string' ? entry.content : '');
        if (String(text || '').trim()) parts.push(String(text));
      }
      return parts.join('\n\n').trim();
    },
    normalizeResponseToolCard: function(tool, idx, prefix) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var identity = this.toolAttemptIdentity(row, idx, prefix || 'tool');
      return {
        id: identity.id,
        name: row.name || row.tool || 'tool',
        running: false,
        expanded: false,
        input: this.stringifyStructuredToolValue(row.input || row.arguments || row.args || '', 16000),
        result: this.stringifyStructuredToolValue(row.result || row.output || row.summary || '', 24000),
        is_error: !!(row.is_error || row.error || row.blocked),
        blocked: row.blocked === true || String(row.status || '').toLowerCase() === 'blocked',
        status: String(row.status || '').trim().toLowerCase(),
        attempt_id: identity.attempt_id,
        attempt_sequence: identity.attempt_sequence,
        identity_key: identity.identity_key,
        tool_attempt_receipt: row.tool_attempt_receipt || null
      };
    },
    toolCardFromAttemptReceipt: function(rawAttempt, idx, prefix) {
      var envelope = rawAttempt && typeof rawAttempt === 'object' ? rawAttempt : {};
      var attempt = envelope.attempt && typeof envelope.attempt === 'object' ? envelope.attempt : envelope;
      var toolName = String(attempt.tool_name || attempt.tool || 'tool').trim() || 'tool';
      var rawStatus = String(attempt.status || attempt.outcome || '').trim().toLowerCase();
      var blocked = rawStatus === 'blocked' || rawStatus === 'policy_denied';
      var isError = !blocked && !!rawStatus && rawStatus !== 'ok';
      var normalizedArgs = envelope.normalized_result && envelope.normalized_result.normalized_args
        ? envelope.normalized_result.normalized_args
        : null;
      var input = '';
      try {
        if (normalizedArgs && typeof normalizedArgs === 'object') input = JSON.stringify(normalizedArgs);
      } catch (_) {}
      var reason = String(envelope.error || attempt.reason || rawStatus || '').trim();
      var backend = String(attempt.backend || '').trim().replace(/_/g, ' ');
      var result = reason;
      if (!result && backend) result = 'Attempted via ' + backend;
      if (!result && rawStatus === 'ok') result = 'Attempt succeeded';
      if (!result) result = 'Attempt recorded';
      var identity = this.toolAttemptIdentity({
        name: toolName,
        attempt_id: attempt.attempt_id || '',
        attempt_sequence: idx + 1,
        tool_attempt_receipt: attempt
      }, idx, prefix || 'attempt');
      return {
        id: identity.id,
        name: toolName,
        running: false,
        expanded: false,
        input: input,
        result: result,
        is_error: isError,
        blocked: blocked,
        status: blocked ? 'blocked' : (rawStatus || (isError ? 'error' : 'ok')),
        attempt_id: identity.attempt_id,
        attempt_sequence: identity.attempt_sequence,
        identity_key: identity.identity_key,
        reason_code: String(attempt.reason_code || '').trim(),
        backend: String(attempt.backend || '').trim(),
        tool_attempt_receipt: attempt
      };
    },
    structuredContentToolRows: function(payload, prefix) {
      var blocks = this.structuredContentBlocksFromPayload(payload);
      if (!blocks.length) return [];
      var rows = [];
      var byKey = {};
      var ensureRow = function(seed, idx) {
        var identity = this.toolAttemptIdentity(seed, idx, prefix || 'content');
        var key = identity.identity_key;
        var current = byKey[key];
        if (!current) {
          current = {
            id: identity.id,
            name: String(seed.name || seed.tool || 'tool').trim() || 'tool',
            running: false,
            expanded: false,
            input: '',
            result: '',
            is_error: false,
            blocked: false,
            status: '',
            attempt_id: identity.attempt_id,
            attempt_sequence: identity.attempt_sequence,
            identity_key: identity.identity_key,
            tool_attempt_receipt: null
          };
          byKey[key] = current;
          rows.push(current);
        }
        return current;
      }.bind(this);
      for (var i = 0; i < blocks.length; i++) {
        var block = blocks[i];
        if (!block || typeof block !== 'object') continue;
        if (this.isToolCallContentType(block.type)) {
          var callName = String(block.name || block.tool || 'tool').trim() || 'tool';
          var callRow = ensureRow({
            name: callName,
            attempt_id: this.resolveToolUseId(block),
            attempt_sequence: rows.length + 1
          }, rows.length);
          if (!callRow.input) callRow.input = this.stringifyStructuredToolValue(this.resolveToolBlockArgs(block), 16000);
          continue;
        }
        if (!this.isToolResultContentType(block.type)) continue;
        var resultName = String(block.name || block.tool || 'tool').trim() || 'tool';
        var resultRow = ensureRow({
          name: resultName,
          attempt_id: this.resolveToolUseId(block),
          attempt_sequence: rows.length + 1
        }, rows.length);
        var resultText = this.stringifyStructuredToolValue(
          block.result != null ? block.result : (
            block.output != null ? block.output : (
              block.content != null ? block.content : (
                block.text != null ? block.text : (
                  block.error != null ? block.error : ''
                )
              )
            )
          ),
          24000
        );
        if (!resultRow.result && resultText) resultRow.result = resultText;
        var rawStatus = String(block.status || '').trim().toLowerCase();
        var blocked = block.blocked === true || rawStatus === 'blocked' || rawStatus === 'policy_denied';
        var isError = block.is_error === true || this.normalizeToolContentType(block.type) === 'tool_result_error' || (!!rawStatus && rawStatus !== 'ok' && !blocked);
        if (blocked) resultRow.blocked = true;
        if (isError) resultRow.is_error = true;
        if (rawStatus) resultRow.status = rawStatus;
      }
      return rows.slice(0, 16);
    },
    mergeToolCardSets: function(baseRows, incomingRows) {
      var merged = Array.isArray(baseRows) ? baseRows.slice() : [];
      var incoming = Array.isArray(incomingRows) ? incomingRows : [];
      var claimedBaseIndexes = {};
      for (var i = 0; i < incoming.length; i++) {
        var candidate = incoming[i];
        if (!candidate) continue;
        var matched = false;
        for (var j = 0; j < merged.length; j++) {
          var current = merged[j];
          if (!current) continue;
          var sameAttempt = !!candidate.attempt_id && String(current.attempt_id || '').trim() === String(candidate.attempt_id || '').trim();
          var sameUnnamedTool = !candidate.attempt_id && String(current.name || '').toLowerCase() === String(candidate.name || '').toLowerCase();
          var adoptUnnamedBase = !sameAttempt && !current.attempt_id && !claimedBaseIndexes[j] && String(current.name || '').toLowerCase() === String(candidate.name || '').toLowerCase();
          if (!sameAttempt && !sameUnnamedTool && !adoptUnnamedBase) continue;
          if (!current.input && candidate.input) current.input = candidate.input;
          if ((!current.result || !String(current.result).trim()) && candidate.result) current.result = candidate.result;
          if (candidate.blocked) current.blocked = true;
          if (candidate.status) current.status = candidate.status;
          if (candidate.is_error) current.is_error = true;
          if (candidate.id) current.id = candidate.id;
          if (candidate.attempt_id) current.attempt_id = candidate.attempt_id;
          if (candidate.attempt_sequence) current.attempt_sequence = candidate.attempt_sequence;
          if (candidate.identity_key) current.identity_key = candidate.identity_key;
          if (!current.tool_attempt_receipt && candidate.tool_attempt_receipt) current.tool_attempt_receipt = candidate.tool_attempt_receipt;
          claimedBaseIndexes[j] = true;
          matched = true;
          break;
        }
        if (!matched) merged.push(candidate);
      }
      return merged.slice(0, 16);
    },
    responseToolRowsFromPayload: function(payload, prefix) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var base = this.mergeToolCardSets(
        Array.isArray(data.tools)
          ? data.tools.map(function(row, idx) { return this.normalizeResponseToolCard(row, idx, prefix || 'tool'); }, this)
          : [],
        this.structuredContentToolRows(data, prefix || 'content')
      );
      var completion =
        data.response_finalization &&
        data.response_finalization.tool_completion &&
        typeof data.response_finalization.tool_completion === 'object'
          ? data.response_finalization.tool_completion
          : null;
      var attempts = Array.isArray(completion && completion.tool_attempts)
        ? completion.tool_attempts
        : [];
      if (!attempts.length) return base;
      var merged = base.slice();
      for (var i = 0; i < attempts.length; i++) {
        var attemptCard = this.toolCardFromAttemptReceipt(attempts[i], i, prefix || 'attempt');
        merged = this.mergeToolCardSets(merged, [attemptCard]);
      }
      return merged.slice(0, 16);
    },
