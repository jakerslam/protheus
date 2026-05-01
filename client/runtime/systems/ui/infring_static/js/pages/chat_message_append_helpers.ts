// Chat user and terminal message append helpers.
'use strict';

function infringChatMessageAppendMethods() {
  return {
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

      var rows = this.ensureActiveChatMessagesArray();
      var last = rows.length ? rows[rows.length - 1] : null;
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
        if (typeof this.syncActiveChatMessages === 'function') this.syncActiveChatMessages();
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
      this.appendActiveChatMessage(msg);
      if (cwd) this.terminalCwd = cwd;
      return msg;
    },
  };
}
