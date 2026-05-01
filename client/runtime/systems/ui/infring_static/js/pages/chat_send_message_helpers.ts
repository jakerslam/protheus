// Chat composer send orchestration helpers.
'use strict';

function infringChatSendMessageMethods() {
  return {
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
        if (matched && this.isShellOwnedSlashCommand(matched.cmd)) {
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
  };
}
