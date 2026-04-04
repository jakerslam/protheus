          if (!Array.isArray(this.agentDrawer._fallbacks)) this.agentDrawer._fallbacks = [];
          this.agentDrawer._fallbacks.push({ provider: fallbackProvider, model: fallbackModel });
          appendedFallback = true;
          configPayload.fallback_models = this.agentDrawer._fallbacks;
        } else if (Array.isArray(this.agentDrawer._fallbacks)) {
          configPayload.fallback_models = this.agentDrawer._fallbacks;
        }

        var configResponse = await InfringAPI.patch('/api/agents/' + agentId + '/config', configPayload);
        if (configResponse && configResponse.rename_notice) {
          this.addNoticeEvent(configResponse.rename_notice);
        } else if (requestedName && requestedName !== previousName) {
          this.addAgentRenameNotice(previousName, requestedName);
        }

        if (this.drawerEditingProvider && String(this.drawerNewProviderValue || '').trim()) {
          var previousProviderName = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
          var previousModelName = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
          var combined = String(this.drawerNewProviderValue || '').trim() + '/' + (this.agentDrawer.model_name || '');
          await this.switchAgentModelWithGuards({ id: combined }, {
            agent_id: agentId,
            previous_model: previousModelName,
            previous_provider: previousProviderName
          });
        } else if (this.drawerEditingModel && String(this.drawerNewModelValue || '').trim()) {
          var previousModelNameForModelEdit = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
          var previousProviderForModelEdit = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
          await this.switchAgentModelWithGuards(
            { id: String(this.drawerNewModelValue || '').trim() },
            {
              agent_id: agentId,
              previous_model: previousModelNameForModelEdit,
              previous_provider: previousProviderForModelEdit
            }
          );
        }

        this.drawerEditingName = false;
        this.drawerEditingEmoji = false;
        this.drawerEditingModel = false;
        this.drawerEditingProvider = false;
        this.drawerEditingFallback = false;
        this.drawerNewModelValue = '';
        this.drawerNewProviderValue = '';
        this.drawerNewFallbackValue = '';
        InfringToast.success('Agent settings saved');
        await this.syncDrawerAgentAfterChange();
      } catch (e) {
        if (appendedFallback) {
          this.agentDrawer._fallbacks = previousFallbacks;
        }
        InfringToast.error('Failed to save agent settings: ' + e.message);
      } finally {
        this.drawerSavePending = false;
        this.drawerConfigSaving = false;
        this.drawerModelSaving = false;
        this.drawerIdentitySaving = false;
      }
    },

    async saveDrawerConfig() {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      var requestedName = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      this.drawerConfigSaving = true;
      try {
        var response = await InfringAPI.patch('/api/agents/' + this.agentDrawer.id + '/config', this.drawerConfigForm || {});
        if (response && response.rename_notice) {
          this.addNoticeEvent(response.rename_notice);
        } else if (requestedName && requestedName !== previousName) {
          this.addAgentRenameNotice(previousName, requestedName);
        }
        InfringToast.success('Config updated');
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to save config: ' + e.message);
      }
      this.drawerConfigSaving = false;
    },

    async saveDrawerIdentity(part) {
      if (!this.agentDrawer || !this.agentDrawer.id) return;
      var payload = {};
      var previousName = String((this.agentDrawer && this.agentDrawer.name) || (this.currentAgent && this.currentAgent.name) || '').trim();
      if (part === 'name') {
        payload.name = String((this.drawerConfigForm && this.drawerConfigForm.name) || '').trim();
      } else if (part === 'emoji') {
        payload.emoji = String((this.drawerConfigForm && this.drawerConfigForm.emoji) || '').trim();
        if (this.sanitizeAgentEmojiForDisplay) {
          payload.emoji = this.sanitizeAgentEmojiForDisplay(this.agentDrawer || this.currentAgent, payload.emoji);
        }
        if (!payload.emoji) {
          InfringToast.info('The gear icon is reserved for the System thread.');
          this.drawerIdentitySaving = false;
          return;
        }
        payload.avatar_url = '';
        if (this.drawerConfigForm && typeof this.drawerConfigForm === 'object') {
          this.drawerConfigForm.avatar_url = '';
        }
        if (this.agentDrawer && typeof this.agentDrawer === 'object') {
          this.agentDrawer.avatar_url = '';
        }
      } else if (part === 'avatar') {
        payload.avatar_url = String((this.drawerConfigForm && this.drawerConfigForm.avatar_url) || '').trim();
      } else {
        return;
      }
      this.drawerIdentitySaving = true;
      try {
        var response = await InfringAPI.patch('/api/agents/' + this.agentDrawer.id + '/config', payload);
        if (response && response.rename_notice) {
          this.addNoticeEvent(response.rename_notice);
        } else if (part === 'name' && payload.name && payload.name !== previousName) {
          this.addAgentRenameNotice(previousName, payload.name);
        }
        if (part === 'name') this.drawerEditingName = false;
        if (part === 'emoji') this.drawerEditingEmoji = false;
        if (part === 'avatar') {
          this.drawerAvatarUploadError = '';
          this.drawerAvatarUrlPickerOpen = false;
          this.drawerAvatarUrlDraft = '';
        }
        InfringToast.success(
          part === 'name'
            ? 'Name updated'
            : (part === 'emoji' ? 'Emoji updated' : 'Avatar updated')
        );
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to save ' + part + ': ' + e.message);
      }
      this.drawerIdentitySaving = false;
    },

    async changeDrawerModel() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewModelValue || '').trim()) return;
      this.drawerModelSaving = true;
      try {
        var previousModel = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
        var previousProvider = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
        var resp = await this.switchAgentModelWithGuards(
          { id: String(this.drawerNewModelValue || '').trim() },
          {
            agent_id: this.agentDrawer.id,
            previous_model: previousModel,
            previous_provider: previousProvider
          }
        );
        var providerInfo = (resp && resp.provider) ? ' (provider: ' + resp.provider + ')' : '';
        InfringToast.success('Model changed' + providerInfo + ' (memory reset)');
        this.drawerEditingModel = false;
        this.drawerNewModelValue = '';
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to change model: ' + e.message);
      }
      this.drawerModelSaving = false;
    },

    async changeDrawerProvider() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewProviderValue || '').trim()) return;
      this.drawerModelSaving = true;
      try {
        var previousProvider = String((this.agentDrawer && this.agentDrawer.model_provider) || (this.currentAgent && this.currentAgent.model_provider) || '').trim();
        var previousModel = String((this.agentDrawer && (this.agentDrawer.runtime_model || this.agentDrawer.model_name)) || (this.currentAgent && (this.currentAgent.runtime_model || this.currentAgent.model_name)) || '').trim();
        var combined = String(this.drawerNewProviderValue || '').trim() + '/' + (this.agentDrawer.model_name || '');
        var resp = await this.switchAgentModelWithGuards(
          { id: combined },
          {
            agent_id: this.agentDrawer.id,
            previous_model: previousModel,
            previous_provider: previousProvider
          }
        );
        InfringToast.success('Provider changed to ' + (resp && resp.provider ? resp.provider : String(this.drawerNewProviderValue || '').trim()));
        this.drawerEditingProvider = false;
        this.drawerNewProviderValue = '';
        await this.syncDrawerAgentAfterChange();
      } catch(e) {
        InfringToast.error('Failed to change provider: ' + e.message);
      }
      this.drawerModelSaving = false;
    },

    async addDrawerFallback() {
      if (!this.agentDrawer || !this.agentDrawer.id || !String(this.drawerNewFallbackValue || '').trim()) return;
      var parts = String(this.drawerNewFallbackValue || '').trim().split('/');
      var provider = parts.length > 1 ? parts[0] : this.agentDrawer.model_provider;
      var model = parts.length > 1 ? parts.slice(1).join('/') : parts[0];
      if (!this.agentDrawer._fallbacks) this.agentDrawer._fallbacks = [];
      this.agentDrawer._fallbacks.push({ provider: provider, model: model });
      try {
        await InfringAPI.patch('/api/agents/' + this.agentDrawer.id + '/config', {
          fallback_models: this.agentDrawer._fallbacks
        });
        InfringToast.success('Fallback added: ' + provider + '/' + model);
        this.drawerEditingFallback = false;
        this.drawerNewFallbackValue = '';
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        this.agentDrawer._fallbacks.pop();
      }
    },

    async removeDrawerFallback(idx) {
      if (!this.agentDrawer || !this.agentDrawer.id || !Array.isArray(this.agentDrawer._fallbacks)) return;
      var removed = this.agentDrawer._fallbacks.splice(idx, 1);
      try {
        await InfringAPI.patch('/api/agents/' + this.agentDrawer.id + '/config', {
          fallback_models: this.agentDrawer._fallbacks
        });
        InfringToast.success('Fallback removed');
      } catch(e) {
        InfringToast.error('Failed to save fallbacks: ' + e.message);
        if (removed && removed.length) this.agentDrawer._fallbacks.splice(idx, 0, removed[0]);
      }
    },

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

    toolDisplayName: function(tool) {
      if (!tool) return 'tool';
      if (this.isThoughtTool(tool)) return 'thought';
      return String(tool.name || 'tool');
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
      if (this.prefersReducedMotion()) { snapshot.ghost.style.opacity = '0.56'; setTimeout(this.clearComposerSendMorph.bind(this, snapshot), 240); return; }
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
