    messageGroupRole: function(msg) {
      if (!msg) return '';
      if (msg.terminal) return 'terminal';
      return String(msg.role || '');
    },

    shouldReloadHistoryForFinalEventPayload: function(payload) {
      return !!(
        payload &&
        typeof payload === 'object' &&
        String(payload.state || '').trim().toLowerCase() === 'final'
      );
    },

    parseChatSideResult: function(payload) {
      if (!payload || typeof payload !== 'object') return null;
      var candidate = payload;
      if (candidate.kind !== 'btw') return null;
      var runId = String(candidate.runId || '').trim();
      var sessionKey = String(candidate.sessionKey || '').trim();
      var question = String(candidate.question || '').trim();
      var text = String(candidate.text || '').trim();
      if (!(runId && sessionKey && question && text)) return null;
      return {
        kind: 'btw',
        runId: runId,
        sessionKey: sessionKey,
        question: question,
        text: text,
        isError: candidate.isError === true,
        ts:
          typeof candidate.ts === 'number' && Number.isFinite(candidate.ts)
            ? candidate.ts
            : Date.now()
      };
    },

    appendChatSideResultNotice: function(payload) {
      var parsed = this.parseChatSideResult(payload);
      if (!parsed) return false;
      this.messages.push({
        id: ++msgId,
        role: 'system',
        text: parsed.text,
        meta: '',
        tools: [],
        system_origin: parsed.isError ? 'runtime:btw:error' : 'runtime:btw',
        notice_label: 'Background note: ' + parsed.question,
        notice_type: parsed.isError ? 'warn' : 'info',
        run_id: parsed.runId,
        session_key: parsed.sessionKey,
        ts: parsed.ts
      });
      this.scrollToBottom();
      return true;
    },

    isStackBoundaryNoticeMessage: function(msg) {
      if (!msg || msg.terminal) return false;
      if (msg.is_notice) return true;
      if (msg.notice_label || msg.notice_type || msg.notice_action) return true;
      var role = String(msg.role || '').trim().toLowerCase();
      if (role !== 'system') return false;
      var text = String(msg.text || '').trim();
      if (!text) return false;
      if (this.isModelSwitchNoticeLabel(text)) return true;
      if (/^changed name from\s+/i.test(text)) return true;
      if (/^initialized\s+.+\s+as\s+/i.test(text)) return true;
      return false;
    },

    messageSourceKey: function(msg) {
      if (!msg || msg.is_notice) return '';
      if (this.isStackBoundaryNoticeMessage(msg)) {
        var noticeLabel = String(msg.notice_label || msg.text || '').trim().toLowerCase();
        var noticeTs = Number(msg.ts || 0) || 0;
        return 'notice:' + noticeLabel + ':' + noticeTs;
      }
      if (msg.terminal) {
        var terminalSource = this.terminalMessageSource(msg);
        if (terminalSource === 'user') return 'terminal:user';
        if (terminalSource === 'system') return 'terminal:system';
        var terminalAgentId = String((msg && msg.agent_id) || (this.currentAgent && this.currentAgent.id) || '').trim();
        return terminalAgentId ? ('terminal:agent:' + terminalAgentId.toLowerCase()) : 'terminal:agent';
      }
      var role = String(msg.role || '').trim().toLowerCase();
      if (!role) return '';
      if (role === 'user') return 'user';
      if (role === 'system') {
        // System rows should stack as one source-run when consecutive, regardless
        // of internal origin tags (inject:test, runtime:error, slash:status, etc).
        // This keeps UI grouping consistent for user-facing system narration.
        return 'system';
      }
      if (role === 'agent') {
        var agentOrigin = String(
          (msg && msg.agent_origin) ||
          (msg && msg.source_agent_id) ||
          (msg && msg.agent_id) ||
          (msg && msg.actor_id) ||
          (msg && msg.actor) ||
          (msg && msg.agent_name) ||
          ''
        ).trim();
        if (!agentOrigin && this.currentAgent && this.currentAgent.id) {
          agentOrigin = String(this.currentAgent.id || '').trim();
        }
        return agentOrigin ? ('agent:' + agentOrigin.toLowerCase()) : 'agent';
      }
      var genericOrigin = String(
        (msg && msg.agent_id) ||
        (msg && msg.actor_id) ||
        (msg && msg.actor) ||
        ''
      ).trim();
      return genericOrigin ? (role + ':' + genericOrigin.toLowerCase()) : role;
    },

    isFirstInSourceRun: function(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx === 0) return true;
      var prev = list[idx - 1];
      if (!prev || this.isStackBoundaryNoticeMessage(prev)) return true;
      var prevKey = this.messageSourceKey(prev);
      return prevKey !== currKey;
    },

    isLastInSourceRun: function(idx, rows) {
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      var curr = list[idx];
      if (!curr || curr.is_notice) return false;
      var currKey = this.messageSourceKey(curr);
      if (!currKey) return false;
      if (idx >= list.length - 1) return true;
      var next = list[idx + 1];
      if (!next || this.isStackBoundaryNoticeMessage(next)) return true;
      var nextKey = this.messageSourceKey(next);
      return nextKey !== currKey;
    },

    messagePreview: function(msg) {
      if (!msg) return '';
      if (msg.is_notice && msg.notice_label) {
        return String(msg.notice_label);
      }
      var raw = '';
      if (typeof msg.text === 'string' && msg.text.trim()) {
        raw = msg.text;
      } else if (Array.isArray(msg.tools) && msg.tools.length) {
        raw = 'Tool calls: ' + msg.tools.map(function(tool) {
          return tool && tool.name ? tool.name : 'tool';
        }).join(', ');
      } else {
        raw = '[' + (msg.role || 'message') + ']';
      }
      var compact = raw.replace(/\s+/g, ' ').trim();
      if (compact.length > 140) return compact.slice(0, 137) + '...';
      return compact;
    },

    messageMapPreview: function(msg) {
      if (this.messageMapMarkerType(msg) === 'tool') {
        return this.messageToolPreview(msg);
      }
      return this.messagePreview(msg);
    },

    appendAgentTerminalTranscript: function(rows) {
      if (!Array.isArray(rows) || !rows.length || typeof this._appendTerminalMessage !== 'function') return false;
      var appended = false;
      for (var i = 0; i < rows.length; i++) {
        var row = rows[i] || {};
        var cwd = row.cwd ? String(row.cwd) : this.terminalPromptPath;
        var command = row.command ? String(row.command).trim() : '';
        var output = row.output ? String(row.output).trim() : '';
        if (command) {
          this._appendTerminalMessage({ role: 'terminal', text: this._terminalPromptLine(cwd, command), meta: cwd, tools: [], ts: Date.now(), terminal_source: 'agent', cwd: cwd });
          appended = true;
        }
        if (output) {
          this._appendTerminalMessage({ role: 'terminal', text: output, meta: row.is_error ? 'command failed' : 'command output', tools: [], ts: Date.now(), terminal_source: 'system', cwd: cwd, _terminal_compact: output.length > 500 });
          appended = true;
        }
      }
      return appended;
    },

    isThinkingPlaceholderText: function(input) {
      var value = String(input || '').replace(/<[^>]*>/g, ' ').replace(/\*+/g, '').replace(/\s+/g, ' ').trim().toLowerCase();
      if (!value) return true;
      if (/^(thinking|processing|working|preparing response|reasoning through context)(\.\.\.|…)?$/.test(value)) return true;
      if (/^(using|calling)\b.+(\.\.\.|…)?$/.test(value)) return true;
      var stripped = value.replace(/[.,!?;:…-]+/g, ' ').replace(/\s+/g, ' ').trim();
      if (stripped) {
        var words = stripped.split(' ').filter(function(part) { return !!part; });
        var placeholderLexicon = {
          thinking: true,
          processing: true,
          working: true,
          preparing: true,
          response: true,
          reasoning: true,
          through: true,
          context: true
        };
        if (words.length > 0 && words.length <= 24) {
          var allPlaceholder = words.every(function(word) {
            return !!placeholderLexicon[word];
          });
          if (allPlaceholder) return true;
        }
      }
      return false;
    },

    normalizeThinkingStatusCandidate: function(rawStatus) {
      var value = String(rawStatus || '').replace(/\r/g, '\n').trim();
      if (!value) return '';
      var lines = value
        .split('\n')
        .map(function(line) { return String(line || '').replace(/\s+/g, ' ').trim(); })
        .filter(function(line) { return !!line; });
      if (!lines.length) return '';
      for (var i = 0; i < lines.length; i++) {
        var line = String(lines[i] || '').trim();
        if (!line) continue;
        if (this.isThinkingPlaceholderText(line)) continue;
        line = line.replace(/\[(?:end|done|start)\]/ig, '').replace(/\s+/g, ' ').trim();
        if (!line) continue;
        var lowered = line.toLowerCase();
        if (/^(active|idle|running)$/.test(lowered)) continue;
        if (/^phase[:\s]/.test(lowered)) {
          line = line.replace(/^phase[:\s]*/i, '').trim();
          lowered = line.toLowerCase();
        }
        if (/web[_\s-]?search|searching (the )?(web|internet)|duckduckgo|serp/.test(lowered)) {
          line = 'Searching internet';
        } else if (/web[_\s-]?fetch|reading web|browse|browsing/.test(lowered)) {
          line = 'Reading web pages';
        } else if (/read(_|\s)?file|file read|reading files?/.test(lowered)) {
          line = 'Scanning files';
        } else if (/folder|directory|filesystem scan|scan folders?/.test(lowered)) {
          line = 'Scanning folders';
        } else if (/terminal|shell|command execution|run command/.test(lowered)) {
          line = 'Running terminal command';
        } else if (/spawn_subagents|spawn_swarm|subagents?|swarm|parallel workers?/.test(lowered)) {
          line = 'Summoning agents';
        } else if (/memory.*query|semantic memory|vector search/.test(lowered)) {
          line = 'Searching memory';
        } else if (/context warning|context limit|context window/.test(lowered)) {
          line = 'Context window warning';
        }
        line = String(line || '').replace(/\s+/g, ' ').trim();
        if (!line || this.isThinkingPlaceholderText(line)) continue;
        if (line.length > 220) line = line.slice(0, 217) + '...';
        return line;
      }
      return '';
    },

    messageToolPreview: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) {
        return this.messagePreview(msg);
      }
      var self = this;
      var compactToolText = function(value, maxLen) {
        if (value == null) return '';
        var raw = '';
        if (typeof value === 'string') {
          raw = value;
        } else {
          try {
            raw = JSON.stringify(value);
          } catch (e) {
            raw = String(value);
          }
        }
        var compact = raw.replace(/\s+/g, ' ').trim();
        if (!compact) return '';
        if (compact.length > maxLen) return compact.slice(0, maxLen - 3) + '...';
        return compact;
      };
