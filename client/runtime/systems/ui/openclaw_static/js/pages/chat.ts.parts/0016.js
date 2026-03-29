              text: errorText,
              meta: '',
              tools: [],
              system_origin: 'runtime:error',
              ts: Date.now()
            });
            self2._inflightPayload = null;
            self2.scrollToBottom();
            self2.$nextTick(function() {
              var el = document.getElementById('msg-input'); if (el) el.focus();
              self2._processQueue();
            });
            self2.refreshPromptSuggestions(true, 'post-error');
          });
          break;

        case 'agent_archived':
          this.setAgentLiveActivity(
            data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''),
            'idle'
          );
          this._clearPendingWsRequest(data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''));
          this.handleAgentInactive(
            data && data.agent_id ? String(data.agent_id) : (this.currentAgent && this.currentAgent.id ? this.currentAgent.id : ''),
            data && data.reason ? String(data.reason) : 'archived'
          );
          break;

        case 'agents_updated':
          if (data.agents) {
            Alpine.store('app').agents = data.agents;
            Alpine.store('app').agentCount = data.agents.length;
          }
          break;

        case 'command_result':
          this.applyContextTelemetry(data);
          var isContextTelemetryResult = Object.prototype.hasOwnProperty.call(data || {}, 'context_tokens') ||
            Object.prototype.hasOwnProperty.call(data || {}, 'context_window') ||
            Object.prototype.hasOwnProperty.call(data || {}, 'context_ratio') ||
            Object.prototype.hasOwnProperty.call(data || {}, 'context_pressure');
          if (!data.silent && !isContextTelemetryResult) {
            this.messages.push({ id: ++msgId, role: 'system', text: data.message || 'Command executed.', meta: '', tools: [], system_origin: 'command:result' });
            this.scrollToBottom();
          }
          break;

        case 'terminal_output':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !(m && m.terminal && m.thinking); });
          var stdout = typeof data.stdout === 'string' ? data.stdout : '';
          var stderr = typeof data.stderr === 'string' ? data.stderr : '';
          var termText = '';
          if (stdout.trim()) termText += stdout;
          if (stderr.trim()) termText += (termText ? '\n' : '') + stderr;
          if (!termText.trim()) termText = '(no output)';
          var termMeta = 'exit ' + (Number.isFinite(Number(data.exit_code)) ? String(Number(data.exit_code)) : '1');
          var termDuration = this.formatResponseDuration(Number(data.duration_ms || 0));
          if (termDuration) termMeta += ' | ' + termDuration;
          var termCwd = this.terminalPromptPath;
          if (data.cwd) {
            termCwd = String(data.cwd);
            this.terminalCwd = termCwd;
            termMeta += ' | ' + termCwd;
          }
          this._appendTerminalMessage({
            role: 'terminal',
            text: termText,
            meta: termMeta,
            tools: [],
            ts: Date.now(),
            terminal_source: data && data.terminal_source ? String(data.terminal_source).toLowerCase() : 'user',
            cwd: termCwd
          });
          this.sending = false;
          this._responseStartedAt = 0;
          this.scrollToBottom();
          this.$nextTick(() => this._processQueue());
          this.refreshPromptSuggestions(true, 'post-terminal');
          break;

        case 'terminal_error':
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !(m && m.terminal && m.thinking); });
          this._appendTerminalMessage({
            role: 'terminal',
            text: 'Terminal error: ' + (data && data.message ? data.message : 'command failed'),
            meta: '',
            tools: [],
            ts: Date.now(),
            terminal_source: data && data.terminal_source ? String(data.terminal_source).toLowerCase() : 'user',
            cwd: this.terminalPromptPath
          });
          this.sending = false;
          this._responseStartedAt = 0;
          this.scrollToBottom();
          this.$nextTick(() => this._processQueue());
          break;

        case 'canvas':
          // Agent presented an interactive canvas — render it in an iframe sandbox
          var canvasHtml = '<div class="canvas-panel" style="border:1px solid var(--border);border-radius:8px;margin:8px 0;overflow:hidden;">';
          canvasHtml += '<div style="padding:6px 12px;background:var(--surface);border-bottom:1px solid var(--border);font-size:0.85em;display:flex;justify-content:space-between;align-items:center;">';
          canvasHtml += '<span>' + (data.title || 'Canvas') + '</span>';
          canvasHtml += '<span style="opacity:0.5;font-size:0.8em;">' + (data.canvas_id || '').substring(0, 8) + '</span></div>';
          canvasHtml += '<iframe sandbox="allow-scripts" srcdoc="' + (data.html || '').replace(/"/g, '&quot;') + '" ';
          canvasHtml += 'style="width:100%;min-height:300px;border:none;background:#fff;" loading="lazy"></iframe></div>';
          this.messages.push({ id: ++msgId, role: 'agent', text: canvasHtml, meta: 'canvas', isHtml: true, tools: [] });
          this.scrollToBottom();
          break;

        case 'pong': break;
      }
      this.scheduleConversationPersist();
    },

    // Format timestamp for display
    formatTime: function(ts) {
      if (!ts) return '';
      var d = new Date(ts);
      if (Number.isNaN(d.getTime())) return '';
      var h = d.getHours();
      var m = d.getMinutes();
      var ampm = h >= 12 ? 'PM' : 'AM';
      h = h % 12 || 12;
      return h + ':' + (m < 10 ? '0' : '') + m + ' ' + ampm;
    },

    isSameDay: function(a, b) {
      if (!a || !b) return false;
      return (
        a.getFullYear() === b.getFullYear() &&
        a.getMonth() === b.getMonth() &&
        a.getDate() === b.getDate()
      );
    },

    // UI-safe timestamp formatter for templates
    messageTs: function(msg) {
      if (!msg || !msg.ts) return '';
      var ts = new Date(msg.ts);
      if (Number.isNaN(ts.getTime())) return '';
      var now = new Date();
      if (this.isSameDay(ts, now)) return this.formatTime(ts);
      var yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      if (this.isSameDay(ts, yesterday)) {
        return 'Yesterday at ' + this.formatTime(ts);
      }
      var dateText = ts.toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
      return dateText + ' at ' + this.formatTime(ts);
    },

    parseProgressFromText: function(text) {
      var value = String(text || '');
      if (!value) return null;
      var explicit = value.match(/\[\[\s*progress\s*:\s*([0-9]{1,3})(?:\s*\/\s*([0-9]{1,3}))?\s*\]\]/i);
      if (explicit) {
        var part = Number(explicit[1] || 0);
        var total = Number(explicit[2] || 100);
        if (Number.isFinite(part) && Number.isFinite(total) && total > 0) {
          var pct = Math.max(0, Math.min(100, Math.round((part / total) * 100)));
          return { percent: pct, label: 'Progress ' + pct + '%' };
        }
      }
      var percent = value.match(/\bprogress(?:\s+is)?\s*[:=-]?\s*([0-9]{1,3})\s*%/i);
      if (percent) {
        var p = Number(percent[1] || 0);
        if (Number.isFinite(p)) {
          var clamped = Math.max(0, Math.min(100, Math.round(p)));
          return { percent: clamped, label: 'Progress ' + clamped + '%' };
        }
      }
      return null;
    },

    messageProgress: function(msg) {
      if (!msg || msg.terminal || msg.is_notice) return null;
      var key = String(msg.id || '') + '|' + String(msg.text || '').length + '|' + String(msg.meta || '').length;
      if (!this._progressCache || typeof this._progressCache !== 'object') this._progressCache = {};
      var keys = Object.keys(this._progressCache);
      if (keys.length > 4096) {
        this._progressCache = {};
      }
      if (Object.prototype.hasOwnProperty.call(this._progressCache, key)) return this._progressCache[key];

      var progress = null;
      if (msg.progress && typeof msg.progress === 'object') {
        var pct = Number(msg.progress.percent);
        if (Number.isFinite(pct)) {
          progress = {
            percent: Math.max(0, Math.min(100, Math.round(pct))),
            label: String(msg.progress.label || ('Progress ' + Math.round(pct) + '%')).trim()
          };
        }
      }
      if (!progress) progress = this.parseProgressFromText(msg.text || '');
      this._progressCache[key] = progress;
      return progress;
    },

    progressFillStyle: function(msg) {
      var progress = this.messageProgress(msg);
      if (!progress) return 'width:0%';
      return 'width:' + progress.percent + '%';
    },

    messageDomId: function(msg, idx) {
      var suffix = (msg && msg.id != null) ? String(msg.id) : String(idx || 0);
      return 'chat-msg-' + suffix;
    },

    messageRoleClass: function(msg) {
      if (msg && msg.terminal) {
        return this.terminalMessageSource(msg) === 'user' ? 'terminal terminal-user' : 'terminal terminal-agent';
      }
      if (!msg || !msg.role) return 'agent';
      return String(msg.role);
    },

    terminalMessageSource: function(msg) {
      if (!msg || !msg.terminal) return 'agent';
      var source = String(msg.terminal_source || '').trim().toLowerCase();
      if (source === 'user' || source === 'agent' || source === 'system') return source;
      return 'agent';
    },

    terminalToolboxSideClass: function(msg) {
      return this.terminalMessageSource(msg) === 'user' ? 'terminal-toolbox-right' : 'terminal-toolbox-left';
    },

    terminalMessageCollapsed: function(msg, idx, rows) {
      if (!msg || !msg.terminal || msg.thinking) return false;
      var list = Array.isArray(rows) ? rows : this.messages;
      if (!Array.isArray(list) || idx < 0 || idx >= list.length) return false;
      for (var i = idx + 1; i < list.length; i++) {
        var row = list[i];
        if (!row || row.is_notice || row.terminal || row.thinking) continue;
        var hasText = typeof row.text === 'string' && row.text.trim().length > 0;
        var hasTools = Array.isArray(row.tools) && row.tools.length > 0;
        var hasArtifact = !!(row.file_output || row.folder_output);
        if (hasText || hasTools || hasArtifact) return true;
      }
      return false;
    },

    terminalToolboxPreview: function(msg) {
      if (!msg || !msg.terminal) return '';
      var text = String(msg.text || '').trim();
      if (!text) return 'Command completed';
      var first = text.split('\n')[0] || '';
      var compact = first.replace(/\s+/g, ' ').trim();
      if (!compact) return 'Command completed';
      if (compact.length > 108) return compact.slice(0, 105) + '...';
      return compact;
    },

    thinkingDisplayText: function(msg) {
      var value = String(msg && msg.text ? msg.text : '').trim();
      if (!value) return 'Thinking...';
      value = value.replace(/^\*+|\*+$/g, '').trim();
      return value || 'Thinking...';
    },

    messageGroupRole: function(msg) {
      if (!msg) return '';
      if (msg.terminal) return 'terminal';
      return String(msg.role || '');
    },

    messageSourceKey: function(msg) {
      if (!msg || msg.is_notice) return '';
      if (msg.terminal) {
        var terminalAgentId = String((msg && msg.agent_id) || (this.currentAgent && this.currentAgent.id) || '').trim();
        return terminalAgentId ? ('terminal:' + terminalAgentId) : 'terminal';
      }
      var role = String(msg.role || '').trim().toLowerCase();
      if (!role) return '';
      if (role === 'user') return 'user';
      if (role === 'system') {
        var systemOrigin = String(
          (msg && msg.system_origin) ||
          (msg && msg.agent_origin) ||
          (msg && msg.agent_id) ||
          (msg && msg.actor_id) ||
          (msg && msg.actor) ||
          ''
        ).trim();
        if (systemOrigin) return 'system:' + systemOrigin.toLowerCase();
        // Legacy/cached rows may not carry system_origin; avoid collapsing all
        // such rows into one visual run.
        var legacySystemId = String(
          (msg && msg.id != null) ? msg.id : ((msg && msg.ts != null) ? msg.ts : '')
        ).trim();
        if (legacySystemId) return 'system:legacy:' + legacySystemId.toLowerCase();
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
      if (!prev || prev.is_notice) return true;
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
      if (!next || next.is_notice) return true;
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

