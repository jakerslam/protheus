
    async sendTerminalMessage() {
      if (this.showFreshArchetypeTiles) {
        InfringToast.info('Launch agent initialization before running terminal commands.');
        return;
      }
      var activeAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!activeAgent || !this.inputText.trim()) return;
      if (!this.isSystemThreadAgent(activeAgent) && this.isArchivedAgentRecord && this.isArchivedAgentRecord(activeAgent)) {
        InfringToast.info('This agent is archived. Revive it to run commands.');
        return;
      }
      this.showFreshArchetypeTiles = false;
      var command = this.inputText.trim();
      this.pushInputHistoryEntry('terminal', command);
      this.inputText = '';
      this.terminalSelectionStart = 0;

      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';

      if (this.sending) {
        this._reconcileSendingState();
      }
      if (this.sending) {
        this.messageQueue.push({
          queue_id: this.nextPromptQueueId(),
          queue_kind: 'terminal',
          queued_at: Date.now(),
          terminal: true,
          command: command
        });
        return;
      }

      this._sendTerminalPayload(command, activeAgent.id);
    },

    async sendMessage() {
      if (this.terminalMode) {
        await this.sendTerminalMessage();
        return;
      }
      if (this.showFreshArchetypeTiles && !this.freshInitLaunching) {
        if (this.freshInitAwaitingOtherPrompt) {
          this.captureFreshInitOtherPrompt();
          return;
        }
        InfringToast.info('Launch agent initialization before chatting.');
        return;
      }
      var activeAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!activeAgent || (!this.inputText.trim() && !this.attachments.length)) return;
      if (this.isArchivedAgentRecord && this.isArchivedAgentRecord(activeAgent)) {
        InfringToast.info('This agent is archived. Revive it to continue this chat.');
        return;
      }
      if (this.isSystemThreadAgent(activeAgent)) {
        if (Array.isArray(this.attachments) && this.attachments.length) {
          InfringToast.info('System thread does not accept file attachments.');
          this.attachments = [];
        }
        await this.sendTerminalMessage();
        return;
      }
      this.showFreshArchetypeTiles = false;
      var rawInput = String(this.inputText == null ? '' : this.inputText);
      var text = rawInput.trim();
      var condensedLargePaste = false;
      if (text && this.shouldConvertLargePasteToAttachment && this.shouldConvertLargePasteToAttachment(rawInput)) {
        var largePasteAttachment = this.buildLargePasteMarkdownAttachment && this.buildLargePasteMarkdownAttachment(rawInput);
        if (largePasteAttachment && largePasteAttachment.file) {
          if (!Array.isArray(this.attachments)) this.attachments = [];
          this.attachments.push(largePasteAttachment);
          text = '';
          condensedLargePaste = true;
        }
      }
      if (text || condensedLargePaste) this.pushInputHistoryEntry('chat', text || '[File: Pasted markdown.md]');
      if (condensedLargePaste) InfringToast.info('Large paste moved to Pasted markdown.md');
      if (text.startsWith('/') && !this.attachments.length) {
        var cmd = text.split(' ')[0].toLowerCase();
        var cmdArgs = text.substring(cmd.length).trim();
        var aliasResolution = this.resolveSlashAlias(cmd, cmdArgs);
        var routedCmd = String(aliasResolution && aliasResolution.cmd ? aliasResolution.cmd : cmd).toLowerCase();
        var routedArgs = String(aliasResolution && typeof aliasResolution.args === 'string' ? aliasResolution.args : cmdArgs).trim();
        var matched = this.slashCommands.find(function(c) { return c.cmd === routedCmd; });
        if (matched) {
          this.executeSlashCommand(matched.cmd, routedArgs);
          return;
        }
      }
      var availableModels = typeof this.ensureUsableModelsForChatSend === 'function'
        ? await this.ensureUsableModelsForChatSend('chat_send')
        : (typeof this.currentAvailableModelCount === 'function' ? this.currentAvailableModelCount() : 0);
      if (availableModels <= 0) {
        if (typeof this.injectNoModelsGuidance === 'function') this.injectNoModelsGuidance('chat_send');
        if (typeof this.addNoModelsRecoveryNotice === 'function') this.addNoModelsRecoveryNotice('chat_send', 'model_discover');
        return;
      }
      this.inputText = '';
      var ta = document.getElementById('msg-input');
      if (ta) ta.style.height = '';
      var fileRefs = [];
      var uploadedFiles = [];
      if (this.attachments.length) {
        for (var i = 0; i < this.attachments.length; i++) {
          var att = this.attachments[i];
          att.uploading = true;
          try {
            var uploadRes = await InfringAPI.upload(activeAgent.id, att.file);
            fileRefs.push('[File: ' + att.file.name + ']');
            uploadedFiles.push({ file_id: uploadRes.file_id, filename: uploadRes.filename, content_type: uploadRes.content_type });
          } catch(e) {
            var reason = (e && e.message) ? String(e.message) : 'upload_failed';
            InfringToast.error('Failed to upload ' + att.file.name + ': ' + reason);
            fileRefs.push('[File: ' + att.file.name + ' (upload failed)]');
          }
          att.uploading = false;
        }
        for (var j = 0; j < this.attachments.length; j++) {
          if (this.attachments[j].preview) URL.revokeObjectURL(this.attachments[j].preview);
        }
        this.attachments = [];
      }
      var finalText = text;
      if (fileRefs.length) {
        finalText = (text ? text + '\n' : '') + fileRefs.join('\n');
      }
      var msgImages = uploadedFiles.filter(function(f) { return f.content_type && f.content_type.startsWith('image/'); });
      if (this.sending) {
        this._reconcileSendingState();
      }
      if (this.sending) {
        this.messageQueue.push({
          queue_id: this.nextPromptQueueId(),
          queue_kind: 'prompt',
          queued_at: Date.now(),
          text: finalText,
          files: uploadedFiles,
          images: msgImages
        });
        this.scheduleConversationPersist();
        return;
      }
      var shouldMorphSend = !!(text && !uploadedFiles.length && !msgImages.length && !fileRefs.length && !this.sending);
      var morphSnapshot = shouldMorphSend && this.captureComposerSendMorph
        ? this.captureComposerSendMorph(text)
        : null;
      var appended = this.appendUserChatMessage(finalText, msgImages, { deferPersist: true });
      if (morphSnapshot && appended && appended.id != null && this.playComposerSendMorphToMessage) {
        var self = this;
        this.$nextTick(function() {
          self.playComposerSendMorphToMessage(morphSnapshot, appended.id);
        });
      } else if (morphSnapshot && this.clearComposerSendMorph) {
        this.clearComposerSendMorph(morphSnapshot);
      }
      this.scheduleConversationPersist();
      this._sendPayload(finalText, uploadedFiles, msgImages, { agent_id: activeAgent.id });
    },

    async _sendTerminalPayload(command, agentIdOverride) {
      var targetAgentId = String(agentIdOverride || (this.currentAgent && this.currentAgent.id) || '').trim();
      if (!targetAgentId) return;
      if (this.isSystemThreadId(targetAgentId)) {
        await this._sendSystemTerminalPayload(command);
        return;
      }
      this.sending = true;
      this.setAgentLiveActivity(targetAgentId, 'working');
      this._responseStartedAt = Date.now();
      this._appendTerminalMessage({
        role: 'terminal',
        text: this._terminalPromptLine(this.terminalPromptPath, command),
        meta: this.terminalPromptPath,
        tools: [],
        ts: Date.now(),
        terminal_source: 'user',
        cwd: this.terminalPromptPath
      });
      this.recomputeContextEstimate();
      this.scrollToBottom();
      this.scheduleConversationPersist();

      if ((!InfringAPI.isWsConnected() || String(this._wsAgent || '') !== targetAgentId) && targetAgentId) {
        this.connectWs(targetAgentId);
        var wsWaitStarted = Date.now();
        while ((!InfringAPI.isWsConnected() || String(this._wsAgent || '') !== targetAgentId) && (Date.now() - wsWaitStarted) < 1500) {
          await new Promise(function(resolve) { setTimeout(resolve, 75); });
        }
      }

      if (InfringAPI.wsSend({ type: 'terminal', command: command, cwd: this.terminalPromptPath })) {
        return;
      }

      try {
        var res = await InfringAPI.post('/api/agents/' + targetAgentId + '/terminal', {
          command: command,
          cwd: this.terminalPromptPath,
        });
        this.handleWsMessage({
          type: 'terminal_output',
          stdout: res && res.stdout ? String(res.stdout) : '',
          stderr: res && res.stderr ? String(res.stderr) : '',
          exit_code: Number(res && res.exit_code != null ? res.exit_code : 1),
          duration_ms: Number(res && res.duration_ms ? res.duration_ms : 0),
          cwd: res && res.cwd ? String(res.cwd) : this.terminalPromptPath,
          requested_command: res && res.requested_command ? String(res.requested_command) : String(command || ''),
          executed_command: res && res.executed_command ? String(res.executed_command) : String(command || ''),
          command_translated: !!(res && res.command_translated),
          translation_reason: res && res.translation_reason ? String(res.translation_reason) : '',
          suggestions: res && Array.isArray(res.suggestions) ? res.suggestions : [],
          permission_gate: res && res.permission_gate ? res.permission_gate : null,
          filter_events: res && Array.isArray(res.filter_events) ? res.filter_events : [],
          low_signal_output: !!(res && res.low_signal_output),
          recovery_hints: res && Array.isArray(res.recovery_hints) ? res.recovery_hints : [],
          tool_summary: res && res.tool_summary ? res.tool_summary : null,
          tracking: res && res.tracking ? res.tracking : null,
        });
      } catch (e) {
        this.handleWsMessage({
          type: 'terminal_error',
          message: e && e.message ? e.message : 'command failed',
        });
      }
    },

    async ensureSystemTerminalSession() {
      var existing = String(this.systemTerminalSessionId || '').trim();
      if (existing) return existing;
      var preferredId = String(this.systemThreadId || 'system').trim() || 'system';
      try {
        var created = await InfringAPI.post('/api/terminal/sessions', {
          id: preferredId,
          cwd: this.terminalPromptPath
        });
        var sid = String(created && created.session && created.session.id ? created.session.id : preferredId).trim() || preferredId;
        this.systemTerminalSessionId = sid;
        return sid;
      } catch (_) {
        this.systemTerminalSessionId = preferredId;
        return preferredId;
      }
    },

    async _sendSystemTerminalPayload(command) {
      var cmd = String(command || '').trim();
      if (!cmd) return;
      this.sending = true;
      this.setAgentLiveActivity(this.systemThreadId || 'system', 'working');
      this._responseStartedAt = Date.now();
      this._appendTerminalMessage({
        role: 'terminal',
        text: this._terminalPromptLine(this.terminalPromptPath, cmd),
        meta: this.terminalPromptPath,
        tools: [],
        ts: Date.now(),
        terminal_source: 'user',
        cwd: this.terminalPromptPath
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
      try {
        var response = null;
        for (var attempt = 0; attempt < 2; attempt += 1) {
          var sessionId = await this.ensureSystemTerminalSession();
          response = await InfringAPI.post('/api/terminal/queue', {
            session_id: sessionId,
            command: cmd,
            cwd: this.terminalPromptPath
          });
          if (response && String(response.error || '').trim() === 'session_not_found') {
            this.systemTerminalSessionId = '';
            continue;
          }
          break;
        }
        if (!response || response.ok === false) {
          throw new Error(String((response && response.error) || 'terminal_exec_failed'));
        }
        this.handleWsMessage({
          type: 'terminal_output',
          stdout: response && response.stdout ? String(response.stdout) : '',
          stderr: response && response.stderr ? String(response.stderr) : '',
          exit_code: Number(response && response.exit_code != null ? response.exit_code : 1),
          duration_ms: 0,
          cwd: this.terminalPromptPath,
          terminal_source: 'system',
          requested_command: response && response.requested_command ? String(response.requested_command) : '',
          executed_command: response && response.executed_command ? String(response.executed_command) : '',
          command_translated: !!(response && response.command_translated),
          translation_reason: response && response.translation_reason ? String(response.translation_reason) : '',
          suggestions: response && Array.isArray(response.suggestions) ? response.suggestions : [],
          permission_gate: response && response.permission_gate ? response.permission_gate : null,
          filter_events: response && Array.isArray(response.filter_events) ? response.filter_events : [],
          low_signal_output: !!(response && response.low_signal_output),
          recovery_hints: response && Array.isArray(response.recovery_hints) ? response.recovery_hints : [],
          tool_summary: response && response.tool_summary ? response.tool_summary : null,
          tracking: response && response.tracking ? response.tracking : null
        });
      } catch (error) {
        this.handleWsMessage({
          type: 'terminal_error',
          message: error && error.message ? error.message : 'command failed',
          terminal_source: 'system'
        });
      }
    },

    async _sendPayload(finalText, uploadedFiles, msgImages, options) {
      var opts = options && typeof options === 'object' ? options : {};
      var ensuredAgent = this.ensureValidCurrentAgent({ clear_when_missing: true });
      if (!ensuredAgent && !opts.agent_id) {
        this.sending = false;
        this._responseStartedAt = 0;
        return;
      }
      this.sending = true;
      var targetAgentId = String(
        opts.agent_id || (ensuredAgent && ensuredAgent.id) || (this.currentAgent && this.currentAgent.id) || ''
      ).trim();
      if (!targetAgentId) {
        this.sending = false;
        this._responseStartedAt = 0;
        return;
      }
      this.setAgentLiveActivity(targetAgentId, 'typing');
      var safeFiles = Array.isArray(uploadedFiles) ? uploadedFiles.slice() : [];
      var safeImages = Array.isArray(msgImages) ? msgImages.slice() : [];
      if (
        !opts.retry_from_failover ||
        !this._inflightPayload ||
        String(this._inflightPayload.agent_id || '') !== targetAgentId
      ) {
        this._inflightPayload = {
          agent_id: targetAgentId,
          final_text: String(finalText || ''),
          uploaded_files: safeFiles,
          msg_images: safeImages,
          failover_attempted: !!opts.retry_from_failover,
          created_at: Date.now()
        };
      } else {
        this._inflightPayload.final_text = String(finalText || '');
        this._inflightPayload.uploaded_files = safeFiles;
        this._inflightPayload.msg_images = safeImages;
        this._inflightPayload.retry_started_at = Date.now();
      }
      this._pendingAutoModelSwitchBaseline = this.captureAutoModelSwitchBaseline();
      var preflightRoute = await this.fetchAutoRoutePreflight(finalText, uploadedFiles);
      var preflightMeta = this.formatAutoRouteMeta(preflightRoute);
      if (preflightRoute) this.applyAutoRouteTelemetry({ auto_route: preflightRoute });
      if (!InfringAPI.isWsConnected() || String(this._wsAgent || '') !== targetAgentId) {
        this.connectWs(targetAgentId);
        var waitStarted = Date.now();
        while ((!InfringAPI.isWsConnected() || String(this._wsAgent || '') !== targetAgentId) && (Date.now() - waitStarted) < 1500) {
          await new Promise(function(resolve) { setTimeout(resolve, 75); });
        }
      }
      var wsPayload = { type: 'message', content: finalText };
      if (uploadedFiles && uploadedFiles.length) wsPayload.attachments = uploadedFiles;
      if (InfringAPI.wsSend(wsPayload)) {
        this._setPendingWsRequest(targetAgentId, finalText);
        this._responseStartedAt = Date.now();
        this.messages.push({
          id: ++msgId,
          role: 'agent',
          text: '',
          meta: preflightMeta || '',
          thinking: true,
          streaming: true,
          tools: [],
          ts: Date.now()
        });
        this.scrollToBottom();
        this.scheduleConversationPersist();
        return;
      }
      this._clearPendingWsRequest(targetAgentId);
      if (!InfringAPI.isWsConnected()) {
        InfringToast.info('Using HTTP mode (no streaming)');
      }
      this.messages.push({
        id: ++msgId,
        role: 'agent',
        text: '',
        meta: preflightMeta || '',
        thinking: true,
        tools: [],
        ts: Date.now()
      });
      this.scrollToBottom();
      this.scheduleConversationPersist();
      var httpStartedAt = Date.now();
      var handedOffToRecovery = false;

      try {
        var httpBody = { message: finalText };
        if (uploadedFiles && uploadedFiles.length) httpBody.attachments = uploadedFiles;
        var httpAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim();
        if (!httpAutoSwitchPrevious) httpAutoSwitchPrevious = this.captureAutoModelSwitchBaseline();
        var res = await InfringAPI.post('/api/agents/' + targetAgentId + '/message', httpBody);
        this.applyContextTelemetry(res);
        var httpRoute = this.applyAutoRouteTelemetry(res);
        typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking; }));
        var httpMeta = (res.input_tokens || 0) + ' in / ' + (res.output_tokens || 0) + ' out';
        if (res.cost_usd != null) httpMeta += ' | $' + res.cost_usd.toFixed(4);
        if (res.iterations) httpMeta += ' | ' + res.iterations + ' iter';
        var httpDurationMs = Math.max(0, Date.now() - httpStartedAt);
        var httpDuration = this.formatResponseDuration(httpDurationMs);
        if (httpDuration) httpMeta += ' | ' + httpDuration;
        var httpRouteMeta = this.formatAutoRouteMeta(httpRoute || preflightRoute);
        if (httpRouteMeta) httpMeta += ' | ' + httpRouteMeta;
        var httpTools = Array.isArray(res.tools)
          ? res.tools.map(function(t, idx) {
              return {
                id: (t && t.id) || ('http-tool-' + Date.now() + '-' + idx),
                name: (t && t.name) || 'tool',
                running: false,
                expanded: false,
                input: (t && t.input) || '',
                result: (t && t.result) || '',
                is_error: !!(t && t.is_error),
              };
            })
          : [];
        var httpText = this.stripModelPrefix(this.sanitizeToolText(res.response || ''));
        var httpArtifactDirectives = this.extractArtifactDirectives(httpText);
        var httpSplit = this.extractThinkingLeak(httpText);
        if (httpSplit.thought) {
          httpTools.unshift(this.makeThoughtToolCard(httpSplit.thought, httpDurationMs));
          httpText = httpSplit.content || '';
        }
        httpText = this.stripArtifactDirectivesFromText(httpText);
        var httpCompact = String(httpText || '').replace(/\s+/g, ' ').trim();
        if (
          typeof this.isThinkingPlaceholderText === 'function' &&
          this.isThinkingPlaceholderText(httpCompact)
        ) {
          httpText = '';
        }
        if (!String(httpText || '').trim()) {
          httpText = this.defaultAssistantFallback(httpSplit.thought || '', httpTools);
        }
        var httpFailure = this.extractRecoverableBackendFailure(httpText);
        if (httpFailure) {
          this._clearPendingWsRequest(targetAgentId);
          this._pendingAutoModelSwitchBaseline = '';
          this.sending = false;
          this._responseStartedAt = 0;
          this.tokenCount = 0;
          this._clearTypingTimeout();
          this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
          handedOffToRecovery = await this.attemptAutomaticFailoverRecovery('http_response', httpText, {
            remove_last_agent_failure: false
          });
          if (handedOffToRecovery) {
            this.scheduleConversationPersist();
            return;
          }
        }
        var httpMessage = {
          id: ++msgId,
          role: 'agent',
          text: httpText,
          meta: httpMeta,
          tools: httpTools,
          ts: Date.now(),
          agent_id: res && res.agent_id ? String(res.agent_id) : (this.currentAgent && this.currentAgent.id ? String(this.currentAgent.id) : ''),
          agent_name: res && res.agent_name ? String(res.agent_name) : (this.currentAgent && this.currentAgent.name ? String(this.currentAgent.name) : '')
        };
        var pushedHttpMessage = this.pushAgentMessageDeduped(httpMessage, { dedupe_window_ms: 90000 }) || httpMessage;
        this.markAgentMessageComplete(pushedHttpMessage);
        this.maybeAddAutoModelSwitchNotice(httpAutoSwitchPrevious, httpRoute || preflightRoute);
        this._pendingAutoModelSwitchBaseline = '';
        this._clearPendingWsRequest(targetAgentId);
        this._inflightPayload = null;
        if (httpArtifactDirectives && httpArtifactDirectives.length) {
          this.resolveArtifactDirectives(httpArtifactDirectives);
        }
        this.scheduleConversationPersist();
      } catch(e) {
        typeof this.clearTransientThinkingRows === 'function' ? this.clearTransientThinkingRows({ force: true }) : (this.messages = this.messages.filter(function(m) { return !m.thinking; }));
        this._clearPendingWsRequest(targetAgentId);
        this._pendingAutoModelSwitchBaseline = '';
        this.sending = false;
        this._responseStartedAt = 0;
        this.tokenCount = 0;
        this._clearTypingTimeout();
        this.setAgentLiveActivity(this.currentAgent && this.currentAgent.id, 'idle');
        var rawHttpError = String(e && e.message ? e.message : e || '');
        var lowerHttpError = rawHttpError.toLowerCase();
        var isAbortError =
          (e && String(e.name || '').toLowerCase() === 'aborterror') ||
          lowerHttpError.indexOf('this operation was aborted') >= 0 ||
          lowerHttpError.indexOf('operation was aborted') >= 0;
        if (isAbortError) {
