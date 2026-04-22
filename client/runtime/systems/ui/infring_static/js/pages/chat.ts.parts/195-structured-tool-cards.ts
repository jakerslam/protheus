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
    responseWorkflowFromPayload: function(payload) {
      var data = payload && typeof payload === 'object' ? payload : {};
      return data.response_workflow && typeof data.response_workflow === 'object'
        ? data.response_workflow
        : null;
    },
    workflowResponseTextFromPayload: function(payload) {
      var workflow = this.responseWorkflowFromPayload(payload);
      if (!workflow) return '';
      var status = String(workflow && workflow.final_llm_response && workflow.final_llm_response.status || '').trim().toLowerCase();
      var response = typeof workflow.response === 'string' ? String(workflow.response || '').trim() : '';
      if (status !== 'synthesized' || !response) return '';
      if (this.textLooksNoFindingsPlaceholder(response) || this.textLooksToolAckWithoutFindings(response)) return '';
      return response;
    },
    assistantTextFromPayload: function(payload) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var workflowText = this.workflowResponseTextFromPayload(data);
      if (workflowText) return workflowText;
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
    parseStructuredToolInput: function(tool) {
      var row = tool && typeof tool === 'object' ? tool : {};
      var input = row.input;
      if (input && typeof input === 'object' && !Array.isArray(input)) return input;
      var raw = typeof input === 'string' ? String(input).trim() : '';
      if (!raw || raw.charAt(0) !== '{') return {};
      try {
        var parsed = JSON.parse(raw);
        return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
      } catch (_) {
        return {};
      }
    },
    toolMetaCandidates: function(tool) {
      var input = this.parseStructuredToolInput(tool);
      var out = [];
      var action = String(input.action || input.method || input.operation || input.op || '').trim();
      if (action) out.push(this.prettifyToolLabel(action));
      var query = String(input.query || input.q || '').trim();
      if (query) out.push('"' + query + '"');
      var url = String(input.url || input.link || '').trim();
      if (url) out.push(url);
      var filePath = String(input.path || input.file || '').trim();
      if (filePath) {
        if (/^\/Users\/[^/]+(\/|$)/.test(filePath)) {
          filePath = filePath.replace(/^\/Users\/[^/]+(\/|$)/, '~$1');
        } else if (/^\/home\/[^/]+(\/|$)/.test(filePath)) {
          filePath = filePath.replace(/^\/home\/[^/]+(\/|$)/, '~$1');
        } else if (/^C:\\Users\\[^\\]+(\\|$)/i.test(filePath)) {
          filePath = filePath.replace(/^C:\\Users\\[^\\]+(\\|$)/i, '~$1');
        }
        out.push(filePath);
      }
      return out.slice(0, 3);
    },
    formatToolAggregateMeta: function(tool) {
      var label = String(tool && tool.name ? tool.name : 'tool').replace(/_/g, ' ').trim() || 'tool';
      var metas = this.toolMetaCandidates(tool);
      if (!metas.length) return label;
      return label + ': ' + metas.join('; ');
    },
    backfillToolRowsFromCompletion: function(rows, payload) {
      var merged = Array.isArray(rows) ? rows.map(function(row) {
        return row && typeof row === 'object' ? Object.assign({}, row) : row;
      }) : [];
      var data = payload && typeof payload === 'object' ? payload : {};
      var completion =
        data.response_finalization &&
        data.response_finalization.tool_completion &&
        typeof data.response_finalization.tool_completion === 'object'
          ? data.response_finalization.tool_completion
          : null;
      var steps = Array.isArray(completion && completion.live_tool_steps)
        ? completion.live_tool_steps
        : [];
      if (!steps.length) return merged.slice(0, 16);
      if (!merged.length) {
        for (var si = 0; si < steps.length && merged.length < 16; si++) {
          var stepSeed = steps[si] && typeof steps[si] === 'object' ? steps[si] : {};
          var stepName = String(stepSeed.tool || stepSeed.name || 'tool').trim() || 'tool';
          var stepStatus = String(stepSeed.status || '').trim();
          if (!stepName && !stepStatus) continue;
          merged.push(this.normalizeResponseToolCard({
            id: 'completion-step-' + (si + 1) + '-' + stepName,
            name: stepName,
            result: stepStatus ? ('Missing tool_result block; last known status: ' + stepStatus) : '',
            is_error: !!stepSeed.is_error,
            status: stepStatus ? stepStatus.toLowerCase() : ''
          }, si, 'completion'));
        }
      }
      for (var i = 0; i < merged.length; i++) {
        var row = merged[i] && typeof merged[i] === 'object' ? merged[i] : null;
        if (!row) continue;
        var rowName = String(row.name || '').trim().toLowerCase();
        var step = null;
        var byIndex = steps[i] && typeof steps[i] === 'object' ? steps[i] : null;
        if (byIndex && String(byIndex.tool || byIndex.name || '').trim().toLowerCase() === rowName && String(byIndex.status || '').trim()) {
          step = byIndex;
        } else {
          for (var si = 0; si < steps.length; si++) {
            var candidate = steps[si] && typeof steps[si] === 'object' ? steps[si] : null;
            if (!candidate) continue;
            if (String(candidate.tool || candidate.name || '').trim().toLowerCase() !== rowName) continue;
            if (!String(candidate.status || '').trim()) continue;
            step = candidate;
            break;
          }
        }
        if (!step) continue;
        var statusText = String(step.status || '').trim();
        if (!row.status && statusText) row.status = statusText.toLowerCase();
        if ((!row.result || !String(row.result).trim()) && statusText) {
          row.result = 'Missing tool_result block; last known status: ' + statusText;
        }
        if (step.is_error === true && !row.blocked) row.is_error = true;
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
      if (!attempts.length) return this.backfillToolRowsFromCompletion(base, data).slice(0, 16);
      var merged = base.slice();
      for (var i = 0; i < attempts.length; i++) {
        var attemptCard = this.toolCardFromAttemptReceipt(attempts[i], i, prefix || 'attempt');
        merged = this.mergeToolCardSets(merged, [attemptCard]);
      }
      return this.backfillToolRowsFromCompletion(merged, data).slice(0, 16);
    },
    responseFinalizationFromPayload: function(payload) {
      var data = payload && typeof payload === 'object' ? payload : {};
      return data.response_finalization && typeof data.response_finalization === 'object'
        ? data.response_finalization
        : null;
    },
    readableToolFailureSummary: function(payload, tools) {
      var rows = Array.isArray(tools) ? tools.filter(function(tool) {
        return !!(tool && String(tool.name || '').toLowerCase() !== 'thought_process');
      }) : [];
      if (!rows.length) return '';
      var blocked = rows.find(function(tool) {
        return !!(tool && !tool.running && this.isBlockedTool(tool));
      }, this);
      if (blocked) {
        var blockedName = this.toolDisplayName(blocked);
        var blockedDetail = this.toolResultSummarySnippet(blocked) || String(blocked.status || '').trim() || 'blocked by policy';
        return 'The ' + (blockedName || 'tool') + ' step was blocked before I could finish the answer: ' + blockedDetail;
      }
      var failed = rows.find(function(tool) {
        return !!(tool && !tool.running && tool.is_error);
      });
      if (failed) {
        var failedName = this.toolDisplayName(failed);
        var failedDetail = this.toolResultSummarySnippet(failed) || String(failed.status || '').trim() || 'step failed';
        return 'The ' + (failedName || 'tool') + ' step failed before I could finish the answer: ' + failedDetail;
      }
      var actionableWeb = rows.find(function(tool) {
        if (!tool || tool.running || !this.isWebLikeToolName(tool.name || '')) return false;
        return (
          this.textMentionsContextGuard(tool.result || '') ||
          this.textLooksNoFindingsPlaceholder(tool.result || '') ||
          this.textLooksToolAckWithoutFindings(tool.result || '')
        );
      }, this);
      if (actionableWeb) {
        return this.lowSignalWebToolSummary(actionableWeb);
      }
      return '';
    },
    fallbackAssistantTextFromPayload: function(payload, tools) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var workflowText = this.workflowResponseTextFromPayload(data);
      if (workflowText) return workflowText;
      var rows = Array.isArray(tools) ? tools : [];
      var failureSummary = this.readableToolFailureSummary(data, rows);
      if (failureSummary) return failureSummary;
      var toolSummary = this.completedToolOnlySummary(rows);
      if (toolSummary) return toolSummary;
      var finalization = this.responseFinalizationFromPayload(data);
      var completion = finalization && finalization.tool_completion && typeof finalization.tool_completion === 'object'
        ? finalization.tool_completion
        : null;
      var reasoning = String(completion && completion.reasoning ? completion.reasoning : '').trim();
      if (reasoning && !this.textLooksNoFindingsPlaceholder(reasoning) && !this.textLooksToolAckWithoutFindings(reasoning)) {
        return reasoning;
      }
      var stepRows = this.backfillToolRowsFromCompletion([], data);
      if (stepRows.length) {
        var stepSummary = this.completedToolOnlySummary(stepRows);
        if (stepSummary) return stepSummary;
      }
      if (finalization && finalization.applied === true) {
        return 'I hit a response finalization edge on that turn. I can continue with a direct answer from current context and avoid extra tool calls unless you explicitly request one.';
      }
      if (
        data &&
        typeof data === 'object' &&
        (
          data.response_finalization ||
          data.turn_transaction ||
          (Array.isArray(data.tools) && data.tools.length) ||
          data.response != null ||
          data.content != null
        )
      ) {
        return 'I completed the run, but no visible reply was returned. Ask me to continue and I will retry the synthesis.';
      }
      return '';
    },
    assistantTurnMetadataFromPayload: function(payload, tools) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var out = {};
      if (data.response_workflow && typeof data.response_workflow === 'object') {
        out.response_workflow = data.response_workflow;
      }
      var finalization = this.responseFinalizationFromPayload(data);
      if (finalization) out.response_finalization = finalization;
      if (data.turn_transaction && typeof data.turn_transaction === 'object') {
        out.turn_transaction = data.turn_transaction;
      }
      if (Array.isArray(data.terminal_transcript) && data.terminal_transcript.length) {
        out.terminal_transcript = data.terminal_transcript.slice(0, 48);
      }
      if (data.attention_queue && typeof data.attention_queue === 'object') {
        out.attention_queue = data.attention_queue;
      }
      var failureSummary = this.readableToolFailureSummary(data, tools);
      if (failureSummary) out.tool_failure_summary = failureSummary;
      return out;
    },
