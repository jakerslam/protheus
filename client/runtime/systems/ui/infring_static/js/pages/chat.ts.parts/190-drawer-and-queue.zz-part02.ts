    isBlockedTool: function(tool) {
      if (!tool) return false;
      if (tool.blocked === true) return true;
      var txt = String(tool.result || '').toLowerCase();
      if (String(tool.status || '').toLowerCase() === 'blocked') return true;
      if (!tool.is_error) return false;
      return (
        txt.indexOf('blocked') >= 0 ||
        txt.indexOf('policy') >= 0 ||
        txt.indexOf('denied') >= 0 ||
        txt.indexOf('not allowed') >= 0 ||
        txt.indexOf('forbidden') >= 0 ||
        txt.indexOf('approval') >= 0 ||
        txt.indexOf('permission') >= 0 ||
        txt.indexOf('fail-closed') >= 0
      );
    },

    isToolSuccessful: function(tool) {
      if (!tool) return false;
      if (tool.running) return false;
      if (this.isBlockedTool(tool)) return false;
      if (tool.is_error) return false;
      return true;
    },

    isThoughtTool: function(tool) {
      return !!(tool && String(tool.name || '').toLowerCase() === 'thought_process');
    },

    toolNameKey: function(tool) {
      if (!tool) return '';
      return String(tool.name || '')
        .trim()
        .toLowerCase()
        .replace(/[\s-]+/g, '_');
    },

    toolInputPayload: function(tool) {
      if (!tool || typeof tool !== 'object') return null;
      var raw = String(tool.input || tool.args || tool.arguments || '').trim();
      if (!raw) return null;
      if (raw.indexOf('<function=') >= 0 && raw.indexOf('{') >= 0) {
        raw = raw.slice(raw.indexOf('{')).trim();
      }
      if (!(raw.charAt(0) === '{' || raw.charAt(0) === '[')) return null;
      try {
        var parsed = JSON.parse(raw);
        return parsed && typeof parsed === 'object' ? parsed : null;
      } catch (_) {
        return null;
      }
    },

    toolPayloadCount: function(payload, keys) {
      if (!payload || typeof payload !== 'object') return 0;
      var list = Array.isArray(keys) ? keys : [];
      for (var i = 0; i < list.length; i++) {
        var key = list[i];
        if (!Object.prototype.hasOwnProperty.call(payload, key)) continue;
        var value = payload[key];
        if (Array.isArray(value)) return value.length;
        if (typeof value === 'number' && Number.isFinite(value)) return Math.max(0, Math.round(value));
        if (typeof value === 'string' && value.trim()) return 1;
      }
      return 0;
    },

    toolDisplayName: function(tool) {
      if (!tool) return 'tool';
      if (this.isThoughtTool(tool)) return 'thought';
      var key = this.toolNameKey(tool);
      switch (key) {
        case 'web_search':
        case 'search_web':
        case 'search':
        case 'web_query':
          return 'Web search';
        case 'web_fetch':
        case 'browse':
        case 'web_conduit_fetch':
          return 'Web fetch';
        case 'file_read':
        case 'read_file':
        case 'file':
          return 'File read';
        case 'folder_export':
        case 'list_folder':
        case 'folder_tree':
        case 'folder':
          return 'Folder export';
        case 'terminal_exec':
        case 'run_terminal':
        case 'terminal':
        case 'shell_exec':
          return 'Terminal command';
        case 'spawn_subagents':
        case 'spawn_swarm':
        case 'agent_spawn':
        case 'sessions_spawn':
          return 'Swarm spawn';
        case 'memory_semantic_query':
          return 'Memory query';
        case 'cron_schedule':
          return 'Schedule task';
        case 'cron_run':
          return 'Run scheduled task';
        case 'cron_list':
          return 'List schedules';
        case 'session_rollback_last_turn':
          return 'Undo last turn';
        default:
          return String(tool.name || 'tool');
      }
    },

    toolThinkingActionLabel: function(tool) {
      if (!tool) return '';
      if (this.isThoughtTool(tool)) return 'Thinking';
      var key = this.toolNameKey(tool);
      var payload = this.toolInputPayload(tool);
      switch (key) {
        case 'web_search':
        case 'search_web':
        case 'search':
        case 'web_query':
          return 'Searching internet';
        case 'web_fetch':
        case 'browse':
        case 'web_conduit_fetch':
          return 'Reading web pages';
        case 'file_read':
        case 'read_file':
        case 'file':
          var fileCount = this.toolPayloadCount(payload, ['paths', 'files', 'file_paths', 'targets', 'path', 'file']);
          if (fileCount > 1) return 'Scanning ' + fileCount + ' files';
          if (fileCount === 1) return 'Scanning 1 file';
          return 'Scanning files';
        case 'folder_export':
        case 'list_folder':
        case 'folder_tree':
        case 'folder':
          var folderCount = this.toolPayloadCount(payload, ['folders', 'paths', 'targets', 'path', 'folder']);
          if (folderCount > 1) return 'Scanning ' + folderCount + ' folders';
          if (folderCount === 1) return 'Scanning 1 folder';
          return 'Scanning folders';
        case 'terminal_exec':
        case 'run_terminal':
        case 'terminal':
        case 'shell_exec':
          return 'Running terminal command';
        case 'spawn_subagents':
        case 'spawn_swarm':
        case 'agent_spawn':
        case 'sessions_spawn':
          var spawnCount = this.toolPayloadCount(payload, ['count', 'agent_count', 'num_agents', 'agents']);
          if (spawnCount > 0) return 'Summoning ' + spawnCount + ' agents';
          return 'Summoning agents';
        case 'memory_semantic_query':
          return 'Searching memory';
        case 'cron_schedule':
          return 'Scheduling follow-up work';
        case 'cron_run':
          return 'Running scheduled work';
        case 'cron_list':
          return 'Checking schedules';
        case 'session_rollback_last_turn':
          return 'Rewinding the last turn';
        default:
          return 'Running ' + this.toolDisplayName(tool);
      }
    },

    currentToolDialogLabel: function(msg) {
      if (!msg || !Array.isArray(msg.tools) || !msg.tools.length) return '';
      for (var i = msg.tools.length - 1; i >= 0; i--) {
        var tool = msg.tools[i];
        if (!tool || this.isThoughtTool(tool) || !tool.running) continue;
        return this.toolThinkingActionLabel(tool);
      }
      return '';
    },

    thoughtToolDurationSeconds: function(tool) {
      if (!tool || typeof tool !== 'object') return 0;
      var ms = Number(tool.duration_ms || tool.durationMs || tool.elapsed_ms || 0);
      if (!Number.isFinite(ms) || ms < 0) ms = 0;
      var seconds = Math.round(ms / 1000);
      if (ms > 0 && seconds < 1) seconds = 1;
      return Math.max(0, seconds);
    },

    thoughtToolLabel: function(tool) {
      return 'Thought for ' + this.thoughtToolDurationSeconds(tool) + ' seconds';
    },

    toolStatusText: function(tool) {
      if (!tool) return '';
      if (tool.running) return 'running...';
      if (this.isThoughtTool(tool)) return 'thought';
      if (this.isBlockedTool(tool)) return 'blocked';
      if (tool.is_error) return 'error';
      if (tool.result) {
        return tool.result.length > 500 ? Math.round(tool.result.length / 1024) + 'KB' : 'done';
      }
      return 'done';
    },

    // Mark chat-rendered error messages for styling
    isErrorMessage: function(msg) {
      if (!msg || !msg.text) return false;
      if (String(msg.role || '').toLowerCase() !== 'system') return false;
      var t = String(msg.text).trim().toLowerCase();
      return t.startsWith('error:');
    },

    messageHasTools: function(msg) {
      return !!(msg && Array.isArray(msg.tools) && msg.tools.length);
    },

    allToolsCollapsed: function(msg) {
      if (!this.messageHasTools(msg)) return true;
      return !msg.tools.some(function(tool) {
        return !!(tool && tool.expanded);
      });
    },

    toggleMessageTools: function(msg) {
      if (!this.messageHasTools(msg)) return;
      var expand = this.allToolsCollapsed(msg);
      msg.tools.forEach(function(tool) {
        if (tool) tool.expanded = expand;
      });
      this.scheduleConversationPersist();
    },

    // Copy message text to clipboard
    copyMessage: function(msg) {
      var text = msg.text || '';
      navigator.clipboard.writeText(text).then(function() {
        msg._copied = true;
        setTimeout(function() { msg._copied = false; }, 2000);
      }).catch(function() {});
    },

    prefersReducedMotion: function() {
      if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return false;
      try {
        return !!window.matchMedia('(prefers-reduced-motion: reduce)').matches;
      } catch (_) {
        return false;
      }
    },

    captureComposerSendMorph: function(textInput) {
      if (this.prefersReducedMotion() || this.terminalMode || this.showFreshArchetypeTiles) return null;
      if (typeof document === 'undefined') return null;
      var shell = document.querySelector('.input-row .composer-shell');
      var input = document.getElementById('msg-input');
      if (!shell || !input) return null;
      var text = String(textInput == null ? '' : textInput).trim();
      if (!text) return null;
      var rect = input.getBoundingClientRect();
      if (!(rect.width > 80 && rect.height > 24)) return null;
      var ghost = document.createElement('div');
      ghost.className = 'composer-send-morph-ghost';
      ghost.textContent = text.length > 260 ? (text.slice(0, 257) + '...') : text;
      ghost.style.left = rect.left + 'px';
      ghost.style.top = rect.top + 'px';
      ghost.style.width = rect.width + 'px';
      ghost.style.minHeight = rect.height + 'px';
      document.body.appendChild(ghost);
      shell.classList.add('composer-shell-send-morph');
      return { shell: shell, ghost: ghost };
    },

    clearComposerSendMorph: function(snapshot) {
      if (!snapshot || typeof snapshot !== 'object') return;
      if (snapshot.shell && snapshot.shell.classList) snapshot.shell.classList.remove('composer-shell-send-morph');
      if (snapshot.ghost && snapshot.ghost.parentNode) snapshot.ghost.parentNode.removeChild(snapshot.ghost);
    },

    playComposerSendMorphToMessage: function(snapshot, messageId) {
      if (!snapshot || !snapshot.ghost) return;
      if (this.prefersReducedMotion()) {
        snapshot.ghost.style.opacity = '0.56';
        setTimeout(this.clearComposerSendMorph.bind(this, snapshot), 240);
        return;
      }
      var row = document.getElementById('chat-msg-' + String(messageId || '').trim());
      var bubble = row ? row.querySelector('.message-bubble') : null;
      if (!bubble) {
        this.clearComposerSendMorph(snapshot);
        return;
      }
      var rect = bubble.getBoundingClientRect();
      if (!(rect.width > 24 && rect.height > 20)) {
        this.clearComposerSendMorph(snapshot);
        return;
      }
      var ghost = snapshot.ghost;
      var self = this;
      ghost.classList.add('in-flight');
      var finish = function() { self.clearComposerSendMorph(snapshot); };
      ghost.addEventListener('transitionend', finish, { once: true });
      requestAnimationFrame(function() {
        ghost.style.left = rect.left + 'px';
        ghost.style.top = rect.top + 'px';
        ghost.style.width = rect.width + 'px';
        ghost.style.minHeight = rect.height + 'px';
        ghost.style.opacity = '0.2';
      });
      setTimeout(finish, 760);
    },

    appendUserChatMessage: function(finalText, msgImages, options) {
      var opts = options && typeof options === 'object' ? options : {};
      var text = String(finalText == null ? '' : finalText);
      var images = Array.isArray(msgImages) ? msgImages : [];
      if (!String(text || '').trim() && !images.length) return;
      var msg = {
        id: ++msgId,
        role: 'user',
        text: text,
        meta: '',
        tools: [],
        images: images,
        ts: Number.isFinite(Number(opts.ts)) ? Number(opts.ts) : Date.now()
      };
      this.messages.push(msg);
      this._stickToBottom = true;
      this.scrollToBottom({ force: true, stabilize: true });
      localStorage.setItem('of-first-msg', 'true');
      this.promptSuggestions = [];
      if (!opts.deferPersist) this.scheduleConversationPersist();
      return msg;
    },

    // Process queued messages after current response completes
    _processQueue: function() {
      if (!this.messageQueue.length || this.sending || this._inflightFailoverInProgress) return;
      var next = this.messageQueue.shift();
      if (next && next.terminal) {
        this._sendTerminalPayload(next.command);
        return;
      }
      var nextText = String(next && next.text ? next.text : '');
      var nextFiles = Array.isArray(next && next.files) ? next.files : [];
      var nextImages = Array.isArray(next && next.images) ? next.images : [];
      if (!nextText.trim() && !nextFiles.length) {
        var self = this;
        this.$nextTick(function() { self._processQueue(); });
        return;
      }
      this.appendUserChatMessage(nextText, nextImages, { deferPersist: true });
      this.scheduleConversationPersist();
      this._sendPayload(nextText, nextFiles, nextImages, {
        from_queue: true,
        queue_id: next && next.queue_id ? String(next.queue_id) : ''
      });
    },

    _terminalPromptLine: function(cwd, command) {
      var path = String(cwd || this.terminalPromptPath || '/workspace');
      var cmd = String(command || '').trim();
      if (!cmd) return path + ' %';
      return path + ' % ' + cmd;
    },

    _appendTerminalMessage: function(entry) {
      var payload = entry || {};
      var text = String(payload.text || '').replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/^\s+|\s+$/g, '');
      var now = Date.now();
      var ts = Number.isFinite(Number(payload.ts)) ? Number(payload.ts) : now;
      var role = payload.role ? String(payload.role) : 'terminal';
      var terminalSource = payload.terminal_source ? String(payload.terminal_source).toLowerCase() : '';
      if (terminalSource !== 'user' && terminalSource !== 'agent' && terminalSource !== 'system') {
        terminalSource = role === 'user' ? 'user' : 'system';
      }
      var cwd = payload.cwd ? String(payload.cwd) : this.terminalPromptPath;
      var meta = payload.meta == null ? '' : String(payload.meta);
      var tools = Array.isArray(payload.tools) ? payload.tools : [];
      var shouldAppendToLast = payload.append_to_last === true;
      var agentId = payload.agent_id ? String(payload.agent_id) : '';
      var agentName = payload.agent_name ? String(payload.agent_name) : '';
      if (terminalSource === 'agent') {
        if (!agentId && this.currentAgent && this.currentAgent.id) agentId = String(this.currentAgent.id);
        if (!agentName && this.currentAgent && this.currentAgent.name) agentName = String(this.currentAgent.name);
      }

      var last = this.messages.length ? this.messages[this.messages.length - 1] : null;
      if (shouldAppendToLast && last && !last.thinking && last.terminal) {
        if (text) {
          if (last.text && !/\n$/.test(last.text)) last.text += '\n';
          last.text += text.replace(/^[\r\n]+/, '');
        }
        if (meta) last.meta = meta;
        if (cwd) {
          last.cwd = cwd;
          this.terminalCwd = cwd;
        }
        if (terminalSource) last.terminal_source = terminalSource;
        if (agentId) last.agent_id = agentId;
        if (agentName) last.agent_name = agentName;
        last.ts = ts;
        if (!Array.isArray(last.tools)) last.tools = [];
        if (tools.length) last.tools = last.tools.concat(tools);
        return last;
      }

      var msg = {
        id: ++msgId,
        role: role,
        text: text,
        meta: meta,
        tools: tools,
        ts: ts,
        terminal: true,
        terminal_source: terminalSource || 'system',
        cwd: cwd
      };
      if (agentId) msg.agent_id = agentId;
      if (agentName) msg.agent_name = agentName;
      this.messages.push(msg);
      if (cwd) this.terminalCwd = cwd;
      return msg;
    },
