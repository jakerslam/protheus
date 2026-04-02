        case '/context':
          // Visual-only update for context ring; no chat message noise.
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'context', args: '', silent: true });
          } else {
            self.recomputeContextEstimate();
            self.setContextWindowFromCurrentAgent();
          }
          break;
        case '/verbose':
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'verbose', args: cmdArgs });
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Not connected. Connect to an agent first.', meta: '', tools: [], system_origin: 'slash:verbose' });
            self.scrollToBottom();
          }
          break;
        case '/queue':
          if (self.currentAgent && InfringAPI.isWsConnected()) {
            InfringAPI.wsSend({ type: 'command', command: 'queue', args: '' });
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Not connected.', meta: '', tools: [], system_origin: 'slash:queue' });
            self.scrollToBottom();
          }
          break;
        case '/status':
          InfringAPI.get('/api/status').then(function(s) {
            self.messages.push({ id: ++msgId, role: 'system', text: '**System Status**\n- Agents: ' + (s.agent_count || 0) + '\n- Uptime: ' + (s.uptime_seconds || 0) + 's\n- Version: ' + (s.version || '?'), meta: '', tools: [], system_origin: 'slash:status' });
            self.scrollToBottom();
          }).catch(function() {});
          break;
        case '/alerts':
          await self.runSlashAlerts();
          break;
        case '/next':
          await self.runSlashNextActions();
          break;
        case '/memory':
          await self.runSlashMemoryHygiene();
          break;
        case '/continuity':
          await self.runSlashContinuity();
          break;
        case '/aliases':
          self.runSlashAliases();
          break;
        case '/alias':
          self.runSlashAliasCommand(cmdArgs);
          break;
        case '/opt':
          await self.runSlashOptimizeWorkers();
          break;
        case '/model':
          if (self.currentAgent) {
            if (cmdArgs) {
              self.switchAgentModelWithGuards({ id: cmdArgs }, {
                agent_id: self.currentAgent.id
              }).catch(function(e) {
                InfringToast.error('Model switch failed: ' + e.message);
              });
            } else {
              self.messages.push({ id: ++msgId, role: 'system', text: '**Current Model**\n- Provider: `' + (self.currentAgent.model_provider || '?') + '`\n- Model: `' + (self.currentAgent.model_name || '?') + '`', meta: '', tools: [], system_origin: 'slash:model' });
              self.scrollToBottom();
            }
          } else {
            self.messages.push({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:model' });
            self.scrollToBottom();
          }
          break;
        case '/file':
          if (!self.currentAgent) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:file' });
            self.scrollToBottom();
            break;
          }
          if (!cmdArgs || !String(cmdArgs).trim()) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Usage: `/file <path>`', meta: '', tools: [], system_origin: 'slash:file' });
            self.scrollToBottom();
            break;
          }
          try {
            var fileRes = await InfringAPI.post('/api/agents/' + self.currentAgent.id + '/file/read', {
              path: String(cmdArgs || '').trim()
            });
            var fileMeta = fileRes && fileRes.file ? fileRes.file : null;
            if (!fileMeta || !fileMeta.ok) {
              self.messages.push({
                id: ++msgId,
                role: 'system',
                text: 'Error: failed to read file output.',
                meta: '',
                tools: [],
                system_origin: 'slash:file',
                ts: Date.now()
              });
            } else {
              var bytes = Number(fileMeta.bytes || 0);
              var fileMetaText = (bytes > 0 ? (bytes + ' bytes') : '');
              if (fileMeta.truncated) {
                var maxBytes = Number(fileMeta.max_bytes || 0);
                fileMetaText += (fileMetaText ? ' | ' : '') + 'truncated to ' + (maxBytes > 0 ? maxBytes : 'limit') + ' bytes';
              }
              self.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: fileMetaText,
                tools: [],
                ts: Date.now(),
                file_output: {
                  path: String(fileMeta.path || cmdArgs || ''),
                  content: String(fileMeta.content || ''),
                  truncated: !!fileMeta.truncated,
                  bytes: bytes
                }
              });
            }
            self.scrollToBottom();
          } catch (e) {
            self.messages.push({
              id: ++msgId,
              role: 'system',
              text: 'Error: ' + (e && e.message ? e.message : 'file read failed'),
              meta: '',
              tools: [],
              system_origin: 'slash:file',
              ts: Date.now()
            });
            self.scrollToBottom();
          }
          break;
        case '/folder':
          if (!self.currentAgent) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'No agent selected.', meta: '', tools: [], system_origin: 'slash:folder' });
            self.scrollToBottom();
            break;
          }
          if (!cmdArgs || !String(cmdArgs).trim()) {
            self.messages.push({ id: ++msgId, role: 'system', text: 'Usage: `/folder <path>`', meta: '', tools: [], system_origin: 'slash:folder' });
            self.scrollToBottom();
            break;
          }
          try {
            var folderRes = await InfringAPI.post('/api/agents/' + self.currentAgent.id + '/folder/export', {
              path: String(cmdArgs || '').trim()
            });
            var folderMeta = folderRes && folderRes.folder ? folderRes.folder : null;
            var archiveMeta = folderRes && folderRes.archive ? folderRes.archive : null;
            if (!folderMeta || !folderMeta.ok) {
              self.messages.push({
                id: ++msgId,
                role: 'system',
                text: 'Error: failed to export folder output.',
                meta: '',
                tools: [],
                system_origin: 'slash:folder',
                ts: Date.now()
              });
            } else {
              var entryCount = Number(folderMeta.entries || 0);
              var folderMetaText = (entryCount > 0 ? (entryCount + ' entries') : '');
              if (folderMeta.truncated) folderMetaText += (folderMetaText ? ' | ' : '') + 'tree truncated';
              if (archiveMeta && archiveMeta.file_name) {
                folderMetaText += (folderMetaText ? ' | ' : '') + archiveMeta.file_name;
              }
              self.messages.push({
                id: ++msgId,
                role: 'agent',
                text: '',
                meta: folderMetaText,
                tools: [],
                ts: Date.now(),
                folder_output: {
                  path: String(folderMeta.path || cmdArgs || ''),
                  tree: String(folderMeta.tree || ''),
                  entries: entryCount,
                  truncated: !!folderMeta.truncated,
                  download_url: archiveMeta && archiveMeta.download_url ? String(archiveMeta.download_url) : '',
                  archive_name: archiveMeta && archiveMeta.file_name ? String(archiveMeta.file_name) : '',
                  archive_bytes: Number(archiveMeta && archiveMeta.bytes ? archiveMeta.bytes : 0)
                }
              });
            }
            self.scrollToBottom();
          } catch (e2) {
            self.messages.push({
              id: ++msgId,
              role: 'system',
              text: 'Error: ' + (e2 && e2.message ? e2.message : 'folder export failed'),
              meta: '',
              tools: [],
              system_origin: 'slash:folder',
              ts: Date.now()
            });
            self.scrollToBottom();
          }
          break;
        case '/clear':
          self.messages = [];
          break;
        case '/exit':
          InfringAPI.wsDisconnect();
          self._wsAgent = null;
          self.currentAgent = null;
          self.setStoreActiveAgentId(null);
          self.messages = [];
          window.dispatchEvent(new Event('close-chat'));
          break;
        case '/budget':
          InfringAPI.get('/api/budget').then(function(b) {
            var fmt = function(v) { return v > 0 ? '$' + v.toFixed(2) : 'unlimited'; };
            self.messages.push({ id: ++msgId, role: 'system', text: '**Budget Status**\n' +
              '- Hourly: $' + (b.hourly_spend||0).toFixed(4) + ' / ' + fmt(b.hourly_limit) + '\n' +
              '- Daily: $' + (b.daily_spend||0).toFixed(4) + ' / ' + fmt(b.daily_limit) + '\n' +
              '- Monthly: $' + (b.monthly_spend||0).toFixed(4) + ' / ' + fmt(b.monthly_limit), meta: '', tools: [], system_origin: 'slash:budget' });
            self.scrollToBottom();
          }).catch(function() {});
          break;
        case '/peers':
          InfringAPI.get('/api/network/status').then(function(ns) {
            self.messages.push({ id: ++msgId, role: 'system', text: '**OFP Network**\n' +
              '- Status: ' + (ns.enabled ? 'Enabled' : 'Disabled') + '\n' +
              '- Connected peers: ' + (ns.connected_peers||0) + ' / ' + (ns.total_peers||0), meta: '', tools: [], system_origin: 'slash:peers' });
            self.scrollToBottom();
          }).catch(function() {});
          break;
        case '/a2a':
          InfringAPI.get('/api/a2a/agents').then(function(res) {
            var agents = res.agents || [];
            if (!agents.length) {
              self.messages.push({ id: ++msgId, role: 'system', text: 'No external A2A agents discovered.', meta: '', tools: [], system_origin: 'slash:a2a' });
            } else {
              var lines = agents.map(function(a) { return '- **' + a.name + '** — ' + a.url; });
              self.messages.push({ id: ++msgId, role: 'system', text: '**A2A Agents (' + agents.length + ')**\n' + lines.join('\n'), meta: '', tools: [], system_origin: 'slash:a2a' });
            }
            self.scrollToBottom();
          }).catch(function() {});
          break;
      }
      this.scheduleConversationPersist();
    },

    maybeDiscardPendingFreshAgent: function(nextAgentId) {
      var store = Alpine.store('app');
      if (!store) return;
      var pendingId = String(store.pendingFreshAgentId || '').trim();
      if (!pendingId) return;
      var currentId = String(this.currentAgent && this.currentAgent.id ? this.currentAgent.id : '').trim();
      var targetId = String(nextAgentId || '').trim();
      if (!currentId || currentId !== pendingId || !targetId || targetId === pendingId) return;
      store.pendingFreshAgentId = null;
      store.pendingAgent = null;
      InfringAPI.del('/api/agents/' + encodeURIComponent(pendingId)).catch(function() {});
      if (typeof store.refreshAgents === 'function') {
        setTimeout(function() { store.refreshAgents({ force: true }).catch(function() {}); }, 0);
      }
    },

    selectAgent(agent) {
      var resolved = this.resolveAgent(agent);
      if (!resolved) return;
      var selectingSystemThread = this.isSystemThreadAgent(resolved);
      this.closeGitTreeMenu();
      var currentAgentId = this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : '';
      var nextAgentId = String((resolved && resolved.id) || '');
      this.maybeDiscardPendingFreshAgent(nextAgentId);
      if (currentAgentId !== nextAgentId) {
        var activeSearch = String(this.searchQuery || '').trim();
        if (activeSearch) {
          this.searchQuery = '';
          this.searchOpen = false;
        }
      }
      this._markAgentPreviewUnread(resolved.id, false);
      var store = Alpine.store('app');
      var pendingFreshId = store && store.pendingFreshAgentId ? String(store.pendingFreshAgentId) : '';
      var forceFreshSession = pendingFreshId && String(resolved.id) === pendingFreshId;
      this.clearHoveredMessageHard();
      this.activeMapPreviewDomId = '';
      this.activeMapPreviewDayKey = '';
      if (this.currentAgent && this.currentAgent.id && this.currentAgent.id !== resolved.id) {
        var switchingFrom = String(this.currentAgent.id || '');
        if (
          this.sending &&
          this._pendingWsRequest &&
          String(this._pendingWsRequest.agent_id || '') === switchingFrom
        ) {
          this._clearTypingTimeout();
          this.messages = this.messages.filter(function(m) { return !m.thinking && !m.streaming; });
          this.sending = false;
          this._responseStartedAt = 0;
          this.setAgentLiveActivity(switchingFrom, 'working');
          this._recoverPendingWsRequest('agent_switch');
        }
        this.cacheAgentConversation(this.currentAgent.id);
      }
      if (this.currentAgent && this.currentAgent.id === resolved.id) {
        if (selectingSystemThread) {
          this.activateSystemThread({ preserve_if_empty: true });
          return;
        }
        this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
        this.touchModelUsage(resolved.model_name || resolved.runtime_model || '');
        if (forceFreshSession) {
          this.messages = [];
          this.contextApproxTokens = 0;
          this.refreshContextPressure();
          this.resetFreshInitStateForAgent(resolved);
          if (this.conversationCache) {
            delete this.conversationCache[String(resolved.id)];
            this.persistConversationCache();
          }
          InfringAPI.post('/api/agents/' + resolved.id + '/session/reset', {}).catch(function() {});
          this.connectWs(resolved.id);
          this.loadSessions(resolved.id);
          this.requestContextTelemetry(true);
          this.clearPromptSuggestions();
          this.startFreshInitSequence(resolved);
          var selfFreshCurrent = this;
          this.$nextTick(function() {
            selfFreshCurrent.scrollToBottomImmediate();
            selfFreshCurrent.stabilizeBottomScroll();
            selfFreshCurrent.pinToLatestOnOpen(null, { maxFrames: 20 });
            selfFreshCurrent.installChatMapWheelLock();
            selfFreshCurrent.scheduleMessageRenderWindowUpdate();
          });
        } else {
          this.loadSession(resolved.id, false);
        }
        return;
      }
      if (selectingSystemThread) {
        this.activateSystemThread({ preserve_if_empty: false });
        return;
      }
      this.currentAgent = this.applyAgentGitTreeState(resolved, resolved) || resolved;
      if (store) this.setStoreActiveAgentId(resolved.id || null);
      this.touchModelUsage(resolved.model_name || resolved.runtime_model || '');
      this.setContextWindowFromCurrentAgent();
      if (forceFreshSession && this.conversationCache) {
        delete this.conversationCache[String(resolved.id)];
        this.persistConversationCache();
        InfringAPI.post('/api/agents/' + resolved.id + '/session/reset', {}).catch(function() {});
      }
      var restored = forceFreshSession ? false : this.restoreAgentConversation(resolved.id);
      if (!restored) {
        this.messages = [];
        this.contextApproxTokens = 0;
        this.refreshContextPressure();
      }
      this.showFreshArchetypeTiles = false;
      this.freshInitRevealMenu = false;
      if (forceFreshSession) {
        this.resetFreshInitStateForAgent(resolved);
        this.clearPromptSuggestions();
        this.startFreshInitSequence(resolved);
      } else {
        this.freshInitStageToken = Number(this.freshInitStageToken || 0) + 1;
        this._freshInitThreadShownFor = '';
      }
      this._reconcileSendingState();
      this.connectWs(resolved.id);
      // Show welcome tips on first use
      if (!restored && !this.showFreshArchetypeTiles && !localStorage.getItem('of-chat-tips-seen')) {
        this.messages.push({
          id: ++msgId,
          role: 'system',
          text: '**Welcome to Infring Chat!**\n\n' +
            '- Type `/` to see available commands\n' +
            '- `/help` shows all commands\n' +
            '- `/think on` enables extended reasoning\n' +
            '- `/context` shows context window usage\n' +
            '- `/verbose off` hides tool details\n' +
            '- `Ctrl+Shift+F` toggles focus mode\n' +
            '- `Ctrl+F` opens file picker\n' +
            '- Drag & drop files to attach them\n' +
            '- `Ctrl+/` opens the command palette',
          meta: '',
          tools: [],
          system_origin: 'chat:welcome'
        });
        localStorage.setItem('of-chat-tips-seen', 'true');
      }
      if (!forceFreshSession) {
        this.loadSession(resolved.id, false);
      }
      this.loadSessions(resolved.id);
      this.requestContextTelemetry(true);
      if (!forceFreshSession) {
        this.refreshPromptSuggestions(false);
      }
      if (this.showAgentDrawer) {
        this.openAgentDrawer();
      }
      // Focus input after agent selection
      var self = this;
      this.$nextTick(function() {
        var el = document.getElementById('msg-input');
        if (el) el.focus();
        self.scrollToBottomImmediate();
        self.stabilizeBottomScroll();
        self.pinToLatestOnOpen(null, { maxFrames: 20 });
        self.installChatMapWheelLock();
        self.scheduleMessageRenderWindowUpdate();
      });
    },

    shouldRenderMessage(msg, idx) {
      if (!msg || msg.is_notice) return true;
      if (!this.currentAgent) return true;
      var id = this.messageDomId(msg, idx);
      if (this.messageHydration && this.messageHydration[id]) return true;
      // Always hydrate newest messages for streaming responsiveness.
      if (idx >= (this.messages.length - 24)) return true;
      return false;
    },

    forceMessageRender(msg, idx, ttlMs) {
      if (!msg) return;
      var id = this.messageDomId(msg, idx);
      if (!id) return;
      var ttl = Number(ttlMs || 0);
      var until = Date.now() + (ttl > 0 ? ttl : 6000);
      if (!this._forcedHydrateById || typeof this._forcedHydrateById !== 'object') {
        this._forcedHydrateById = {};
      }
      this._forcedHydrateById[id] = until;
      this.scheduleMessageRenderWindowUpdate();
    },

    scheduleMessageRenderWindowUpdate(container) {
      var self = this;
      if (this._renderWindowRaf && typeof cancelAnimationFrame === 'function') {
        cancelAnimationFrame(this._renderWindowRaf);
        this._renderWindowRaf = 0;
      }
      var run = function() {
        self._renderWindowRaf = 0;
        self.updateMessageRenderWindow(container);
      };
      if (typeof requestAnimationFrame === 'function') {
        this._renderWindowRaf = requestAnimationFrame(run);
      } else {
        setTimeout(run, 0);
      }
    },

    updateMessageRenderWindow(container) {
      var el = this.resolveMessagesScroller(container || null);
      if (!el || !this.currentAgent) return;
      var viewportHeight = Number(el.clientHeight || 0);
      if (!Number.isFinite(viewportHeight) || viewportHeight <= 0) return;
      var minY = Math.max(0, el.scrollTop - viewportHeight);
      var maxY = el.scrollTop + (viewportHeight * 2);
      var next = {};
      var blocks = el.querySelectorAll('.chat-message-block[data-msg-idx]');
      for (var i = 0; i < blocks.length; i++) {
        var block = blocks[i];
        if (!block || !block.id) continue;
        var top = Number(block.offsetTop || 0);
        var height = Number(block.offsetHeight || 0);
        if (!Number.isFinite(height) || height <= 0) height = 48;
        var bottom = top + height;
        if (bottom >= minY && top <= maxY) next[block.id] = true;
