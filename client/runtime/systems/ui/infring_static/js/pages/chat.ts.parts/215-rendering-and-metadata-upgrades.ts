    _messageToolRows: function(msg) {
      if (!msg || !Array.isArray(msg.tools)) return [];
      return msg.tools.filter(function(tool) {
        return !!tool && String(tool.name || '').toLowerCase() !== 'thought_process';
      });
    },

    _collectSourceCandidatesFromValue: function(value, out, seen, depth) {
      if (!value || !out || !seen) return;
      var nextDepth = Number(depth || 0);
      if (!Number.isFinite(nextDepth) || nextDepth < 0) nextDepth = 0;
      if (nextDepth > 4 || out.length >= 24) return;
      if (typeof value === 'string') {
        var text = String(value || '').trim();
        if (/^https?:\/\//i.test(text)) {
          if (!seen[text]) {
            seen[text] = true;
            out.push({ url: text, label: '', source: '' });
          }
        }
        return;
      }
      if (Array.isArray(value)) {
        for (var ai = 0; ai < value.length && out.length < 24; ai += 1) {
          this._collectSourceCandidatesFromValue(value[ai], out, seen, nextDepth + 1);
        }
        return;
      }
      if (typeof value !== 'object') return;
      var url = String(
        value.url ||
        value.href ||
        value.link ||
        value.source_url ||
        value.final_url ||
        value.resolved_url ||
        ''
      ).trim();
      if (url && /^https?:\/\//i.test(url) && !seen[url]) {
        seen[url] = true;
        out.push({
          url: url,
          label: String(value.title || value.name || value.label || '').trim(),
          source: String(value.source || value.provider || value.domain || '').trim()
        });
      }
      var keys = Object.keys(value);
      for (var ki = 0; ki < keys.length && out.length < 24; ki += 1) {
        var key = keys[ki];
        if (!Object.prototype.hasOwnProperty.call(value, key)) continue;
        if (key === 'url' || key === 'href' || key === 'link' || key === 'source_url' || key === 'final_url' || key === 'resolved_url') continue;
        if (key === 'content' || key === 'result' || key === 'output' || key === 'payload' || key === 'data') {
          this._collectSourceCandidatesFromValue(value[key], out, seen, nextDepth + 1);
          continue;
        }
        if (nextDepth <= 2 && typeof value[key] === 'object') {
          this._collectSourceCandidatesFromValue(value[key], out, seen, nextDepth + 1);
        }
      }
    },

    _normalizeMessageSourceChip: function(row, idx) {
      var entry = row && typeof row === 'object' ? row : {};
      var url = String(entry.url || entry.href || entry.link || '').trim();
      if (!url || !/^https?:\/\//i.test(url)) return null;
      var label = String(entry.label || entry.title || entry.name || '').trim();
      var host = '';
      try {
        host = new URL(url).hostname.replace(/^www\./i, '');
      } catch (_) {
        host = '';
      }
      var source = String(entry.source || '').trim();
      if (!label) label = source || host || ('Source ' + (Number(idx || 0) + 1));
      if (label.length > 64) label = label.slice(0, 61).trim() + '...';
      return {
        id: 'src-' + (idx + 1) + '-' + label.toLowerCase().replace(/[^a-z0-9]+/g, '-'),
        label: label,
        host: host,
        source: source,
        url: url
      };
    },

    assistantTurnMetadataFromPayload: function(payload, tools) {
      var data = payload && typeof payload === 'object' ? payload : {};
      var out = {};
      if (data.response_workflow && typeof data.response_workflow === 'object') out.response_workflow = data.response_workflow;
      var finalization = typeof this.responseFinalizationFromPayload === 'function'
        ? this.responseFinalizationFromPayload(data)
        : (data.response_finalization && typeof data.response_finalization === 'object' ? data.response_finalization : null);
      if (finalization) out.response_finalization = finalization;
      if (data.turn_transaction && typeof data.turn_transaction === 'object') out.turn_transaction = data.turn_transaction;
      if (Array.isArray(data.terminal_transcript) && data.terminal_transcript.length) out.terminal_transcript = data.terminal_transcript.slice(0, 48);
      if (data.attention_queue && typeof data.attention_queue === 'object') out.attention_queue = data.attention_queue;
      if (Array.isArray(data.sources) && data.sources.length) out.sources = data.sources.slice(0, 16);
      if (Array.isArray(data.citations) && data.citations.length) out.citations = data.citations.slice(0, 24);
      if (Array.isArray(data.reference_links) && data.reference_links.length) out.reference_links = data.reference_links.slice(0, 24);
      var failureSummary = typeof this.readableToolFailureSummary === 'function'
        ? this.readableToolFailureSummary(data, tools)
        : '';
      if (failureSummary) out.tool_failure_summary = failureSummary;
      return out;
    },

    messageSourceChips: function(msg) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var signature = [
        String(row.id || ''),
        String(row.text || '').length,
        Array.isArray(row.tools) ? row.tools.length : 0,
        row.response_workflow ? 'wf1' : 'wf0',
        row.response_finalization ? 'rf1' : 'rf0',
        row.turn_transaction ? 'tx1' : 'tx0'
      ].join('|');
      if (row._source_chip_signature === signature && Array.isArray(row._source_chips_cached)) {
        return row._source_chips_cached;
      }
      var candidates = [];
      var seenUrls = {};
      this._collectSourceCandidatesFromValue(row.sources, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_workflow && row.response_workflow.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_workflow && row.response_workflow.sources, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_finalization && row.response_finalization.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.response_finalization && row.response_finalization.sources, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.turn_transaction && row.turn_transaction.citations, candidates, seenUrls, 0);
      this._collectSourceCandidatesFromValue(row.turn_transaction && row.turn_transaction.evidence, candidates, seenUrls, 0);
      if (Array.isArray(row.tools)) {
        for (var i = 0; i < row.tools.length && candidates.length < 24; i += 1) {
          var tool = row.tools[i] || {};
          var parsedResult = null;
          if (tool.result && typeof tool.result === 'string') {
            var trimmed = String(tool.result || '').trim();
            if (trimmed && (trimmed.charAt(0) === '{' || trimmed.charAt(0) === '[')) {
              try { parsedResult = JSON.parse(trimmed); } catch (_) {}
            }
          } else if (tool.result && typeof tool.result === 'object') {
            parsedResult = tool.result;
          }
          this._collectSourceCandidatesFromValue(parsedResult, candidates, seenUrls, 0);
          if (Array.isArray(tool._imageUrls)) {
            for (var ui = 0; ui < tool._imageUrls.length && candidates.length < 24; ui += 1) {
              this._collectSourceCandidatesFromValue(tool._imageUrls[ui], candidates, seenUrls, 0);
            }
          }
        }
      }
      var chips = [];
      for (var ci = 0; ci < candidates.length && chips.length < 8; ci += 1) {
        var normalized = this._normalizeMessageSourceChip(candidates[ci], ci);
        if (!normalized) continue;
        chips.push(normalized);
      }
      row._source_chip_signature = signature;
      row._source_chips_cached = chips;
      return chips;
    },

    messageHasSourceChips: function(msg) {
      return this.messageSourceChips(msg).length > 0;
    },

    messageToolTraceSummary: function(msg) {
      var rows = this._messageToolRows(msg);
      var summary = {
        visible: false,
        running: false,
        total: 0,
        done: 0,
        blocked: 0,
        errored: 0,
        label: '',
        detail: ''
      };
      if (!rows.length) return summary;
      summary.visible = true;
      summary.total = rows.length;
      for (var i = 0; i < rows.length; i += 1) {
        var tool = rows[i];
        if (!tool) continue;
        if (tool.running) {
          summary.running = true;
          continue;
        }
        if (this.isBlockedTool(tool)) {
          summary.blocked += 1;
          continue;
        }
        if (tool.is_error) {
          summary.errored += 1;
          continue;
        }
        summary.done += 1;
      }
      summary.label = summary.running ? 'Tool trace running' : 'Tool trace complete';
      var bits = [];
      if (summary.done > 0) bits.push(summary.done + ' done');
      if (summary.errored > 0) bits.push(summary.errored + ' error');
      if (summary.blocked > 0) bits.push(summary.blocked + ' blocked');
      if (summary.running) bits.push((summary.total - (summary.done + summary.errored + summary.blocked)) + ' in progress');
      if (!bits.length) bits.push(summary.total + ' recorded');
      summary.detail = bits.join(' · ');
      return summary;
    },

    messageToolTraceRows: function(msg) {
      var rows = this._messageToolRows(msg);
      var out = [];
      for (var i = 0; i < rows.length && out.length < 6; i += 1) {
        var tool = rows[i] || {};
        var label = this.toolDisplayName(tool);
        var state = tool.running
          ? 'running'
          : (this.isBlockedTool(tool) ? 'blocked' : (tool.is_error ? 'error' : 'done'));
        out.push({
          id: String(tool.id || tool.attempt_id || (label + '-' + i)).trim(),
          label: label,
          state: state,
          detail: String(tool.status || '').trim()
        });
      }
      return out;
    },

    thinkingPhaseText: function(msg) {
      if (!msg || !msg.thinking) return '';
      var primary = typeof this.thinkingStatusText === 'function'
        ? String(this.thinkingStatusText(msg) || '').trim()
        : '';
      var primaryNorm = primary.toLowerCase().replace(/\s+/g, ' ').trim();
      var summary = this.thinkingToolStatusSummary(msg);
      if (summary && summary.text) {
        var summaryText = String(summary.text || '').trim();
        var summaryNorm = summaryText.toLowerCase().replace(/\s+/g, ' ').trim();
        if (
          summaryNorm &&
          primaryNorm &&
          (summaryNorm === primaryNorm || summaryNorm.indexOf(primaryNorm) >= 0 || primaryNorm.indexOf(summaryNorm) >= 0)
        ) {
          return '';
        }
        return summaryText;
      }
      if (primaryNorm && primaryNorm !== 'thinking') {
        // Prevent duplicate waiting/workflow status lines.
        return '';
      }
      if (this._pendingWsRequest && this._pendingWsRequest.agent_id) return 'Waiting for runtime response...';
      return 'Analyzing next step...';
    },

    thinkingTraceSummary: function(msg) {
      if (!msg || !msg.thinking) return '';
      var rows = this.messageToolTraceRows(msg);
      if (!rows.length) return '';
      var running = rows.filter(function(row) { return row.state === 'running'; });
      if (running.length) {
        return running.slice(0, 2).map(function(row) { return row.label; }).join(' · ');
      }
      var failed = rows.filter(function(row) { return row.state === 'error' || row.state === 'blocked'; });
      if (failed.length) {
        return failed.slice(0, 2).map(function(row) { return row.label + ' (' + row.state + ')'; }).join(' · ');
      }
      return rows.slice(0, 2).map(function(row) { return row.label; }).join(' · ');
    },

    _workspaceState: function() {
      if (!this._messageWorkspaceState || typeof this._messageWorkspaceState !== 'object') {
        this._messageWorkspaceState = {
          open: false,
          payload: null
        };
      }
      return this._messageWorkspaceState;
    },

    isWorkspacePanelOpen: function() {
      var state = this._workspaceState();
      return !!state.open && !!state.payload;
    },

    closeWorkspacePanel: function() {
      var state = this._workspaceState();
      state.open = false;
      state.payload = null;
    },

    _messageTextPreviewForWorkspace: function(msg) {
      var text = '';
      if (typeof this.extractMessageVisibleText === 'function') {
        text = String(this.extractMessageVisibleText(msg) || '').trim();
      }
      if (!text) text = String(msg && msg.text || '').trim();
      if (text.length > 420) text = text.slice(0, 417).trim() + '...';
      return text;
    },

    _messageArtifactsForWorkspace: function(msg) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var out = [];
      if (row.file_output && row.file_output.path) {
        out.push({ id: 'file-' + String(row.file_output.path), type: 'File', label: String(row.file_output.path), detail: String(row.file_output.bytes || '') });
      }
      if (row.folder_output && row.folder_output.path) {
        out.push({ id: 'folder-' + String(row.folder_output.path), type: 'Folder', label: String(row.folder_output.path), detail: String(row.folder_output.entries || '') + ' entries' });
      }
      if (Array.isArray(row.images) && row.images.length) {
        out.push({ id: 'images-' + row.images.length, type: 'Images', label: String(row.images.length) + ' uploaded image(s)', detail: '' });
      }
      return out;
    },

    openWorkspacePanelForMessage: function(msg, idx, rows) {
      var row = msg && typeof msg === 'object' ? msg : {};
      var state = this._workspaceState();
      var trace = this.messageToolTraceRows(row);
      state.payload = {
        id: String(row.id || ('msg-' + String(idx || 0))).trim(),
        actor: typeof this.messageActorLabel === 'function' ? this.messageActorLabel(row) : String(row.role || 'Message'),
        timestamp: typeof this.messageTs === 'function' ? this.messageTs(row) : '',
        preview: this._messageTextPreviewForWorkspace(row),
        sources: this.messageSourceChips(row),
        trace: trace,
        artifacts: this._messageArtifactsForWorkspace(row),
        rows_count: Array.isArray(rows) ? rows.length : 0
      };
      state.open = true;
    },

    workspacePanelPayload: function() {
      var state = this._workspaceState();
      return state.payload && typeof state.payload === 'object' ? state.payload : null;
    },

    messageRetrySource: function(msg, idx, rows) {
      var list = Array.isArray(rows) ? rows : (Array.isArray(this.messages) ? this.messages : []);
      var rowId = String(msg && msg.id || '').trim();
      var resolvedIndex = Number(idx);
      if (!Number.isFinite(resolvedIndex) || resolvedIndex < 0 || resolvedIndex >= list.length || (list[resolvedIndex] && rowId && String(list[resolvedIndex].id || '').trim() !== rowId)) {
        resolvedIndex = -1;
        for (var li = list.length - 1; li >= 0; li -= 1) {
          var probe = list[li];
          if (!probe) continue;
          if (probe === msg) {
            resolvedIndex = li;
            break;
          }
          if (rowId && String(probe.id || '').trim() === rowId) {
            resolvedIndex = li;
            break;
          }
        }
      }
      if (resolvedIndex < 0) return null;
      for (var i = resolvedIndex; i >= 0; i -= 1) {
        var candidate = list[i];
        if (!candidate || candidate.is_notice) continue;
        var isHumanOrigin = typeof this.messageIsHumanOrigin === 'function'
          ? this.messageIsHumanOrigin(candidate)
          : String(candidate.role || '').toLowerCase() === 'user';
        if (!isHumanOrigin) continue;
        var text = String(candidate.text || '').trim();
        if (!text) continue;
        return candidate;
      }
      return null;
    },

    messageCanRetryFromMeta: function(msg, idx, rows) {
      if (typeof this.messageIsAgentOrigin === 'function' && !this.messageIsAgentOrigin(msg)) {
        return false;
      }
      return !!this.messageRetrySource(msg, idx, rows);
    },

    messageCanForkFromMeta: function(msg) {
      if (!this.currentAgent || !this.currentAgent.id) return false;
      if (typeof this.messageIsAgentOrigin === 'function' && !this.messageIsAgentOrigin(msg)) {
        return false;
      }
      return true;
    },

    _forkAgentRequestedName: function(sourceName) {
      var base = String(sourceName || '').trim();
      if (!base) base = 'agent';
      var requested = base + '-fork';
      if (requested.length > 120) requested = requested.slice(0, 120).trim();
      if (!requested) requested = 'agent-fork';
      return requested;
    },

    retryMessageFromMeta: async function(msg, idx, rows) {
      if (this.sending) return;
      if (typeof this.messageIsAgentOrigin === 'function' && !this.messageIsAgentOrigin(msg)) return;
      var source = this.messageRetrySource(msg, idx, rows);
      if (!source) {
        if (typeof InfringToast !== 'undefined') InfringToast.info('No prior user prompt was found for retry.');
        return;
      }
      var text = String(source.text || '').trim();
      if (!text) {
        if (typeof InfringToast !== 'undefined') InfringToast.info('Retry source is empty.');
        return;
      }
      await this._sendPayload(text, [], [], {
        agent_id: this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '',
        retry_from_meta: true
      });
    },

    forkMessageFromMeta: async function(msg, idx, rows) {
      if (!this.currentAgent || !this.currentAgent.id || this.sending) return;
      void idx;
      void rows;
      if (typeof this.messageCanForkFromMeta === 'function' && !this.messageCanForkFromMeta(msg)) return;
      var sourceAgent = this.currentAgent && typeof this.currentAgent === 'object' ? this.currentAgent : {};
      var sourceAgentId = String(sourceAgent.id || '').trim();
      if (!sourceAgentId) return;
      var sourceAgentName = String(sourceAgent.name || sourceAgentId).trim();
      var requestedName = typeof this._forkAgentRequestedName === 'function'
        ? this._forkAgentRequestedName(sourceAgentName)
        : (sourceAgentName + '-fork');
      try {
        this.cacheCurrentConversation();
        var created = await InfringAPI.post(
          '/api/agents/' + encodeURIComponent(sourceAgentId) + '/clone',
          { new_name: requestedName }
        );
        var forkedAgentId = String(
          (created && (created.agent_id || created.id)) ||
          ''
        ).trim();
        if (!forkedAgentId) {
          throw new Error('agent_clone_failed');
        }
        var forkedAgentName = String((created && created.name) || requestedName || forkedAgentId).trim();
        var store = Alpine.store('app');
        if (store && typeof store.refreshAgents === 'function') {
          await store.refreshAgents({ force: true });
        }
        var resolvedForkedAgent = this.resolveAgent(forkedAgentId);
        if (!resolvedForkedAgent) {
          resolvedForkedAgent = {
            id: forkedAgentId,
            name: forkedAgentName,
            role: String(sourceAgent.role || 'analyst')
          };
        }
        this.selectAgent(resolvedForkedAgent);
        if (typeof InfringToast !== 'undefined') {
          InfringToast.success('Forked to new agent "' + forkedAgentName + '"');
        }
      } catch (e) {
        if (typeof InfringToast !== 'undefined') InfringToast.error('Failed to fork message: ' + (e && e.message ? e.message : 'unknown error'));
      }
    },
